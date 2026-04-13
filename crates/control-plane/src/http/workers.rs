#![expect(
    clippy::shadow_reuse,
    reason = "Path parameters are re-bound into validated domain identifiers for request handling."
)]

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use dlp_api::workers::{
    ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse, WorkerHeartbeatRequest,
    WorkerHeartbeatResponse,
};
use dlp_domain::{DomainError, WorkerId};

use crate::{SharedState, application::ControlPlaneService, mappers};

pub(super) async fn register_worker(
    State(state): State<SharedState>,
    Json(request): Json<RegisterWorkerRequest>,
) -> Result<Json<RegisterWorkerResponse>, (StatusCode, String)> {
    let service = ControlPlaneService::new(state.clone());
    let worker = service.register_worker(request).await.map_err(map_error)?;
    service
        .reconcile_once()
        .await
        .map_err(|error| internal_error(&error))?;
    Ok(Json(RegisterWorkerResponse {
        worker: mappers::worker_to_dto(&worker),
    }))
}

pub(super) async fn heartbeat_worker(
    State(state): State<SharedState>,
    Path(worker_id): Path<String>,
    Json(request): Json<WorkerHeartbeatRequest>,
) -> Result<Json<WorkerHeartbeatResponse>, (StatusCode, String)> {
    let worker_id =
        WorkerId::new(worker_id).map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let service = ControlPlaneService::new(state.clone());
    let (worker, assignments) = service
        .heartbeat_worker(&worker_id, mappers::worker_state_from_dto(request.state))
        .await
        .map_err(|error| internal_error(&error))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown worker: {worker_id}"),
            )
        })?;
    service
        .reconcile_once()
        .await
        .map_err(|error| internal_error(&error))?;

    Ok(Json(WorkerHeartbeatResponse {
        acknowledged: true,
        assignments,
        worker: mappers::worker_to_dto(&worker),
    }))
}

pub(super) async fn list_workers(
    State(state): State<SharedState>,
) -> Result<Json<ListWorkersResponse>, (StatusCode, String)> {
    let service = ControlPlaneService::new(state);
    let workers = service
        .list_workers()
        .await
        .map_err(|error| internal_error(&error))?;
    Ok(Json(ListWorkersResponse {
        workers: workers.iter().map(mappers::worker_to_dto).collect(),
    }))
}

fn internal_error(error: &impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "The handler receives the owned anyhow error from map_err and inspects it before \
              rendering."
)]
fn map_error(error: anyhow::Error) -> (StatusCode, String) {
    if let Some(domain_error) = error.downcast_ref::<DomainError>() {
        return (StatusCode::BAD_REQUEST, domain_error.to_string());
    }

    internal_error(&error)
}
