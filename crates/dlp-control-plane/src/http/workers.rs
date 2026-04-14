use axum::{
    Json,
    extract::{Path, State},
};
use dlp_api::workers::{
    ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse, WorkerHeartbeatRequest,
    WorkerHeartbeatResponse,
};
use dlp_domain::WorkerId;

use crate::{SharedState, application::ControlPlaneService, http::HttpError, mappers};

pub(super) async fn register_worker(
    State(state): State<SharedState>,
    Json(request): Json<RegisterWorkerRequest>,
) -> Result<Json<RegisterWorkerResponse>, HttpError> {
    let service = ControlPlaneService::new(state.clone());
    let worker = service.register_worker(request).await?;
    service.reconcile_once().await?;
    Ok(Json(RegisterWorkerResponse {
        worker: mappers::worker_to_dto(&worker),
    }))
}

pub(super) async fn heartbeat_worker(
    State(state): State<SharedState>,
    Path(worker_id): Path<String>,
    Json(request): Json<WorkerHeartbeatRequest>,
) -> Result<Json<WorkerHeartbeatResponse>, HttpError> {
    let worker_id = WorkerId::new(worker_id)?;
    let service = ControlPlaneService::new(state);
    let (worker, assignments) = service
        .heartbeat_worker(&worker_id, mappers::worker_state_from_dto(request.state))
        .await?
        .ok_or_else(|| HttpError::NotFound(format!("unknown worker: {worker_id}")))?;

    Ok(Json(WorkerHeartbeatResponse {
        acknowledged: true,
        assignments,
        worker: mappers::worker_to_dto(&worker),
    }))
}

pub(super) async fn list_workers(
    State(state): State<SharedState>,
) -> Result<Json<ListWorkersResponse>, HttpError> {
    let service = ControlPlaneService::new(state);
    let workers = service.list_workers().await?;
    Ok(Json(ListWorkersResponse {
        workers: workers.iter().map(mappers::worker_to_dto).collect(),
    }))
}
