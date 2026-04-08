use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use client_sdk::{
    CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse, ListReplicasResponse,
    ModelReplica, UpdateReplicaStatusRequest,
};
use serde::Deserialize;

use crate::{
    reconcile::reconcile_once,
    state::{SharedState, UpdateReplicaStatusError},
};

#[derive(Debug, Deserialize)]
pub struct ReplicaListQuery {
    deployment_id: Option<String>,
}

impl ReplicaListQuery {
    fn deployment_id(&self) -> Option<&str> {
        self.deployment_id.as_deref()
    }
}

pub(crate) async fn create_deployment(
    State(state): State<SharedState>,
    Json(request): Json<CreateDeploymentRequest>,
) -> Result<Json<CreateDeploymentResponse>, (StatusCode, String)> {
    let response = {
        let mut guard = state.lock().await;
        guard.create_deployment(request)
    };
    reconcile_once(&state).await;

    Ok(Json(response))
}

pub(crate) async fn get_deployment(
    State(state): State<SharedState>,
    Path(deployment_id): Path<String>,
) -> Result<Json<GetDeploymentResponse>, (StatusCode, String)> {
    let response = {
        let mut guard = state.lock().await;
        guard.get_deployment(&deployment_id)
    }
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("unknown deployment: {deployment_id}"),
        )
    })?;

    Ok(Json(response))
}

pub(crate) async fn list_replicas(
    State(state): State<SharedState>,
    Query(query): Query<ReplicaListQuery>,
) -> Result<Json<ListReplicasResponse>, (StatusCode, String)> {
    let response = {
        let guard = state.lock().await;
        guard.list_replicas(query.deployment_id())
    };

    Ok(Json(response))
}

pub(crate) async fn update_replica_status(
    State(state): State<SharedState>,
    Path(replica_id): Path<String>,
    Json(request): Json<UpdateReplicaStatusRequest>,
) -> Result<Json<ModelReplica>, (StatusCode, String)> {
    let response = {
        let mut guard = state.lock().await;
        guard.update_replica_status(&replica_id, request)
    }
    .map_err(|error| match error {
        UpdateReplicaStatusError::UnknownReplica => (
            StatusCode::NOT_FOUND,
            format!("unknown replica: {replica_id}"),
        ),
        UpdateReplicaStatusError::LeaseConflict(message) => (StatusCode::CONFLICT, message),
    })?;
    reconcile_once(&state).await;

    Ok(Json(response))
}
