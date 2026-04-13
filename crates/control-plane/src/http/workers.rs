use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use dlp_api::workers::{
    ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse, WorkerHeartbeatRequest,
    WorkerHeartbeatResponse,
};
use dlp_domain::{errors::DomainError, ids::WorkerId};

use crate::{
    application::{ControlPlaneService, SharedState},
    mappers,
};

pub(crate) async fn register_worker(
    State(state): State<SharedState>,
    Json(request): Json<RegisterWorkerRequest>,
) -> Result<Json<RegisterWorkerResponse>, (StatusCode, String)> {
    let service = ControlPlaneService::new(state.clone());
    let worker = service.register_worker(request).await.map_err(map_error)?;
    service.reconcile_once().await.map_err(internal_error)?;
    Ok(Json(RegisterWorkerResponse {
        worker: mappers::worker_to_dto(&worker),
    }))
}

pub(crate) async fn heartbeat_worker(
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
        .map_err(internal_error)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown worker: {worker_id}"),
            )
        })?;
    service.reconcile_once().await.map_err(internal_error)?;

    Ok(Json(WorkerHeartbeatResponse {
        acknowledged: true,
        assignments,
        worker: mappers::worker_to_dto(&worker),
    }))
}

pub(crate) async fn list_workers(
    State(state): State<SharedState>,
) -> Result<Json<ListWorkersResponse>, (StatusCode, String)> {
    let service = ControlPlaneService::new(state);
    let workers = service.list_workers().await.map_err(internal_error)?;
    Ok(Json(ListWorkersResponse {
        workers: workers.iter().map(mappers::worker_to_dto).collect(),
    }))
}

fn internal_error(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn map_error(error: anyhow::Error) -> (StatusCode, String) {
    if let Some(domain_error) = error.downcast_ref::<DomainError>() {
        return (StatusCode::BAD_REQUEST, domain_error.to_string());
    }

    internal_error(error)
}
