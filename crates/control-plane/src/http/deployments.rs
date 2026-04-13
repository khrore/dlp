#![expect(
    clippy::shadow_reuse,
    reason = "Path parameters are re-bound into validated domain identifiers for request handling."
)]

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use dlp_api::{
    deployments::{CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse},
    replicas::{ListReplicasResponse, ReplicaDto, UpdateReplicaStatusRequest},
};
use dlp_domain::{DeploymentId, DomainError, ReplicaId};
use serde::Deserialize;

use crate::{
    application::{ControlPlaneService, UpdateReplicaStatusError},
    mappers,
    SharedState,
};

#[derive(Debug, Deserialize)]
pub(super) struct ReplicaListQuery {
    deployment_id: Option<String>,
}

pub(super) async fn create_deployment(
    State(state): State<SharedState>,
    Json(request): Json<CreateDeploymentRequest>,
) -> Result<Json<CreateDeploymentResponse>, (StatusCode, String)> {
    let service = ControlPlaneService::new(state.clone());
    let deployment = service
        .create_deployment(request)
        .await
        .map_err(map_error)?;
    service
        .reconcile_once()
        .await
        .map_err(|error| internal_error(&error))?;
    Ok(Json(CreateDeploymentResponse {
        deployment: mappers::deployment_to_dto(&deployment),
    }))
}

pub(super) async fn get_deployment(
    State(state): State<SharedState>,
    Path(deployment_id): Path<String>,
) -> Result<Json<GetDeploymentResponse>, (StatusCode, String)> {
    let deployment_id = DeploymentId::new(deployment_id).map_err(|error| invalid_request(&error))?;
    let service = ControlPlaneService::new(state);
    let (deployment, replicas) = service
        .get_deployment(&deployment_id)
        .await
        .map_err(|error| internal_error(&error))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown deployment: {deployment_id}"),
            )
        })?;

    Ok(Json(GetDeploymentResponse {
        deployment: mappers::deployment_to_dto(&deployment),
        replicas:   replicas.iter().map(mappers::replica_to_dto).collect(),
    }))
}

pub(super) async fn list_replicas(
    State(state): State<SharedState>,
    Query(query): Query<ReplicaListQuery>,
) -> Result<Json<ListReplicasResponse>, (StatusCode, String)> {
    let deployment_id = query
        .deployment_id
        .map(DeploymentId::new)
        .transpose()
        .map_err(|error| invalid_request(&error))?;
    let service = ControlPlaneService::new(state);
    let replicas = service
        .list_replicas(deployment_id.as_ref())
        .await
        .map_err(|error| internal_error(&error))?;

    Ok(Json(ListReplicasResponse {
        replicas: replicas.iter().map(mappers::replica_to_dto).collect(),
    }))
}

pub(super) async fn update_replica_status(
    State(state): State<SharedState>,
    Path(replica_id): Path<String>,
    Json(request): Json<UpdateReplicaStatusRequest>,
) -> Result<Json<ReplicaDto>, (StatusCode, String)> {
    let replica_id = ReplicaId::new(replica_id).map_err(|error| invalid_request(&error))?;
    let service = ControlPlaneService::new(state.clone());
    let replica = service
        .update_replica_status(&replica_id, request)
        .await
        .map_err(|error| match error {
            UpdateReplicaStatusError::UnknownReplica => (
                StatusCode::NOT_FOUND,
                format!("unknown replica: {replica_id}"),
            ),
            UpdateReplicaStatusError::LeaseConflict(message) => (StatusCode::CONFLICT, message),
        })?;
    service
        .reconcile_once()
        .await
        .map_err(|error| internal_error(&error))?;
    Ok(Json(mappers::replica_to_dto(&replica)))
}

fn invalid_request(error: &impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error.to_string())
}

fn internal_error(error: &impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "The handler receives the owned anyhow error from map_err and inspects it before rendering."
)]
fn map_error(error: anyhow::Error) -> (StatusCode, String) {
    if let Some(domain_error) = error.downcast_ref::<DomainError>() {
        return invalid_request(domain_error);
    }

    internal_error(&error)
}
