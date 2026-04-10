use std::{sync::Arc, time::Duration};

use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    Deployment, DeploymentId, DomainError, Lease, LeaseId, LeaseState, Replica, ReplicaId,
    ReplicaState, Worker, WorkerId, WorkerState,
};
use tokio::sync::Mutex;

use crate::{
    domain_services::scheduler::{available_capacity_for_requirement, worker_is_eligible},
    mappers,
    repositories::{
        DeploymentRepository, LeaseRepository, ReplicaRepository, WorkerRepository,
        memory::MemoryStore,
    },
};

#[derive(Debug, Clone)]
pub struct SharedState(Arc<Mutex<MemoryStore>>);

#[derive(Debug, Clone)]
pub struct ControlPlaneService {
    state: SharedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateReplicaStatusError {
    LeaseConflict(String),
    UnknownReplica,
}

impl SharedState {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(MemoryStore::new())))
    }
}

impl ControlPlaneService {
    #[must_use]
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }

    pub async fn create_deployment(
        &self,
        request: dlp_api::CreateDeploymentRequest,
    ) -> Result<Deployment, DomainError> {
        let mut store = self.state.0.lock().await;
        let deployment_id = DeploymentId::new(store.next_id("deployment"))?;
        let deployment = Deployment::new(
            deployment_id.clone(),
            request.name,
            mappers::artifact_ref_from_string(request.artifact_ref)?,
            request.replicas_desired,
            mappers::requirement_from_dto(request.requirement)?,
        );

        store.insert_deployment(deployment.clone());
        self.ensure_deployment_capacity_with_store(&mut store, &deployment_id)?;
        self.refresh_deployment_status_with_store(&mut store, &deployment_id);

        store.deployment(&deployment_id).ok_or_else(|| {
            DomainError::LeaseConflict(format!("deployment disappeared: {deployment_id}"))
        })
    }

    pub async fn get_deployment(
        &self,
        deployment_id: &DeploymentId,
    ) -> Option<(Deployment, Vec<Replica>)> {
        let mut store = self.state.0.lock().await;
        self.refresh_deployment_status_with_store(&mut store, deployment_id);
        let deployment = store.deployment(deployment_id)?;
        let replicas = store.replicas_for_deployment(deployment_id);
        Some((deployment, replicas))
    }

    pub async fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Vec<Replica> {
        let store = self.state.0.lock().await;
        store.list_replicas(deployment_id)
    }

    pub async fn list_workers(&self) -> Vec<Worker> {
        let mut store = self.state.0.lock().await;
        self.refresh_worker_summaries_with_store(&mut store);
        store
            .worker_ids()
            .into_iter()
            .filter_map(|worker_id| store.worker(&worker_id))
            .collect()
    }

    pub async fn register_worker(
        &self,
        request: dlp_api::RegisterWorkerRequest,
    ) -> Result<Worker, DomainError> {
        let mut store = self.state.0.lock().await;
        let worker_id = WorkerId::new(request.worker_id)?;
        if store.worker(&worker_id).is_some() {
            self.expire_worker_with_store(
                &mut store,
                &worker_id,
                "worker restarted before replica completed",
            );
        }

        let capabilities = request
            .capabilities
            .into_iter()
            .map(mappers::capability_from_dto)
            .collect::<Result<Vec<_>, _>>()?;
        let worker = Worker::new(worker_id.clone(), request.display_name, capabilities);
        store.insert_worker(worker);
        self.refresh_worker_summaries_with_store(&mut store);

        store
            .worker(&worker_id)
            .ok_or_else(|| DomainError::LeaseConflict(format!("worker disappeared: {worker_id}")))
    }

    pub async fn heartbeat_worker(
        &self,
        worker_id: &WorkerId,
        worker_state: WorkerState,
    ) -> Option<(Worker, Vec<WorkerAssignmentDto>)> {
        let mut store = self.state.0.lock().await;
        let assignments = store.drain_assignments(worker_id)?;
        store.touch_heartbeat(worker_id);
        store.set_worker_state(worker_id, worker_state)?;
        self.refresh_worker_summaries_with_store(&mut store);
        let worker = store.worker(worker_id)?;
        Some((worker, assignments))
    }

    pub async fn update_replica_status(
        &self,
        replica_id: &ReplicaId,
        request: dlp_api::UpdateReplicaStatusRequest,
    ) -> Result<Replica, UpdateReplicaStatusError> {
        let mut store = self.state.0.lock().await;
        let mut replica = store
            .replica(replica_id)
            .ok_or(UpdateReplicaStatusError::UnknownReplica)?;
        let current_lease_id = replica.lease_id().cloned().ok_or_else(|| {
            UpdateReplicaStatusError::LeaseConflict(format!(
                "replica {replica_id} is not owned by an active lease"
            ))
        })?;
        let request_lease_id = LeaseId::new(request.lease_id.clone())
            .map_err(|error| UpdateReplicaStatusError::LeaseConflict(error.to_string()))?;

        if current_lease_id != request_lease_id {
            return Err(UpdateReplicaStatusError::LeaseConflict(format!(
                "replica {replica_id} is owned by lease {current_lease_id}, not {}",
                request.lease_id
            )));
        }

        let mut lease = store.lease(&current_lease_id).ok_or_else(|| {
            UpdateReplicaStatusError::LeaseConflict(format!(
                "unknown lease for replica {replica_id}: {current_lease_id}"
            ))
        })?;
        if lease.replica_id() != replica_id {
            return Err(UpdateReplicaStatusError::LeaseConflict(format!(
                "lease {current_lease_id} does not belong to replica {replica_id}"
            )));
        }
        if lease.state() != &LeaseState::Active {
            return Err(UpdateReplicaStatusError::LeaseConflict(format!(
                "lease {current_lease_id} is no longer active"
            )));
        }

        replica
            .update_status(
                mappers::replica_state_from_dto(request.state),
                request.status_message,
            )
            .map_err(|error| UpdateReplicaStatusError::LeaseConflict(error.to_string()))?;
        if matches!(
            replica.state(),
            ReplicaState::Failed | ReplicaState::Stopped
        ) {
            lease.release();
            store.save_lease(lease);
        }
        let deployment_id = replica.deployment_id().clone();
        store.save_replica(replica.clone());
        self.refresh_deployment_status_with_store(&mut store, &deployment_id);
        Ok(replica)
    }

    pub async fn reconcile_once(&self) {
        let mut store = self.state.0.lock().await;
        let lost_workers =
            store.lost_worker_ids(crate::domain_services::reconcile::DEFAULT_WORKER_LOST_TIMEOUT);
        for worker_id in lost_workers {
            self.expire_worker_with_store(
                &mut store,
                &worker_id,
                "worker lost before replica completed",
            );
        }

        let deployment_ids = store.deployment_ids();
        for deployment_id in &deployment_ids {
            let _ = self.ensure_deployment_capacity_with_store(&mut store, deployment_id);
            self.refresh_deployment_status_with_store(&mut store, deployment_id);
        }

        let pending_replica_ids = store.pending_replica_ids();
        for replica_id in &pending_replica_ids {
            self.try_assign_replica_with_store(&mut store, replica_id);
        }

        self.refresh_worker_summaries_with_store(&mut store);
        let deployment_ids = store.deployment_ids();
        for deployment_id in &deployment_ids {
            self.refresh_deployment_status_with_store(&mut store, deployment_id);
        }
    }

    pub async fn force_last_heartbeat_age(&self, worker_id: &WorkerId, elapsed: Duration) -> bool {
        let mut store = self.state.0.lock().await;
        store.force_last_heartbeat_age(worker_id, elapsed)
    }

    fn ensure_deployment_capacity_with_store(
        &self,
        store: &mut MemoryStore,
        deployment_id: &DeploymentId,
    ) -> Result<(), DomainError> {
        let Some(deployment) = store.deployment(deployment_id) else {
            return Ok(());
        };

        let active_replicas_count = store
            .replicas_for_deployment(deployment_id)
            .iter()
            .filter(|replica| {
                matches!(
                    replica.state(),
                    ReplicaState::Pending
                        | ReplicaState::Assigned
                        | ReplicaState::Pulling
                        | ReplicaState::Starting
                        | ReplicaState::Ready
                )
            })
            .count();
        let active_replicas = u32::try_from(active_replicas_count).unwrap_or(u32::MAX);

        for _ in active_replicas..deployment.replicas_desired() {
            let replica_id = ReplicaId::new(store.next_id("replica"))?;
            store.insert_replica(Replica::new_pending(replica_id, deployment_id.clone()));
        }

        Ok(())
    }

    fn refresh_deployment_status_with_store(
        &self,
        store: &mut MemoryStore,
        deployment_id: &DeploymentId,
    ) {
        let Some(mut deployment) = store.deployment(deployment_id) else {
            return;
        };
        let replicas = store.replicas_for_deployment(deployment_id);
        deployment.refresh_status(replicas.iter());
        store.save_deployment(deployment);
    }

    fn refresh_worker_summaries_with_store(&self, store: &mut MemoryStore) {
        let worker_ids = store.worker_ids();
        for worker_id in worker_ids {
            let active_leases = store.active_leases_for_worker(&worker_id);
            if let Some(mut worker) = store.worker(&worker_id) {
                let available_slots = worker
                    .capabilities()
                    .iter()
                    .filter_map(|capability| {
                        available_capacity_for_requirement(
                            &worker,
                            &dlp_domain::WorkloadRequirement::new(
                                capability.framework().clone(),
                                capability.mode().clone(),
                                capability.device().clone(),
                                capability.accelerator_runtime().clone(),
                                capability.architecture_family().clone(),
                                0,
                                0,
                            ),
                            &active_leases,
                        )
                        .map(|(slots, _)| slots)
                    })
                    .fold(0, u32::saturating_add);

                worker.set_capacity_summary(
                    u32::try_from(active_leases.len()).unwrap_or(u32::MAX),
                    available_slots,
                );
                store.save_worker(worker);
            }
        }
    }

    fn select_worker_with_store(
        &self,
        store: &MemoryStore,
        requirement: &dlp_domain::WorkloadRequirement,
    ) -> Option<WorkerId> {
        store.ready_workers().into_iter().find_map(|worker| {
            let leases = store.active_leases_for_worker(worker.id());
            worker_is_eligible(&worker, requirement, &leases).then(|| worker.id().clone())
        })
    }

    fn try_assign_replica_with_store(&self, store: &mut MemoryStore, replica_id: &ReplicaId) {
        let Some(mut replica) = store.replica(replica_id) else {
            return;
        };
        let Some(deployment) = store.deployment(replica.deployment_id()) else {
            return;
        };
        let Some(worker_id) = self.select_worker_with_store(store, deployment.requirement()) else {
            return;
        };
        let Ok(lease_id) = LeaseId::new(store.next_id("lease")) else {
            return;
        };
        let lease = Lease::new(
            lease_id.clone(),
            worker_id.clone(),
            deployment.id().clone(),
            replica.id().clone(),
            deployment.requirement().clone(),
        );
        if replica.assign(worker_id.clone(), lease_id).is_err() {
            return;
        }
        let assignment = mappers::assignment_to_dto(&deployment, &lease, &replica);
        store.insert_lease(lease);
        store.save_replica(replica);
        store.enqueue_assignment(&worker_id, assignment);
    }

    fn expire_worker_with_store(
        &self,
        store: &mut MemoryStore,
        worker_id: &WorkerId,
        message: &str,
    ) {
        store.clear_assignments(worker_id);
        if let Some(mut worker) = store.worker(worker_id) {
            worker.set_state(WorkerState::Lost);
            store.save_worker(worker);
        }
        let lease_ids = store.active_lease_ids_for_worker(worker_id);
        for lease_id in lease_ids {
            if let Some(mut lease) = store.lease(&lease_id) {
                let deployment_id = lease.deployment_id().clone();
                let replica_id = lease.replica_id().clone();
                lease.expire();
                store.save_lease(lease);
                if let Some(mut replica) = store.replica(&replica_id) {
                    replica.mark_stopped(message.to_owned());
                    store.save_replica(replica);
                }
                self.refresh_deployment_status_with_store(store, &deployment_id);
            }
        }
    }
}
