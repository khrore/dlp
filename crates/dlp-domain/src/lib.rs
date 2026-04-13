//! Core domain model for DLP.

mod artifacts;
mod deployments;
mod errors;
mod ids;
mod leases;
mod replicas;
mod requirements;
mod workers;

pub use artifacts::ArtifactRef;
pub use deployments::{Deployment, DeploymentStatusCounts, DeploymentStatusSummary};
pub use errors::{DomainError, DomainResult};
pub use ids::{DeploymentId, LeaseId, ReplicaId, WorkerId};
pub use leases::{Lease, LeaseState};
pub use replicas::{Replica, ReplicaState};
pub use requirements::{
    ArchitectureFamily, DeviceClass, Framework, RuntimeName, WorkerCapability,
    WorkerCapabilitySpec, WorkloadMode, WorkloadProfile, WorkloadRequirement,
    WorkloadRequirementSpec,
};
pub use workers::{Worker, WorkerState};
