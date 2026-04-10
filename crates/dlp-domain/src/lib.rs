//! Core domain model for DLP.

pub mod artifacts;
pub mod deployments;
pub mod errors;
pub mod ids;
pub mod leases;
pub mod replicas;
pub mod requirements;
pub mod workers;

pub use artifacts::ArtifactRef;
pub use deployments::{Deployment, DeploymentStatusSummary};
pub use errors::{DomainError, DomainResult};
pub use ids::{DeploymentId, LeaseId, ReplicaId, WorkerId};
pub use leases::{Lease, LeaseState};
pub use replicas::{Replica, ReplicaState};
pub use requirements::{
    ArchitectureFamily, DeviceClass, Framework, RuntimeName, WorkerCapability, WorkloadMode,
    WorkloadRequirement,
};
pub use workers::{Worker, WorkerState};
