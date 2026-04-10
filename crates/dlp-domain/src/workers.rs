//! Worker lifecycle state.

use strum::Display;

use crate::{ids::WorkerId, requirements::WorkerCapability};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum WorkerState {
    Draining,
    Lost,
    Ready,
    Starting,
    Unhealthy,
}

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
    #[must_use]
    pub fn new(id: WorkerId, display_name: String, capabilities: Vec<WorkerCapability>) -> Self {
        Self {
            id,
            display_name,
            capabilities,
            state: WorkerState::Starting,
            assigned_replicas: 0,
            available_slots: 0,
        }
    }

    #[must_use]
    pub fn capabilities(&self) -> &[WorkerCapability] {
        &self.capabilities
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn id(&self) -> &WorkerId {
        &self.id
    }

    #[must_use]
    pub const fn assigned_replicas(&self) -> u32 {
        self.assigned_replicas
    }

    #[must_use]
    pub const fn available_slots(&self) -> u32 {
        self.available_slots
    }

    #[must_use]
    pub fn state(&self) -> &WorkerState {
        &self.state
    }

    pub fn set_state(&mut self, state: WorkerState) {
        self.state = state;
    }

    pub fn set_capacity_summary(&mut self, assigned_replicas: u32, available_slots: u32) {
        self.assigned_replicas = assigned_replicas;
        self.available_slots = available_slots;
    }

    #[must_use]
    pub fn rehydrate(
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
