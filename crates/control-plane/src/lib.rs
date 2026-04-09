//! Axum application and integration tests for the DLP control plane.

pub(crate) mod deployments;
pub(crate) mod reconcile;
pub(crate) mod scheduler;
pub(crate) mod state;
pub(crate) mod workers;

use app_config as _;
use axum::{
    Json, Router,
    routing::{get, post},
};
use clap as _;
use client_sdk::HealthResponse;
use env_logger as _;
use log as _;
use serde as _;
#[cfg(test)]
use serde_json as _;
use tokio as _;
#[cfg(test)]
use tower as _;

/// Opaque handle to the control-plane's shared application state.
#[derive(Clone, Debug)]
pub struct SharedState(state::SharedState);

/// Creates a new shared in-memory application state value.
#[inline]
#[must_use]
pub fn new_shared_state() -> SharedState {
    SharedState(state::new_shared_state())
}

/// Spawns the background reconcile loop for the shared application state.
#[inline]
pub fn spawn_reconcile_loop(state: SharedState) {
    reconcile::spawn_reconcile_loop(state.0);
}

/// Builds the Axum router for the control-plane API.
#[inline]
pub fn app(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/workers", get(workers::list_workers))
        .route("/workers/register", post(workers::register_worker))
        .route(
            "/workers/{worker_id}/heartbeat",
            post(workers::heartbeat_worker),
        )
        .route("/deployments", post(deployments::create_deployment))
        .route(
            "/deployments/{deployment_id}",
            get(deployments::get_deployment),
        )
        .route("/replicas", get(deployments::list_replicas))
        .route(
            "/replicas/{replica_id}/status",
            post(deployments::update_replica_status),
        )
        .with_state(state.0)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok("control-plane"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use client_sdk::{
        CreateDeploymentRequest, DeviceClass, Framework, GetDeploymentResponse,
        ListReplicasResponse, ListWorkersResponse, RegisterWorkerRequest, ReplicaState,
        UpdateReplicaStatusRequest, WorkerCapability, WorkerHeartbeatRequest,
        WorkerHeartbeatResponse, WorkerState, WorkloadMode, WorkloadRequirement,
    };
    use serde::de::DeserializeOwned;
    use tower::util::ServiceExt as _;

    use crate::{
        app, new_shared_state, reconcile::reconcile_once, state::DEFAULT_WORKER_LOST_TIMEOUT,
    };

    fn sample_requirement(device: DeviceClass) -> WorkloadRequirement {
        WorkloadRequirement {
            framework: Framework::Pytorch,
            mode: WorkloadMode::Training,
            device,
            accelerator_runtime: "cpu".to_owned(),
            architecture_family: "generic".to_owned(),
            memory_requirement_bytes: 1024,
            concurrency_requirement: 1,
        }
    }

    fn worker_request(device: DeviceClass, slots: u32) -> RegisterWorkerRequest {
        RegisterWorkerRequest {
            worker_id:    "worker-1".to_owned(),
            display_name: "trainer-1".to_owned(),
            capabilities: vec![WorkerCapability {
                framework: Framework::Pytorch,
                mode: WorkloadMode::Training,
                device,
                accelerator_runtime: "cpu".to_owned(),
                architecture_family: "generic".to_owned(),
                available_memory_bytes: 8192,
                concurrency_slots: slots,
            }],
        }
    }

    fn first_assignment(
        response: Option<WorkerHeartbeatResponse>,
    ) -> Option<client_sdk::WorkerAssignment> {
        let heartbeat = response?;
        heartbeat.assignments.into_iter().next()
    }

    async fn json_response<Response>(
        router: axum::Router,
        request: Request<Body>,
    ) -> (StatusCode, Option<Response>)
    where
        Response: DeserializeOwned,
    {
        let response = router
            .oneshot(request)
            .await
            .expect("router request should succeed");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let parsed = serde_json::from_slice::<Response>(&body).ok();

        (status, parsed)
    }

    fn post_json<RequestBody>(uri: &str, body: &RequestBody) -> Request<Body>
    where
        RequestBody: serde::Serialize,
    {
        let json = serde_json::to_vec(body).expect("request body should serialize");
        Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json))
            .expect("request should be constructible")
    }

    fn get_request(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("request should be constructible")
    }

    #[tokio::test]
    async fn health_endpoint_returns_expected_payload() {
        let state = new_shared_state();
        let request = get_request("/health");
        let (status, payload) =
            json_response::<client_sdk::HealthResponse>(app(state), request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload,
            Some(client_sdk::HealthResponse::ok("control-plane"))
        );
    }

    #[tokio::test]
    async fn deployment_with_no_workers_stays_pending() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        let (status, created) = json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            created
                .as_ref()
                .map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );

        let deployment_id = created
            .as_ref()
            .map(|response| response.deployment.id.clone())
            .unwrap_or_default();
        let (deployment_status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request(&format!("/deployments/{deployment_id}")),
        )
        .await;

        assert_eq!(deployment_status, StatusCode::OK);
        assert_eq!(
            deployment
                .as_ref()
                .map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );
        assert_eq!(deployment.map(|response| response.replicas.len()), Some(1));
    }

    #[tokio::test]
    async fn ready_worker_receives_assignment_on_next_heartbeat() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;

        let first = json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let second = json_response::<WorkerHeartbeatResponse>(
            app(state),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;

        assert_eq!(first.0, StatusCode::OK);
        assert_eq!(first.1.map(|response| response.assignments.len()), Some(0));
        assert_eq!(second.0, StatusCode::OK);
        assert_eq!(second.1.map(|response| response.assignments.len()), Some(1));
    }

    #[tokio::test]
    async fn capability_mismatch_prevents_assignment() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cuda, 1)),
        )
        .await;
        let heartbeat = json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;

        assert_eq!(
            heartbeat.1.map(|response| response.assignments.len()),
            Some(0)
        );
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            deployment.map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );
    }

    #[tokio::test]
    async fn slot_exhaustion_keeps_extra_replica_pending() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 2,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            deployment
                .as_ref()
                .map(|response| response.deployment.status.assigned_replicas),
            Some(1)
        );
        assert_eq!(
            deployment.map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );
    }

    #[tokio::test]
    async fn replica_failure_requeues_replacement_work() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (_, heartbeat) = json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let assignment = first_assignment(heartbeat);
        let replica_id = assignment
            .as_ref()
            .map(|value| value.replica_id.clone())
            .unwrap_or_default();
        let lease_id = assignment
            .as_ref()
            .map(|value| value.lease_id.clone())
            .unwrap_or_default();

        let failure = json_response::<client_sdk::ModelReplica>(
            app(state.clone()),
            post_json(
                &format!("/replicas/{replica_id}/status"),
                &UpdateReplicaStatusRequest {
                    lease_id:       lease_id.clone(),
                    state:          ReplicaState::Failed,
                    status_message: Some("provider failed".to_owned()),
                },
            ),
        )
        .await;
        let stale_retry = json_response::<client_sdk::ModelReplica>(
            app(state.clone()),
            post_json(
                &format!("/replicas/{replica_id}/status"),
                &UpdateReplicaStatusRequest {
                    lease_id,
                    state: ReplicaState::Ready,
                    status_message: Some("late ready".to_owned()),
                },
            ),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (status, replicas) =
            json_response::<ListReplicasResponse>(app(state), get_request("/replicas")).await;

        assert_eq!(failure.0, StatusCode::OK);
        assert_eq!(stale_retry.0, StatusCode::CONFLICT);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            replicas.as_ref().map(|response| response
                .replicas
                .iter()
                .filter(|replica| replica.state == ReplicaState::Failed)
                .count()),
            Some(1)
        );
        assert_eq!(
            replicas.map(|response| response
                .replicas
                .iter()
                .filter(|replica| replica.state == ReplicaState::Assigned
                    || replica.state == ReplicaState::Pending)
                .count()),
            Some(1)
        );
    }

    #[tokio::test]
    async fn re_registering_worker_expires_old_lease_and_restores_pending_capacity() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (_, assigned_heartbeat) = json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let assignment = first_assignment(assigned_heartbeat);
        assert!(assignment.is_some());

        let (status, registration) = json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        let (deployment_status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            registration.map(|response| response.worker.state),
            Some(WorkerState::Starting)
        );
        assert_eq!(deployment_status, StatusCode::OK);
        assert_eq!(
            deployment
                .as_ref()
                .map(|response| response.deployment.status.stopped_replicas),
            Some(1)
        );
        assert_eq!(
            deployment
                .as_ref()
                .map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );
        assert_eq!(deployment.map(|response| response.replicas.len()), Some(2));
    }

    #[tokio::test]
    async fn stale_status_update_with_expired_lease_is_rejected() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (_, assigned_heartbeat) = json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let assignment = first_assignment(assigned_heartbeat);
        let replica_id = assignment
            .as_ref()
            .map(|value| value.replica_id.clone())
            .unwrap_or_default();
        let lease_id = assignment
            .as_ref()
            .map(|value| value.lease_id.clone())
            .unwrap_or_default();

        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        let (status, stale_update) = json_response::<client_sdk::ModelReplica>(
            app(state.clone()),
            post_json(
                &format!("/replicas/{replica_id}/status"),
                &UpdateReplicaStatusRequest {
                    lease_id,
                    state: ReplicaState::Ready,
                    status_message: Some("late ready".to_owned()),
                },
            ),
        )
        .await;
        let (deployment_status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(stale_update, None);
        assert_eq!(deployment_status, StatusCode::OK);
        assert_eq!(
            deployment.map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );
    }

    #[tokio::test]
    async fn wrong_lease_id_is_rejected_for_replica_update() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let (_, assigned_heartbeat) = json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        let replica_id = first_assignment(assigned_heartbeat)
            .map(|assignment| assignment.replica_id)
            .unwrap_or_default();

        let (status, stale_update) = json_response::<client_sdk::ModelReplica>(
            app(state.clone()),
            post_json(
                &format!("/replicas/{replica_id}/status"),
                &UpdateReplicaStatusRequest {
                    lease_id:       "lease-does-not-match".to_owned(),
                    state:          ReplicaState::Ready,
                    status_message: Some("late ready".to_owned()),
                },
            ),
        )
        .await;
        let (deployment_status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(stale_update, None);
        assert_eq!(deployment_status, StatusCode::OK);
        assert_eq!(
            deployment.map(|response| response.deployment.status.assigned_replicas),
            Some(1)
        );
    }

    #[tokio::test]
    async fn lost_worker_expires_leases_and_restores_pending_replica() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<client_sdk::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<client_sdk::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;
        json_response::<WorkerHeartbeatResponse>(
            app(state.clone()),
            post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            }),
        )
        .await;

        {
            let updated = state.0.lock().await.force_last_heartbeat_age(
                "worker-1",
                DEFAULT_WORKER_LOST_TIMEOUT.saturating_add(Duration::from_secs(1)),
            );
            assert!(updated);
        }
        reconcile_once(&state.0).await;
        let (worker_status, workers) =
            json_response::<ListWorkersResponse>(app(state.clone()), get_request("/workers")).await;
        let (replica_status, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;

        assert_eq!(worker_status, StatusCode::OK);
        assert_eq!(
            workers.map(|response| response.workers.first().map(|worker| worker.state.clone())),
            Some(Some(WorkerState::Lost))
        );
        assert_eq!(replica_status, StatusCode::OK);
        assert_eq!(
            deployment.map(|response| response.deployment.status.pending_replicas),
            Some(1)
        );
    }
}
