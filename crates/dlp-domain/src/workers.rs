//! Worker lifecycle state.

use std::{collections::VecDeque, time::Instant};

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
}

#[derive(Debug, Clone)]
pub struct WorkerRecord<Assignment> {
    worker:            Worker,
    assignment_queue:  VecDeque<Assignment>,
    last_heartbeat_at: Instant,
}

impl<Assignment> WorkerRecord<Assignment> {
    #[must_use]
    pub fn new(worker: Worker) -> Self {
        Self {
            worker,
            assignment_queue: VecDeque::new(),
            last_heartbeat_at: Instant::now(),
        }
    }

    #[must_use]
    pub fn last_heartbeat_at(&self) -> Instant {
        self.last_heartbeat_at
    }

    pub fn set_last_heartbeat_at(&mut self, instant: Instant) {
        self.last_heartbeat_at = instant;
    }

    #[must_use]
    pub fn worker(&self) -> &Worker {
        &self.worker
    }

    #[must_use]
    pub fn worker_mut(&mut self) -> &mut Worker {
        &mut self.worker
    }

    #[must_use]
    pub fn drain_assignments(&mut self) -> Vec<Assignment> {
        self.assignment_queue.drain(..).collect()
    }

    pub fn push_assignment(&mut self, assignment: Assignment) {
        self.assignment_queue.push_back(assignment);
    }

    pub fn clear_assignments(&mut self) {
        self.assignment_queue.clear();
    }
}
