use axum::{
    Json,
    extract::{Path, Query, State},
};
use dlp_api::{
    deployments::{CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse},
    replicas::{ListReplicasResponse, ReplicaDto, UpdateReplicaStatusRequest},
};
use dlp_domain::{DeploymentId, ReplicaId};
use serde::Deserialize;

use crate::{
    SharedState,
    application::{ControlPlaneService, UpdateReplicaStatusError},
    http::HttpError,
    mappers,
};

#[derive(Debug, Deserialize)]
pub(super) struct ReplicaListQuery {
    deployment_id: Option<String>,
}

pub(super) async fn create_deployment(
    State(state): State<SharedState>,
    Json(request): Json<CreateDeploymentRequest>,
) -> Result<Json<CreateDeploymentResponse>, HttpError> {
    let service = ControlPlaneService::new(state.clone());
    let deployment = service.create_deployment(request).await?;
    service.reconcile_once().await?;
    Ok(Json(CreateDeploymentResponse {
        deployment: mappers::deployment_to_dto(&deployment),
    }))
}

pub(super) async fn get_deployment(
    State(state): State<SharedState>,
    Path(deployment_id): Path<String>,
) -> Result<Json<GetDeploymentResponse>, HttpError> {
    let deployment_id = DeploymentId::new(deployment_id)?;
    let service = ControlPlaneService::new(state);
    let (deployment, replicas) = service
        .get_deployment(&deployment_id)
        .await?
        .ok_or_else(|| HttpError::NotFound(format!("unknown deployment: {deployment_id}")))?;

    Ok(Json(GetDeploymentResponse {
        deployment: mappers::deployment_to_dto(&deployment),
        replicas:   replicas.iter().map(mappers::replica_to_dto).collect(),
    }))
}

pub(super) async fn list_replicas(
    State(state): State<SharedState>,
    Query(query): Query<ReplicaListQuery>,
) -> Result<Json<ListReplicasResponse>, HttpError> {
    let deployment_id = query.deployment_id.map(DeploymentId::new).transpose()?;
    let service = ControlPlaneService::new(state);
    let replicas = service.list_replicas(deployment_id.as_ref()).await?;

    Ok(Json(ListReplicasResponse {
        replicas: replicas.iter().map(mappers::replica_to_dto).collect(),
    }))
}

pub(super) async fn update_replica_status(
    State(state): State<SharedState>,
    Path(replica_id): Path<String>,
    Json(request): Json<UpdateReplicaStatusRequest>,
) -> Result<Json<ReplicaDto>, HttpError> {
    let replica_id = ReplicaId::new(replica_id)?;
    let service = ControlPlaneService::new(state.clone());
    let replica = service
        .update_replica_status(&replica_id, request)
        .await
        .map_err(|error| match error {
            UpdateReplicaStatusError::UnknownReplica => {
                HttpError::NotFound(format!("unknown replica: {replica_id}"))
            }
            other @ (UpdateReplicaStatusError::Internal(_)
            | UpdateReplicaStatusError::LeaseConflict(_)) => HttpError::from(other),
        })?;
    service.reconcile_once().await?;
    Ok(Json(mappers::replica_to_dto(&replica)))
}
