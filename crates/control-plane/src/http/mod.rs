mod deployments;
mod workers;

use axum::{
    Json, Router,
    routing::{get, post},
};
use dlp_api::health::StatusDto;

use crate::SharedState;

/// Builds the internal Axum router for the control-plane API.
pub fn router(state: SharedState) -> Router {
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
        .with_state(state)
}

async fn health() -> Json<StatusDto> {
    Json(StatusDto::ok("control-plane"))
}
