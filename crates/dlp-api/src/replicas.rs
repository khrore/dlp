use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Lifecycle states reported for a replica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ReplicaState {
    /// The replica has been assigned to a worker.
    Assigned,
    /// The replica has failed and will not progress further.
    Failed,
    /// The replica is waiting for assignment.
    Pending,
    /// The replica is pulling its artifact or dependencies.
    Pulling,
    /// The replica is ready to serve work.
    Ready,
    /// The replica runtime is starting.
    Starting,
    /// The replica was stopped after assignment.
    Stopped,
}

/// Replica resource returned by API responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplicaDto {
    /// Identifier of the owning deployment.
    pub deployment_id:  String,
    /// Stable replica identifier.
    pub id:             String,
    /// Active lease identifier, when assigned.
    pub lease_id:       Option<String>,
    /// Current replica lifecycle state.
    pub state:          ReplicaState,
    /// Human-readable status message from the worker or control plane.
    pub status_message: Option<String>,
    /// Assigned worker identifier, when present.
    pub worker_id:      Option<String>,
}

/// Response body for listing replicas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReplicasResponse {
    /// Replicas matching the request filter.
    pub replicas: Vec<ReplicaDto>,
}

/// Request body for updating replica status from a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateReplicaStatusRequest {
    /// Active lease identifier for optimistic concurrency checks.
    pub lease_id:       String,
    /// Next replica lifecycle state.
    pub state:          ReplicaState,
    /// Optional status message describing the transition.
    pub status_message: Option<String>,
}
