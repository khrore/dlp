#![expect(
    clippy::redundant_pub_crate,
    reason = "Worker handlers are visible through the crate-private module boundary."
)]

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use client_sdk::{
    ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse, WorkerHeartbeatRequest,
    WorkerHeartbeatResponse,
};

use crate::{reconcile::reconcile_once, state::SharedState};

pub(crate) async fn register_worker(
    State(state): State<SharedState>,
    Json(request): Json<RegisterWorkerRequest>,
) -> Result<Json<RegisterWorkerResponse>, (StatusCode, String)> {
    let response = {
        let mut guard = state.lock().await;
        guard.register_worker(request)
    };
    reconcile_once(&state).await;

    Ok(Json(response))
}

pub(crate) async fn heartbeat_worker(
    State(state): State<SharedState>,
    Path(worker_id): Path<String>,
    Json(request): Json<WorkerHeartbeatRequest>,
) -> Result<Json<WorkerHeartbeatResponse>, (StatusCode, String)> {
    let response = {
        let mut guard = state.lock().await;
        guard.heartbeat_worker(&worker_id, request.state)
    }
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("unknown worker: {worker_id}"),
        )
    })?;
    reconcile_once(&state).await;

    Ok(Json(response))
}

pub(crate) async fn list_workers(
    State(state): State<SharedState>,
) -> Result<Json<ListWorkersResponse>, (StatusCode, String)> {
    let response = {
        let mut guard = state.lock().await;
        guard.list_workers()
    };

    Ok(Json(response))
}
