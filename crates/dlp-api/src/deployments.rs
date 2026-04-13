use serde::{Deserialize, Serialize};

use crate::{replicas::ReplicaDto, shared::WorkloadRequirementDto};

/// Replica counts grouped by deployment lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentStatusSummaryDto {
    /// Number of replicas assigned to workers.
    pub assigned_replicas: u32,
    /// Number of replicas in a failed state.
    pub failed_replicas:   u32,
    /// Number of replicas waiting for assignment.
    pub pending_replicas:  u32,
    /// Number of replicas currently pulling artifacts.
    pub pulling_replicas:  u32,
    /// Number of replicas ready to serve work.
    pub ready_replicas:    u32,
    /// Number of replicas starting their runtime.
    pub starting_replicas: u32,
    /// Number of replicas stopped after being lost or terminated.
    pub stopped_replicas:  u32,
}

/// Deployment resource returned by API responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentDto {
    /// Artifact reference assigned to the deployment.
    pub artifact_ref:     String,
    /// Stable deployment identifier.
    pub id:               String,
    /// Human-readable deployment name.
    pub name:             String,
    /// Requested replica count.
    pub replicas_desired: u32,
    /// Workload requirements used for scheduling.
    pub requirement:      WorkloadRequirementDto,
    /// Current deployment status summary.
    pub status:           DeploymentStatusSummaryDto,
}

/// Request body for creating a deployment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentRequest {
    /// Artifact reference to deploy.
    pub artifact_ref:     String,
    /// Human-readable deployment name.
    pub name:             String,
    /// Requested replica count.
    pub replicas_desired: u32,
    /// Workload requirements used for scheduling.
    pub requirement:      WorkloadRequirementDto,
}

/// Response body returned after creating a deployment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentResponse {
    /// The created deployment resource.
    pub deployment: DeploymentDto,
}

/// Response body returned when fetching a deployment and its replicas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetDeploymentResponse {
    /// The deployment resource.
    pub deployment: DeploymentDto,
    /// Replicas currently tracked for the deployment.
    pub replicas:   Vec<ReplicaDto>,
}
