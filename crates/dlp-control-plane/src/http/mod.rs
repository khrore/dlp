mod deployments;
mod workers;

use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use dlp_api::health::StatusDto;
use dlp_domain::DomainError;

use crate::{ControlPlaneError, SharedState, application::UpdateReplicaStatusError};

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
    Json(StatusDto::ok("dlp-control-plane"))
}

#[derive(Debug, thiserror::Error)]
enum HttpError {
    #[error(transparent)]
    ControlPlane(#[from] ControlPlaneError),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self {
            Self::ControlPlane(error) => {
                let status = if error.domain_error().is_some() {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };
                (status, error.to_string()).into_response()
            }
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::Conflict(message) => (StatusCode::CONFLICT, message).into_response(),
        }
    }
}

impl From<UpdateReplicaStatusError> for HttpError {
    fn from(error: UpdateReplicaStatusError) -> Self {
        match error {
            UpdateReplicaStatusError::Internal(error) => Self::ControlPlane(error),
            UpdateReplicaStatusError::UnknownReplica => {
                Self::NotFound("unknown replica".to_owned())
            }
            UpdateReplicaStatusError::LeaseConflict(message) => Self::Conflict(message),
        }
    }
}

impl From<DomainError> for HttpError {
    fn from(error: DomainError) -> Self {
        Self::ControlPlane(error.into())
    }
}
