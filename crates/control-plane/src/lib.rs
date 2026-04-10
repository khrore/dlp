//! Axum application and integration tests for the DLP control plane.

pub(crate) mod application;
pub(crate) mod domain_services;
pub(crate) mod http;
pub(crate) mod mappers;
pub(crate) mod repositories;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use app_config as _;
use app_config::{ControlPlaneConfig, StorageBackend as ConfigStorageBackend};
pub use application::SharedState;
use axum::Router;
use clap as _;
use env_logger as _;
use log as _;
use repositories::{
    StorageBackend as RuntimeStorageBackend, memory::MemoryStorage, migration::Migrator,
    postgres::PostgresStorage,
};
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::MigratorTrait;

#[must_use]
pub fn new_shared_state() -> SharedState {
    SharedState::new(Arc::new(MemoryStorage::new()))
}

pub async fn new_shared_state_from_config(config: &ControlPlaneConfig) -> Result<SharedState> {
    let storage: Arc<dyn RuntimeStorageBackend> = match config.storage.backend {
        ConfigStorageBackend::Memory => Arc::new(MemoryStorage::new()),
        ConfigStorageBackend::Postgres => {
            let database_url = config.storage.database_url.as_deref().ok_or_else(|| {
                anyhow!("control_plane.storage.database_url is required for postgres backend")
            })?;
            let mut options = ConnectOptions::new(database_url.to_owned());
            options.max_connections(config.storage.pool.max_connections);
            options.min_connections(config.storage.pool.min_connections);
            let connection = Database::connect(options).await?;
            Migrator::up(&connection, None).await?;
            Arc::new(PostgresStorage::new(connection))
        }
    };
    Ok(SharedState::new(storage))
}

pub fn spawn_reconcile_loop(state: SharedState) {
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(domain_services::reconcile::DEFAULT_RECONCILE_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let _ = application::ControlPlaneService::new(state.clone())
                .reconcile_once()
                .await;
        }
    });
}

#[must_use]
pub fn app(state: SharedState) -> Router {
    http::router(state)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use dlp_api::{
        CreateDeploymentRequest, DeviceClass, Framework, GetDeploymentResponse,
        ListReplicasResponse, ListWorkersResponse, RegisterWorkerRequest, ReplicaState,
        UpdateReplicaStatusRequest, WorkerCapabilityDto, WorkerHeartbeatRequest,
        WorkerHeartbeatResponse, WorkerState, WorkloadMode, WorkloadRequirementDto,
    };
    use serde::de::DeserializeOwned;
    use tower::util::ServiceExt as _;

    use crate::{
        app, application::ControlPlaneService,
        domain_services::reconcile::DEFAULT_WORKER_LOST_TIMEOUT, new_shared_state,
    };

    fn sample_requirement(device: DeviceClass) -> WorkloadRequirementDto {
        WorkloadRequirementDto {
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
            capabilities: vec![WorkerCapabilityDto {
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
    ) -> Option<dlp_api::WorkerAssignmentDto> {
        response?.assignments.into_iter().next()
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
        let (status, payload) = json_response::<dlp_api::HealthResponse>(app(state), request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload, Some(dlp_api::HealthResponse::ok("control-plane")));
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
        let (status, created) = json_response::<dlp_api::CreateDeploymentResponse>(
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
        json_response::<dlp_api::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<dlp_api::RegisterWorkerResponse>(
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
    async fn slot_exhaustion_keeps_extra_replica_pending() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 2,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<dlp_api::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<dlp_api::RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
        )
        .await;

        let first_pending_assignment = first_assignment(
            json_response::<WorkerHeartbeatResponse>(
                app(state.clone()),
                post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                    state: WorkerState::Ready,
                }),
            )
            .await
            .1,
        );
        let second_pending_assignment = first_assignment(
            json_response::<WorkerHeartbeatResponse>(
                app(state.clone()),
                post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                    state: WorkerState::Ready,
                }),
            )
            .await
            .1,
        );

        assert!(first_pending_assignment.is_none());
        assert!(second_pending_assignment.is_some());

        let (status, replicas) =
            json_response::<ListReplicasResponse>(app(state), get_request("/replicas")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            replicas.as_ref().map(|response| {
                response
                    .replicas
                    .iter()
                    .filter(|replica| replica.state == ReplicaState::Pending)
                    .count()
            }),
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
        json_response::<dlp_api::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<dlp_api::RegisterWorkerResponse>(
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
        let assignment = first_assignment(
            json_response::<WorkerHeartbeatResponse>(
                app(state.clone()),
                post_json("/workers/worker-1/heartbeat", &WorkerHeartbeatRequest {
                    state: WorkerState::Ready,
                }),
            )
            .await
            .1,
        );

        let (status, _) = json_response::<dlp_api::replicas::ReplicaDto>(
            app(state),
            post_json(
                &format!(
                    "/replicas/{}/status",
                    assignment
                        .as_ref()
                        .map_or("", |value| value.replica_id.as_str())
                ),
                &UpdateReplicaStatusRequest {
                    lease_id:       "lease-does-not-match".to_owned(),
                    state:          ReplicaState::Ready,
                    status_message: Some("late ready".to_owned()),
                },
            ),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
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
        json_response::<dlp_api::CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<dlp_api::RegisterWorkerResponse>(
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

        let service = ControlPlaneService::new(state.clone());
        assert!(
            service
                .force_last_heartbeat_age(
                    &dlp_domain::WorkerId::new("worker-1").expect("valid"),
                    DEFAULT_WORKER_LOST_TIMEOUT + Duration::from_secs(1),
                )
                .await
                .expect("force heartbeat age should succeed")
        );
        service.reconcile_once().await;

        let (status, workers) =
            json_response::<ListWorkersResponse>(app(state.clone()), get_request("/workers")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            workers.map(|response| response.workers.first().map(|worker| worker.state.clone())),
            Some(Some(WorkerState::Lost))
        );

        let (_, deployment) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request("/deployments/deployment-1"),
        )
        .await;
        assert_eq!(
            deployment.map(|response| response.deployment.status.stopped_replicas),
            Some(1)
        );
    }
}
