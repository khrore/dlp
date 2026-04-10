use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use dlp_api::{
    CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse, ListReplicasResponse,
    ReplicaDto, UpdateReplicaStatusRequest,
};
use dlp_domain::DeploymentId;
use serde::Deserialize;

use crate::{
    application::{ControlPlaneService, SharedState, UpdateReplicaStatusError},
    mappers,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ReplicaListQuery {
    deployment_id: Option<String>,
}

pub(crate) async fn create_deployment(
    State(state): State<SharedState>,
    Json(request): Json<CreateDeploymentRequest>,
) -> Result<Json<CreateDeploymentResponse>, (StatusCode, String)> {
    let service = ControlPlaneService::new(state.clone());
    let deployment = service
        .create_deployment(request)
        .await
        .map_err(invalid_request)?;
    service.reconcile_once().await;
    Ok(Json(CreateDeploymentResponse {
        deployment: mappers::deployment_to_dto(&deployment),
    }))
}

pub(crate) async fn get_deployment(
    State(state): State<SharedState>,
    Path(deployment_id): Path<String>,
) -> Result<Json<GetDeploymentResponse>, (StatusCode, String)> {
    let deployment_id = DeploymentId::new(deployment_id).map_err(invalid_request)?;
    let service = ControlPlaneService::new(state);
    let (deployment, replicas) = service
        .get_deployment(&deployment_id)
        .await
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

pub(crate) async fn list_replicas(
    State(state): State<SharedState>,
    Query(query): Query<ReplicaListQuery>,
) -> Result<Json<ListReplicasResponse>, (StatusCode, String)> {
    let deployment_id = query
        .deployment_id
        .map(DeploymentId::new)
        .transpose()
        .map_err(invalid_request)?;
    let service = ControlPlaneService::new(state);
    let replicas = service.list_replicas(deployment_id.as_ref()).await;

    Ok(Json(ListReplicasResponse {
        replicas: replicas.iter().map(mappers::replica_to_dto).collect(),
    }))
}

pub(crate) async fn update_replica_status(
    State(state): State<SharedState>,
    Path(replica_id): Path<String>,
    Json(request): Json<UpdateReplicaStatusRequest>,
) -> Result<Json<ReplicaDto>, (StatusCode, String)> {
    let replica_id = dlp_domain::ReplicaId::new(replica_id).map_err(invalid_request)?;
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
    service.reconcile_once().await;
    Ok(Json(mappers::replica_to_dto(&replica)))
}

fn invalid_request(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error.to_string())
}
