//! Replica lifecycle state.

use strum::Display;

use crate::{
    errors::{DomainError, DomainResult},
    ids::{DeploymentId, LeaseId, ReplicaId, WorkerId},
};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ReplicaState {
    Assigned,
    Failed,
    Pending,
    Pulling,
    Ready,
    Starting,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replica {
    id:             ReplicaId,
    deployment_id:  DeploymentId,
    lease_id:       Option<LeaseId>,
    state:          ReplicaState,
    status_message: Option<String>,
    worker_id:      Option<WorkerId>,
}

impl Replica {
    #[must_use]
    pub fn new_pending(id: ReplicaId, deployment_id: DeploymentId) -> Self {
        Self {
            id,
            deployment_id,
            lease_id: None,
            state: ReplicaState::Pending,
            status_message: Some("pending scheduling".to_owned()),
            worker_id: None,
        }
    }

    #[must_use]
    pub fn id(&self) -> &ReplicaId {
        &self.id
    }

    #[must_use]
    pub fn deployment_id(&self) -> &DeploymentId {
        &self.deployment_id
    }

    #[must_use]
    pub fn lease_id(&self) -> Option<&LeaseId> {
        self.lease_id.as_ref()
    }

    #[must_use]
    pub fn worker_id(&self) -> Option<&WorkerId> {
        self.worker_id.as_ref()
    }

    #[must_use]
    pub fn state(&self) -> &ReplicaState {
        &self.state
    }

    #[must_use]
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn assign(&mut self, worker_id: WorkerId, lease_id: LeaseId) -> DomainResult<()> {
        if self.state != ReplicaState::Pending {
            return Err(DomainError::InvalidStateTransition {
                entity: "replica",
                from:   self.state.to_string(),
                to:     ReplicaState::Assigned.to_string(),
            });
        }

        self.state = ReplicaState::Assigned;
        self.worker_id = Some(worker_id.clone());
        self.lease_id = Some(lease_id);
        self.status_message = Some(format!("assigned to {worker_id}"));
        Ok(())
    }

    pub fn update_status(
        &mut self,
        state: ReplicaState,
        status_message: Option<String>,
    ) -> DomainResult<()> {
        let is_valid = matches!(
            (&self.state, &state),
            (ReplicaState::Pending, ReplicaState::Assigned)
                | (ReplicaState::Assigned, ReplicaState::Pulling)
                | (ReplicaState::Assigned, ReplicaState::Failed)
                | (ReplicaState::Pulling, ReplicaState::Starting)
                | (ReplicaState::Pulling, ReplicaState::Failed)
                | (ReplicaState::Starting, ReplicaState::Ready)
                | (ReplicaState::Starting, ReplicaState::Failed)
                | (ReplicaState::Ready, ReplicaState::Failed)
                | (_, ReplicaState::Stopped)
        ) || self.state == state;

        if !is_valid {
            return Err(DomainError::InvalidStateTransition {
                entity: "replica",
                from:   self.state.to_string(),
                to:     state.to_string(),
            });
        }

        self.state = state;
        self.status_message = status_message;
        Ok(())
    }

    pub fn mark_stopped(&mut self, message: impl Into<String>) {
        self.state = ReplicaState::Stopped;
        self.status_message = Some(message.into());
    }
}
