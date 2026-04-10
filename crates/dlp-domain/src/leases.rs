//! Lease lifecycle state.

use strum::Display;

use crate::{
    ids::{DeploymentId, LeaseId, ReplicaId, WorkerId},
    requirements::WorkloadRequirement,
};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum LeaseState {
    Active,
    Expired,
    Released,
}

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
    #[must_use]
    pub fn new(
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

    #[must_use]
    pub fn id(&self) -> &LeaseId {
        &self.id
    }

    #[must_use]
    pub fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }

    #[must_use]
    pub fn deployment_id(&self) -> &DeploymentId {
        &self.deployment_id
    }

    #[must_use]
    pub fn replica_id(&self) -> &ReplicaId {
        &self.replica_id
    }

    #[must_use]
    pub fn requirement(&self) -> &WorkloadRequirement {
        &self.requirement
    }

    #[must_use]
    pub fn state(&self) -> &LeaseState {
        &self.state
    }

    pub fn expire(&mut self) {
        self.state = LeaseState::Expired;
    }

    pub fn release(&mut self) {
        self.state = LeaseState::Released;
    }
}
