pub mod memory;

use std::time::Duration;

use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    Deployment, DeploymentId, Lease, LeaseId, Replica, ReplicaId, Worker, WorkerId, WorkerState,
    WorkloadRequirement,
};

pub trait DeploymentRepository {
    fn insert_deployment(&mut self, deployment: Deployment);
    fn deployment(&self, deployment_id: &DeploymentId) -> Option<Deployment>;
    fn deployment_ids(&self) -> Vec<DeploymentId>;
    fn save_deployment(&mut self, deployment: Deployment);
}

pub trait ReplicaRepository {
    fn insert_replica(&mut self, replica: Replica);
    fn replica(&self, replica_id: &ReplicaId) -> Option<Replica>;
    fn save_replica(&mut self, replica: Replica);
    fn replicas_for_deployment(&self, deployment_id: &DeploymentId) -> Vec<Replica>;
    fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Vec<Replica>;
    fn pending_replica_ids(&self) -> Vec<ReplicaId>;
}

pub trait LeaseRepository {
    fn insert_lease(&mut self, lease: Lease);
    fn lease(&self, lease_id: &LeaseId) -> Option<Lease>;
    fn save_lease(&mut self, lease: Lease);
    fn active_leases_for_worker(&self, worker_id: &WorkerId) -> Vec<Lease>;
    fn active_leases_for_requirement(
        &self,
        worker_id: &WorkerId,
        requirement: &WorkloadRequirement,
    ) -> Vec<Lease>;
    fn active_lease_ids_for_worker(&self, worker_id: &WorkerId) -> Vec<LeaseId>;
}

pub trait WorkerRepository {
    fn insert_worker(&mut self, worker: Worker);
    fn worker(&self, worker_id: &WorkerId) -> Option<Worker>;
    fn save_worker(&mut self, worker: Worker);
    fn worker_ids(&self) -> Vec<WorkerId>;
    fn ready_workers(&self) -> Vec<Worker>;
    fn drain_assignments(&mut self, worker_id: &WorkerId) -> Option<Vec<WorkerAssignmentDto>>;
    fn enqueue_assignment(&mut self, worker_id: &WorkerId, assignment: WorkerAssignmentDto);
    fn clear_assignments(&mut self, worker_id: &WorkerId);
    fn set_worker_state(&mut self, worker_id: &WorkerId, state: WorkerState) -> Option<Worker>;
    fn touch_heartbeat(&mut self, worker_id: &WorkerId);
    fn force_last_heartbeat_age(&mut self, worker_id: &WorkerId, elapsed: Duration) -> bool;
    fn lost_worker_ids(&self, lost_timeout: Duration) -> Vec<WorkerId>;
}
