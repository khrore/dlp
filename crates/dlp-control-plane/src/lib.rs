//! Axum application and integration tests for the DLP control plane.

mod application;
mod domain_services;
mod errors;
mod http;
mod mappers;
mod repositories;

use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::Arc,
};

use application::ControlPlaneService;
use axum::Router;
use clap as _;
use dlp_config as _;
use dlp_config::{ControlPlaneConfig, StorageBackend as ConfigStorageBackend};
use env_logger as _;
use log as _;
use repositories::{
    MemoryStorage, Migrator, PostgresStorage, StorageBackend as RuntimeStorageBackend,
};
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::MigratorTrait as _;
use tokio::time::{self, MissedTickBehavior};

pub use self::errors::{ControlPlaneError, Result};

#[doc(hidden)]
pub mod internal {
    pub use super::{
        application::{ControlPlaneService, UpdateReplicaStatusError},
        domain_services::{
            reconcile,
            reconcile::{DEFAULT_RECONCILE_INTERVAL, DEFAULT_WORKER_LOST_TIMEOUT},
            scheduler,
            scheduler::{
                available_capacity_for_requirement, capability_matches, worker_is_eligible,
            },
        },
        http::router,
        mappers::{
            artifact_ref_from_string, assignment_to_dto, capability_from_dto, capability_to_dto,
            deployment_status_to_dto, deployment_to_dto, replica_state_from_dto, replica_to_dto,
            requirement_from_dto, requirement_to_dto, worker_state_from_dto, worker_to_dto,
        },
        repositories::{
            MemoryStorage, Migrator, PostgresStorage, StorageBackend, UpdateReplicaStatusResult,
        },
    };
}

/// Shared application state wrapper used by the control plane.
#[derive(Clone)]
pub struct SharedState(Arc<dyn RuntimeStorageBackend>);

impl Debug for SharedState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_tuple("SharedState").finish()
    }
}

impl SharedState {
    fn new(storage: Arc<dyn RuntimeStorageBackend>) -> Self {
        Self(storage)
    }
}

/// Creates a control-plane state backed by in-memory storage.
#[must_use]
pub fn new_shared_state() -> SharedState {
    SharedState::new(Arc::new(MemoryStorage::new()))
}

/// Creates a control-plane state from the configured storage backend.
///
/// # Errors
///
/// Returns an error when the configured database connection cannot be
/// established or when migrations fail.
pub async fn new_shared_state_from_config(config: &ControlPlaneConfig) -> Result<SharedState> {
    let storage: Arc<dyn RuntimeStorageBackend> = match config.storage.backend {
        ConfigStorageBackend::Memory => Arc::new(MemoryStorage::new()),
        ConfigStorageBackend::Postgres => {
            let database_url = config
                .storage
                .database_url
                .as_deref()
                .ok_or(ControlPlaneError::MissingDatabaseUrl)?;
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

/// Starts the background reconcile loop for the control plane.
pub fn spawn_reconcile_loop(state: SharedState) {
    tokio::spawn(async move {
        let mut ticker = time::interval(domain_services::reconcile::DEFAULT_RECONCILE_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            drop(
                ControlPlaneService::new(state.clone())
                    .reconcile_once()
                    .await,
            );
        }
    });
}

/// Builds the Axum router for the control-plane API.
pub fn app(state: SharedState) -> Router {
    http::router(state)
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs, path::Path, time::Duration};

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use dlp_api::{
        deployments::{CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse},
        health::StatusDto,
        replicas::{ListReplicasResponse, ReplicaDto, ReplicaState, UpdateReplicaStatusRequest},
        shared::{DeviceClass, Framework, WorkloadMode, WorkloadRequirementDto},
        workers::{
            ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse,
            WorkerAssignmentDto, WorkerCapabilityDto, WorkerHeartbeatRequest,
            WorkerHeartbeatResponse, WorkerState,
        },
    };
    use dlp_domain::WorkerId;
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

    fn first_assignment(response: Option<WorkerHeartbeatResponse>) -> Option<WorkerAssignmentDto> {
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
        let (status, payload) = json_response::<StatusDto>(app(state), request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload, Some(StatusDto::ok("dlp-control-plane")));
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
        let (status, created) = json_response::<CreateDeploymentResponse>(
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
    async fn ready_worker_receives_assignment_on_same_heartbeat() {
        let state = new_shared_state();
        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        json_response::<CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<RegisterWorkerResponse>(
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

        assert_eq!(first.0, StatusCode::OK);
        assert_eq!(first.1.map(|response| response.assignments.len()), Some(1));
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
        json_response::<CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<RegisterWorkerResponse>(
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

        assert!(first_pending_assignment.is_some());
        assert!(second_pending_assignment.is_none());

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
        json_response::<CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<RegisterWorkerResponse>(
            app(state.clone()),
            post_json("/workers/register", &worker_request(DeviceClass::Cpu, 1)),
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

        let (status, _) = json_response::<ReplicaDto>(
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
        json_response::<CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;
        json_response::<RegisterWorkerResponse>(
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

        let service = ControlPlaneService::new(state.clone());
        assert!(
            service
                .force_last_heartbeat_age(
                    &WorkerId::new("worker-1").expect("valid"),
                    DEFAULT_WORKER_LOST_TIMEOUT + Duration::from_secs(1),
                )
                .await
                .expect("force heartbeat age should succeed")
        );
        drop(service.reconcile_once().await);

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

    #[tokio::test]
    async fn create_deployment_returns_post_reconcile_status() {
        let state = new_shared_state();
        json_response::<RegisterWorkerResponse>(
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

        let create_request = CreateDeploymentRequest {
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(DeviceClass::Cpu),
        };
        let (create_status, created) = json_response::<CreateDeploymentResponse>(
            app(state.clone()),
            post_json("/deployments", &create_request),
        )
        .await;

        assert_eq!(create_status, StatusCode::OK);
        assert_eq!(
            created
                .as_ref()
                .map(|response| response.deployment.status.assigned_replicas),
            Some(1)
        );
        assert_eq!(
            created
                .as_ref()
                .map(|response| response.deployment.status.pending_replicas),
            Some(0)
        );

        let deployment_id = created
            .as_ref()
            .map(|response| response.deployment.id.clone())
            .unwrap_or_default();
        let (get_status, fetched) = json_response::<GetDeploymentResponse>(
            app(state),
            get_request(&format!("/deployments/{deployment_id}")),
        )
        .await;

        assert_eq!(get_status, StatusCode::OK);
        assert_eq!(
            created.map(|response| response.deployment),
            fetched.map(|response| response.deployment)
        );
    }

    #[tokio::test]
    async fn register_worker_rejects_transport_unsafe_worker_id() {
        let state = new_shared_state();
        let (status, response) = json_response::<RegisterWorkerResponse>(
            app(state),
            post_json("/workers/register", &RegisterWorkerRequest {
                worker_id:    "worker/1".to_owned(),
                display_name: "trainer-1".to_owned(),
                capabilities: vec![WorkerCapabilityDto {
                    framework:              Framework::Pytorch,
                    mode:                   WorkloadMode::Training,
                    device:                 DeviceClass::Cpu,
                    accelerator_runtime:    "cpu".to_owned(),
                    architecture_family:    "generic".to_owned(),
                    available_memory_bytes: 8192,
                    concurrency_slots:      1,
                }],
            }),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(response.is_none());
    }

    #[test]
    fn application_uses_typed_storage_id_methods() {
        let source = include_str!("application/mod.rs");
        assert!(!source.contains("next_id("));
        assert!(source.contains("next_deployment_id("));
        assert!(source.contains("next_replica_id("));
        assert!(source.contains("next_lease_id("));
    }

    #[test]
    fn raw_sql_is_confined_to_postgres_modules() {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_no_raw_sql_outside_allowed_modules(&source_root);
    }

    fn assert_no_raw_sql_outside_allowed_modules(root: &Path) {
        for dir_entry in fs::read_dir(root).expect("source directory should be readable") {
            let entry = dir_entry.expect("directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                assert_no_raw_sql_outside_allowed_modules(&path);
                continue;
            }
            if path.extension().and_then(OsStr::to_str) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
                .expect("path should be under crate root");
            let relative_str = relative.to_string_lossy();
            if relative_str == "src/repositories/postgres.rs"
                || relative_str == "src/repositories/migration.rs"
                || relative_str == "src/lib.rs"
            {
                continue;
            }
            let source = fs::read_to_string(&path).expect("source file should be readable");
            assert!(
                !source.contains("Statement::from_string("),
                "unexpected raw SQL builder in {relative_str}"
            );
            assert!(
                !source.contains("execute_unprepared("),
                "unexpected unprepared SQL in {relative_str}"
            );
            assert!(
                !source.contains("SELECT nextval("),
                "unexpected sequence SQL in {relative_str}"
            );
        }
    }
}
