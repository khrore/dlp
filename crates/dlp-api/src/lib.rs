//! API DTOs for DLP.

pub mod deployments;
pub mod health;
pub mod replicas;
pub mod shared;
pub mod workers;

pub use deployments::{
    CreateDeploymentRequest, CreateDeploymentResponse, DeploymentDto, DeploymentStatusSummaryDto,
    GetDeploymentResponse,
};
pub use health::HealthResponse;
pub use replicas::{ListReplicasResponse, ReplicaDto, ReplicaState, UpdateReplicaStatusRequest};
pub use shared::{DeviceClass, Framework, WorkloadMode, WorkloadRequirementDto};
pub use workers::{
    ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse, WorkerAssignmentDto,
    WorkerCapabilityDto, WorkerDto, WorkerHeartbeatRequest, WorkerHeartbeatResponse, WorkerState,
};
