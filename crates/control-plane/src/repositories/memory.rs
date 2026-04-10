use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    Deployment, DeploymentId, Lease, LeaseId, LeaseState, Replica, ReplicaId, Worker, WorkerId,
    WorkerState, WorkloadRequirement, workers::WorkerRecord,
};

use super::{DeploymentRepository, LeaseRepository, ReplicaRepository, WorkerRepository};

#[derive(Debug)]
pub struct MemoryStore {
    deployments: BTreeMap<DeploymentId, Deployment>,
    leases:      BTreeMap<LeaseId, Lease>,
    next_id:     u64,
    replicas:    BTreeMap<ReplicaId, Replica>,
    workers:     BTreeMap<WorkerId, WorkerRecord<WorkerAssignmentDto>>,
}

impl MemoryStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            deployments: BTreeMap::new(),
            leases:      BTreeMap::new(),
            next_id:     0,
            replicas:    BTreeMap::new(),
            workers:     BTreeMap::new(),
        }
    }

    pub fn next_id(&mut self, prefix: &str) -> String {
        self.next_id = self.next_id.saturating_add(1);
        format!("{prefix}-{}", self.next_id)
    }
}

impl DeploymentRepository for MemoryStore {
    fn insert_deployment(&mut self, deployment: Deployment) {
        self.deployments.insert(deployment.id().clone(), deployment);
    }

    fn deployment(&self, deployment_id: &DeploymentId) -> Option<Deployment> {
        self.deployments.get(deployment_id).cloned()
    }

    fn deployment_ids(&self) -> Vec<DeploymentId> {
        self.deployments.keys().cloned().collect()
    }

    fn save_deployment(&mut self, deployment: Deployment) {
        self.deployments.insert(deployment.id().clone(), deployment);
    }
}

impl ReplicaRepository for MemoryStore {
    fn insert_replica(&mut self, replica: Replica) {
        self.replicas.insert(replica.id().clone(), replica);
    }

    fn replica(&self, replica_id: &ReplicaId) -> Option<Replica> {
        self.replicas.get(replica_id).cloned()
    }

    fn save_replica(&mut self, replica: Replica) {
        self.replicas.insert(replica.id().clone(), replica);
    }

    fn replicas_for_deployment(&self, deployment_id: &DeploymentId) -> Vec<Replica> {
        self.replicas
            .values()
            .filter(|replica| replica.deployment_id() == deployment_id)
            .cloned()
            .collect()
    }

    fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Vec<Replica> {
        self.replicas
            .values()
            .filter(|replica| {
                deployment_id.is_none_or(|requested_id| replica.deployment_id() == requested_id)
            })
            .cloned()
            .collect()
    }

    fn pending_replica_ids(&self) -> Vec<ReplicaId> {
        self.replicas
            .values()
            .filter(|replica| replica.state() == &dlp_domain::ReplicaState::Pending)
            .map(|replica| replica.id().clone())
            .collect()
    }
}

impl LeaseRepository for MemoryStore {
    fn insert_lease(&mut self, lease: Lease) {
        self.leases.insert(lease.id().clone(), lease);
    }

    fn lease(&self, lease_id: &LeaseId) -> Option<Lease> {
        self.leases.get(lease_id).cloned()
    }

    fn save_lease(&mut self, lease: Lease) {
        self.leases.insert(lease.id().clone(), lease);
    }

    fn active_leases_for_worker(&self, worker_id: &WorkerId) -> Vec<Lease> {
        self.leases
            .values()
            .filter(|lease| lease.worker_id() == worker_id && lease.state() == &LeaseState::Active)
            .cloned()
            .collect()
    }

    fn active_leases_for_requirement(
        &self,
        worker_id: &WorkerId,
        requirement: &WorkloadRequirement,
    ) -> Vec<Lease> {
        self.leases
            .values()
            .filter(|lease| {
                lease.worker_id() == worker_id
                    && lease.state() == &LeaseState::Active
                    && lease.requirement() == requirement
            })
            .cloned()
            .collect()
    }

    fn active_lease_ids_for_worker(&self, worker_id: &WorkerId) -> Vec<LeaseId> {
        self.leases
            .values()
            .filter(|lease| lease.worker_id() == worker_id && lease.state() == &LeaseState::Active)
            .map(|lease| lease.id().clone())
            .collect()
    }
}

impl WorkerRepository for MemoryStore {
    fn insert_worker(&mut self, worker: Worker) {
        self.workers
            .insert(worker.id().clone(), WorkerRecord::new(worker));
    }

    fn worker(&self, worker_id: &WorkerId) -> Option<Worker> {
        self.workers
            .get(worker_id)
            .map(|record| record.worker().clone())
    }

    fn save_worker(&mut self, worker: Worker) {
        if let Some(record) = self.workers.get_mut(worker.id()) {
            *record.worker_mut() = worker;
        } else {
            self.insert_worker(worker);
        }
    }

    fn worker_ids(&self) -> Vec<WorkerId> {
        self.workers.keys().cloned().collect()
    }

    fn ready_workers(&self) -> Vec<Worker> {
        self.workers
            .values()
            .filter(|record| record.worker().state() == &WorkerState::Ready)
            .map(|record| record.worker().clone())
            .collect()
    }

    fn drain_assignments(&mut self, worker_id: &WorkerId) -> Option<Vec<WorkerAssignmentDto>> {
        self.workers
            .get_mut(worker_id)
            .map(WorkerRecord::drain_assignments)
    }

    fn enqueue_assignment(&mut self, worker_id: &WorkerId, assignment: WorkerAssignmentDto) {
        if let Some(record) = self.workers.get_mut(worker_id) {
            record.push_assignment(assignment);
        }
    }

    fn clear_assignments(&mut self, worker_id: &WorkerId) {
        if let Some(record) = self.workers.get_mut(worker_id) {
            record.clear_assignments();
        }
    }

    fn set_worker_state(&mut self, worker_id: &WorkerId, state: WorkerState) -> Option<Worker> {
        let record = self.workers.get_mut(worker_id)?;
        record.worker_mut().set_state(state);
        Some(record.worker().clone())
    }

    fn touch_heartbeat(&mut self, worker_id: &WorkerId) {
        if let Some(record) = self.workers.get_mut(worker_id) {
            record.set_last_heartbeat_at(Instant::now());
        }
    }

    fn force_last_heartbeat_age(&mut self, worker_id: &WorkerId, elapsed: Duration) -> bool {
        if let Some(record) = self.workers.get_mut(worker_id) {
            let Some(last_heartbeat_at) = Instant::now().checked_sub(elapsed) else {
                return false;
            };
            record.set_last_heartbeat_at(last_heartbeat_at);
            return true;
        }

        false
    }

    fn lost_worker_ids(&self, lost_timeout: Duration) -> Vec<WorkerId> {
        self.workers
            .iter()
            .filter(|(_, record)| {
                record.worker().state() != &WorkerState::Lost
                    && record.last_heartbeat_at().elapsed() >= lost_timeout
            })
            .map(|(worker_id, _)| worker_id.clone())
            .collect()
    }
}
