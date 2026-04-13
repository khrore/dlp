use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use async_trait::async_trait;
use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    Deployment, DeploymentId, Lease, LeaseId, LeaseState, Replica, ReplicaId, ReplicaState,
    Worker, WorkerId, WorkerState,
};
use tokio::sync::Mutex;

use super::{StorageBackend, UpdateReplicaStatusResult};

#[derive(Debug, Clone)]
pub(crate) struct MemoryStorage {
    inner: Arc<Mutex<MemoryState>>,
}

#[derive(Debug)]
struct MemoryWorkerRecord {
    worker:            Worker,
    assignment_queue:  VecDeque<WorkerAssignmentDto>,
    last_heartbeat_at: Instant,
}

#[derive(Debug)]
struct MemoryState {
    deployments: BTreeMap<DeploymentId, Deployment>,
    leases:      BTreeMap<LeaseId, Lease>,
    next_id:     u64,
    replicas:    BTreeMap<ReplicaId, Replica>,
    workers:     BTreeMap<WorkerId, MemoryWorkerRecord>,
}

impl MemoryStorage {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryState {
                deployments: BTreeMap::new(),
                leases:      BTreeMap::new(),
                next_id:     0,
                replicas:    BTreeMap::new(),
                workers:     BTreeMap::new(),
            })),
        }
    }

    async fn next_typed_id(&self, prefix: &str) -> Result<String> {
        let mut state = self.inner.lock().await;
        state.next_id = state.next_id.saturating_add(1);
        Ok(format!("{prefix}-{}", state.next_id))
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn next_deployment_id(&self) -> Result<String> {
        self.next_typed_id("deployment").await
    }

    async fn next_replica_id(&self) -> Result<String> {
        self.next_typed_id("replica").await
    }

    async fn next_lease_id(&self) -> Result<String> {
        self.next_typed_id("lease").await
    }

    async fn create_deployment_with_replicas(
        &self,
        deployment: Deployment,
        replicas: Vec<Replica>,
    ) -> Result<()> {
        let mut state = self.inner.lock().await;
        state
            .deployments
            .insert(deployment.id().clone(), deployment);
        for replica in replicas {
            state.replicas.insert(replica.id().clone(), replica);
        }
        Ok(())
    }

    async fn deployment(&self, deployment_id: &DeploymentId) -> Result<Option<Deployment>> {
        let state = self.inner.lock().await;
        Ok(state.deployments.get(deployment_id).cloned())
    }

    async fn deployment_ids(&self) -> Result<Vec<DeploymentId>> {
        let state = self.inner.lock().await;
        Ok(state.deployments.keys().cloned().collect())
    }

    async fn save_deployment(&self, deployment: Deployment) -> Result<()> {
        let mut state = self.inner.lock().await;
        state
            .deployments
            .insert(deployment.id().clone(), deployment);
        Ok(())
    }

    async fn replica(&self, replica_id: &ReplicaId) -> Result<Option<Replica>> {
        let state = self.inner.lock().await;
        Ok(state.replicas.get(replica_id).cloned())
    }

    async fn save_replica(&self, replica: Replica) -> Result<()> {
        let mut state = self.inner.lock().await;
        state.replicas.insert(replica.id().clone(), replica);
        Ok(())
    }

    async fn replicas_for_deployment(&self, deployment_id: &DeploymentId) -> Result<Vec<Replica>> {
        let state = self.inner.lock().await;
        Ok(state
            .replicas
            .values()
            .filter(|replica| replica.deployment_id() == deployment_id)
            .cloned()
            .collect())
    }

    async fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Result<Vec<Replica>> {
        let state = self.inner.lock().await;
        Ok(state
            .replicas
            .values()
            .filter(|replica| {
                deployment_id.is_none_or(|requested_id| replica.deployment_id() == requested_id)
            })
            .cloned()
            .collect())
    }

    async fn pending_replica_ids(&self) -> Result<Vec<ReplicaId>> {
        let state = self.inner.lock().await;
        Ok(state
            .replicas
            .values()
            .filter(|replica| replica.state() == &ReplicaState::Pending)
            .map(|replica| replica.id().clone())
            .collect())
    }

    async fn lease(&self, lease_id: &LeaseId) -> Result<Option<Lease>> {
        let state = self.inner.lock().await;
        Ok(state.leases.get(lease_id).cloned())
    }

    async fn save_lease(&self, lease: Lease) -> Result<()> {
        let mut state = self.inner.lock().await;
        state.leases.insert(lease.id().clone(), lease);
        Ok(())
    }

    async fn active_leases_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<Lease>> {
        let state = self.inner.lock().await;
        Ok(state
            .leases
            .values()
            .filter(|lease| lease.worker_id() == worker_id && lease.state() == &LeaseState::Active)
            .cloned()
            .collect())
    }

    async fn active_lease_ids_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<LeaseId>> {
        let state = self.inner.lock().await;
        Ok(state
            .leases
            .values()
            .filter(|lease| lease.worker_id() == worker_id && lease.state() == &LeaseState::Active)
            .map(|lease| lease.id().clone())
            .collect())
    }

    async fn worker(&self, worker_id: &WorkerId) -> Result<Option<Worker>> {
        let state = self.inner.lock().await;
        Ok(state
            .workers
            .get(worker_id)
            .map(|record| record.worker.clone()))
    }

    async fn worker_ids(&self) -> Result<Vec<WorkerId>> {
        let state = self.inner.lock().await;
        Ok(state.workers.keys().cloned().collect())
    }

    async fn ready_workers(&self) -> Result<Vec<Worker>> {
        let state = self.inner.lock().await;
        Ok(state
            .workers
            .values()
            .filter(|record| record.worker.state() == &WorkerState::Ready)
            .map(|record| record.worker.clone())
            .collect())
    }

    async fn register_worker(&self, worker: Worker, restart_message: &str) -> Result<()> {
        let mut state = self.inner.lock().await;
        if state.workers.contains_key(worker.id()) {
            expire_worker_in_state(&mut state, worker.id(), restart_message);
        }
        state
            .workers
            .insert(worker.id().clone(), MemoryWorkerRecord {
                worker,
                assignment_queue: VecDeque::new(),
                last_heartbeat_at: Instant::now(),
            });
        Ok(())
    }

    async fn heartbeat_worker(
        &self,
        worker_id: &WorkerId,
        worker_state: WorkerState,
    ) -> Result<Option<(Worker, Vec<WorkerAssignmentDto>)>> {
        let mut state = self.inner.lock().await;
        let record = match state.workers.get_mut(worker_id) {
            Some(record) => record,
            None => return Ok(None),
        };
        let assignments = record.assignment_queue.drain(..).collect();
        record.last_heartbeat_at = Instant::now();
        record.worker.set_state(worker_state);
        Ok(Some((record.worker.clone(), assignments)))
    }

    async fn expire_worker(&self, worker_id: &WorkerId, message: &str) -> Result<()> {
        let mut state = self.inner.lock().await;
        expire_worker_in_state(&mut state, worker_id, message);
        Ok(())
    }

    async fn assign_replica(
        &self,
        replica_id: &ReplicaId,
        worker_id: &WorkerId,
        lease: Lease,
        assignment: WorkerAssignmentDto,
    ) -> Result<bool> {
        let mut state = self.inner.lock().await;
        let Some(replica) = state.replicas.get_mut(replica_id) else {
            return Ok(false);
        };
        if replica.state() != &ReplicaState::Pending {
            return Ok(false);
        }
        if replica
            .assign(worker_id.clone(), lease.id().clone())
            .is_err()
        {
            return Ok(false);
        }
        state.leases.insert(lease.id().clone(), lease);
        if let Some(record) = state.workers.get_mut(worker_id) {
            record.assignment_queue.push_back(assignment);
        }
        Ok(true)
    }

    async fn update_replica_status(
        &self,
        replica_id: &ReplicaId,
        lease_id: &LeaseId,
        state: ReplicaState,
        status_message: Option<String>,
    ) -> Result<UpdateReplicaStatusResult> {
        let mut memory = self.inner.lock().await;
        let Some(current_replica) = memory.replicas.get(replica_id).cloned() else {
            return Ok(UpdateReplicaStatusResult::UnknownReplica);
        };
        let Some(current_lease_id) = current_replica.lease_id().cloned() else {
            return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                "replica {replica_id} is not owned by an active lease"
            )));
        };
        if &current_lease_id != lease_id {
            return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                "replica {replica_id} is owned by lease {current_lease_id}, not {lease_id}"
            )));
        }
        let Some(current_lease) = memory.leases.get(lease_id).cloned() else {
            return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                "unknown lease for replica {replica_id}: {lease_id}"
            )));
        };
        if current_lease.replica_id() != replica_id {
            return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                "lease {lease_id} does not belong to replica {replica_id}"
            )));
        }
        if current_lease.state() != &LeaseState::Active {
            return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                "lease {lease_id} is no longer active"
            )));
        }
        let Some(replica) = memory.replicas.get_mut(replica_id) else {
            return Ok(UpdateReplicaStatusResult::UnknownReplica);
        };
        if let Err(error) = replica.update_status(state, status_message) {
            return Ok(UpdateReplicaStatusResult::LeaseConflict(error.to_string()));
        }
        let updated_replica = replica.clone();
        if matches!(
            replica.state(),
            ReplicaState::Failed | ReplicaState::Stopped
        ) {
            if let Some(lease) = memory.leases.get_mut(lease_id) {
                lease.release();
            }
        }
        Ok(UpdateReplicaStatusResult::Success(updated_replica))
    }

    async fn touch_worker_capacity(
        &self,
        worker_id: &WorkerId,
        assigned_replicas: u32,
        available_slots: u32,
    ) -> Result<()> {
        let mut state = self.inner.lock().await;
        if let Some(record) = state.workers.get_mut(worker_id) {
            record
                .worker
                .set_capacity_summary(assigned_replicas, available_slots);
        }
        Ok(())
    }

    async fn lost_worker_ids(&self, lost_timeout: Duration) -> Result<Vec<WorkerId>> {
        let state = self.inner.lock().await;
        Ok(state
            .workers
            .iter()
            .filter(|(_, record)| {
                record.worker.state() != &WorkerState::Lost
                    && record.last_heartbeat_at.elapsed() >= lost_timeout
            })
            .map(|(worker_id, _)| worker_id.clone())
            .collect())
    }

    async fn force_last_heartbeat_age(
        &self,
        worker_id: &WorkerId,
        elapsed: Duration,
    ) -> Result<bool> {
        let mut state = self.inner.lock().await;
        let Some(record) = state.workers.get_mut(worker_id) else {
            return Ok(false);
        };
        let Some(last_heartbeat_at) = Instant::now().checked_sub(elapsed) else {
            return Ok(false);
        };
        record.last_heartbeat_at = last_heartbeat_at;
        Ok(true)
    }
}

fn expire_worker_in_state(state: &mut MemoryState, worker_id: &WorkerId, message: &str) {
    if let Some(record) = state.workers.get_mut(worker_id) {
        record.assignment_queue.clear();
        record.worker.set_state(WorkerState::Lost);
    }
    let lease_ids: Vec<_> = state
        .leases
        .values()
        .filter(|lease| lease.worker_id() == worker_id && lease.state() == &LeaseState::Active)
        .map(|lease| lease.id().clone())
        .collect();
    for lease_id in lease_ids {
        if let Some(lease) = state.leases.get_mut(&lease_id) {
            let replica_id = lease.replica_id().clone();
            lease.expire();
            if let Some(replica) = state.replicas.get_mut(&replica_id) {
                replica.mark_stopped(message.to_owned());
            }
        }
    }
}
