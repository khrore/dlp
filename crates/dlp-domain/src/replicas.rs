//! Replica lifecycle state.

use strum::Display;

use crate::{
    errors::{DomainError, DomainResult},
    ids::{DeploymentId, LeaseId, ReplicaId, WorkerId},
};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
/// Lifecycle state for a replica.
pub enum ReplicaState {
    /// The replica has been assigned to a worker.
    Assigned,
    /// The replica failed during startup or execution.
    Failed,
    /// The replica is waiting for scheduling.
    Pending,
    /// The replica is pulling required artifacts.
    Pulling,
    /// The replica is ready to serve work.
    Ready,
    /// The replica is starting the runtime.
    Starting,
    /// The replica has been stopped.
    Stopped,
}

/// Replica entity tracked by the control plane.
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
    /// Creates a new pending replica for a deployment.
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

    /// Returns the replica identifier.
    #[must_use]
    pub const fn id(&self) -> &ReplicaId {
        &self.id
    }

    /// Returns the parent deployment identifier.
    #[must_use]
    pub const fn deployment_id(&self) -> &DeploymentId {
        &self.deployment_id
    }

    /// Returns the active lease identifier, if present.
    #[must_use]
    pub const fn lease_id(&self) -> Option<&LeaseId> {
        self.lease_id.as_ref()
    }

    /// Returns the assigned worker identifier, if present.
    #[must_use]
    pub const fn worker_id(&self) -> Option<&WorkerId> {
        self.worker_id.as_ref()
    }

    /// Returns the current replica state.
    #[must_use]
    pub const fn state(&self) -> &ReplicaState {
        &self.state
    }

    /// Returns the latest status message, if any.
    #[must_use]
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Assigns the replica to a worker and lease.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidStateTransition`] when the replica is not pending.
    pub fn assign(&mut self, worker_id: WorkerId, lease_id: LeaseId) -> DomainResult<()> {
        if self.state != ReplicaState::Pending {
            return Err(DomainError::InvalidStateTransition {
                entity: "replica",
                from:   self.state.to_string(),
                to:     ReplicaState::Assigned.to_string(),
            });
        }

        self.state = ReplicaState::Assigned;
        self.status_message = Some(format!("assigned to {worker_id}"));
        self.worker_id = Some(worker_id);
        self.lease_id = Some(lease_id);
        Ok(())
    }

    /// Updates the replica lifecycle state and status message.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidStateTransition`] when the requested state change is not
    /// allowed from the current replica state.
    pub fn update_status(
        &mut self,
        state: ReplicaState,
        status_message: Option<String>,
    ) -> DomainResult<()> {
        let is_valid = matches!(
            (&self.state, &state),
            (ReplicaState::Pending, ReplicaState::Assigned)
                | (ReplicaState::Assigned, ReplicaState::Pulling | ReplicaState::Failed)
                | (ReplicaState::Pulling, ReplicaState::Starting | ReplicaState::Failed)
                | (ReplicaState::Starting, ReplicaState::Ready | ReplicaState::Failed)
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

    /// Marks the replica as stopped with a final message.
    pub fn mark_stopped<Message>(&mut self, message: Message)
    where
        Message: Into<String>,
    {
        self.state = ReplicaState::Stopped;
        self.status_message = Some(message.into());
    }

    /// Reconstructs a replica from persisted state.
    #[must_use]
    pub const fn rehydrate(
        id: ReplicaId,
        deployment_id: DeploymentId,
        lease_id: Option<LeaseId>,
        state: ReplicaState,
        status_message: Option<String>,
        worker_id: Option<WorkerId>,
    ) -> Self {
        Self {
            id,
            deployment_id,
            lease_id,
            state,
            status_message,
            worker_id,
        }
    }
}
