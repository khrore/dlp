use serde::{Deserialize, Serialize};

use crate::{replicas::ReplicaDto, shared::WorkloadRequirementDto};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentStatusSummaryDto {
    pub assigned_replicas: u32,
    pub failed_replicas:   u32,
    pub pending_replicas:  u32,
    pub pulling_replicas:  u32,
    pub ready_replicas:    u32,
    pub starting_replicas: u32,
    pub stopped_replicas:  u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentDto {
    pub artifact_ref:     String,
    pub id:               String,
    pub name:             String,
    pub replicas_desired: u32,
    pub requirement:      WorkloadRequirementDto,
    pub status:           DeploymentStatusSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentRequest {
    pub artifact_ref:     String,
    pub name:             String,
    pub replicas_desired: u32,
    pub requirement:      WorkloadRequirementDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentResponse {
    pub deployment: DeploymentDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetDeploymentResponse {
    pub deployment: DeploymentDto,
    pub replicas:   Vec<ReplicaDto>,
}
