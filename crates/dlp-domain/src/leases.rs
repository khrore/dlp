//! Lease lifecycle state.

use strum::Display;

use crate::{
    ids::{DeploymentId, LeaseId, ReplicaId, WorkerId},
    requirements::WorkloadRequirement,
};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
/// Lifecycle state for a worker lease.
pub enum LeaseState {
    /// The lease is currently active.
    Active,
    /// The lease expired without a clean shutdown.
    Expired,
    /// The lease was explicitly released.
    Released,
}

/// Assignment of one replica to one worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    id:            LeaseId,
    worker_id:     WorkerId,
    deployment_id: DeploymentId,
    replica_id:    ReplicaId,
    requirement:   WorkloadRequirement,
    state:         LeaseState,
}

impl Lease {
    /// Creates a new active lease.
    #[must_use]
    pub const fn new(
        id: LeaseId,
        worker_id: WorkerId,
        deployment_id: DeploymentId,
        replica_id: ReplicaId,
        requirement: WorkloadRequirement,
    ) -> Self {
        Self {
            id,
            worker_id,
            deployment_id,
            replica_id,
            requirement,
            state: LeaseState::Active,
        }
    }

    /// Returns the lease identifier.
    #[must_use]
    pub const fn id(&self) -> &LeaseId {
        &self.id
    }

    /// Returns the owning worker identifier.
    #[must_use]
    pub const fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }

    /// Returns the deployment identifier for this lease.
    #[must_use]
    pub const fn deployment_id(&self) -> &DeploymentId {
        &self.deployment_id
    }

    /// Returns the replica identifier for this lease.
    #[must_use]
    pub const fn replica_id(&self) -> &ReplicaId {
        &self.replica_id
    }

    /// Returns the workload requirement held by this lease.
    #[must_use]
    pub const fn requirement(&self) -> &WorkloadRequirement {
        &self.requirement
    }

    /// Returns the current lease state.
    #[must_use]
    pub const fn state(&self) -> &LeaseState {
        &self.state
    }

    /// Marks the lease as expired.
    pub const fn expire(&mut self) {
        self.state = LeaseState::Expired;
    }

    /// Marks the lease as released.
    pub const fn release(&mut self) {
        self.state = LeaseState::Released;
    }

    /// Reconstructs a lease from persisted state.
    #[must_use]
    pub const fn rehydrate(
        id: LeaseId,
        worker_id: WorkerId,
        deployment_id: DeploymentId,
        replica_id: ReplicaId,
        requirement: WorkloadRequirement,
        state: LeaseState,
    ) -> Self {
        Self {
            id,
            worker_id,
            deployment_id,
            replica_id,
            requirement,
            state,
        }
    }
}
