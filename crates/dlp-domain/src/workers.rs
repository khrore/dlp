//! Worker lifecycle state.

use strum::Display;

use crate::{ids::WorkerId, requirements::WorkerCapability};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
/// Lifecycle state for a registered worker.
pub enum WorkerState {
    /// The worker is draining existing assignments.
    Draining,
    /// The worker has been considered lost.
    Lost,
    /// The worker is ready for new assignments.
    Ready,
    /// The worker is starting up.
    Starting,
    /// The worker is unhealthy.
    Unhealthy,
}

/// Worker entity and its current capacity summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worker {
    id:                WorkerId,
    display_name:      String,
    capabilities:      Vec<WorkerCapability>,
    state:             WorkerState,
    assigned_replicas: u32,
    available_slots:   u32,
}

impl Worker {
    /// Creates a new worker in the starting state.
    #[must_use]
    pub const fn new(
        id: WorkerId,
        display_name: String,
        capabilities: Vec<WorkerCapability>,
    ) -> Self {
        Self {
            id,
            display_name,
            capabilities,
            state: WorkerState::Starting,
            assigned_replicas: 0,
            available_slots: 0,
        }
    }

    /// Returns the worker capabilities.
    #[must_use]
    pub fn capabilities(&self) -> &[WorkerCapability] {
        &self.capabilities
    }

    /// Returns the worker display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the worker identifier.
    #[must_use]
    pub const fn id(&self) -> &WorkerId {
        &self.id
    }

    /// Returns the number of assigned replicas.
    #[must_use]
    pub const fn assigned_replicas(&self) -> u32 {
        self.assigned_replicas
    }

    /// Returns the number of available concurrency slots.
    #[must_use]
    pub const fn available_slots(&self) -> u32 {
        self.available_slots
    }

    /// Returns the current worker state.
    #[must_use]
    pub const fn state(&self) -> &WorkerState {
        &self.state
    }

    /// Updates the worker lifecycle state.
    pub const fn set_state(&mut self, state: WorkerState) {
        self.state = state;
    }

    /// Updates the current assignment and capacity summary.
    pub const fn set_capacity_summary(&mut self, assigned_replicas: u32, available_slots: u32) {
        self.assigned_replicas = assigned_replicas;
        self.available_slots = available_slots;
    }

    /// Reconstructs a worker from persisted state.
    #[must_use]
    pub const fn rehydrate(
        id: WorkerId,
        display_name: String,
        capabilities: Vec<WorkerCapability>,
        state: WorkerState,
        assigned_replicas: u32,
        available_slots: u32,
    ) -> Self {
        Self {
            id,
            display_name,
            capabilities,
            state,
            assigned_replicas,
            available_slots,
        }
    }
}
