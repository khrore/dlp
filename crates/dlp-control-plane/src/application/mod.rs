use std::{result::Result as StdResult, time::Duration};

use dlp_api::{
    deployments::CreateDeploymentRequest,
    replicas::UpdateReplicaStatusRequest,
    workers::{RegisterWorkerRequest, WorkerAssignmentDto},
};
use dlp_domain::{
    Deployment, DeploymentId, Lease, LeaseId, Replica, ReplicaId, ReplicaState, Worker, WorkerId,
    WorkerState, WorkloadProfile, WorkloadRequirement, WorkloadRequirementSpec,
};

use crate::{
    ControlPlaneError, Result, SharedState,
    domain_services::{
        reconcile::DEFAULT_WORKER_LOST_TIMEOUT,
        scheduler::{available_capacity_for_requirement, worker_is_eligible},
    },
    mappers,
    repositories::UpdateReplicaStatusResult,
};

/// Internal control-plane orchestration service used by the HTTP layer and
/// tests.
#[derive(Debug, Clone)]
pub struct ControlPlaneService {
    state: SharedState,
}

/// Internal errors produced while applying replica status updates.
#[derive(Debug, thiserror::Error)]
pub enum UpdateReplicaStatusError {
    /// An internal control-plane failure occurred while applying the update.
    #[error(transparent)]
    Internal(#[from] ControlPlaneError),
    /// The provided lease information conflicts with the stored replica lease.
    #[error("{0}")]
    LeaseConflict(String),
    /// The requested replica does not exist.
    #[error("unknown replica")]
    UnknownReplica,
}

impl ControlPlaneService {
    #[must_use]
    pub(super) const fn new(state: SharedState) -> Self {
        Self { state }
    }

    pub(super) async fn create_deployment(
        &self,
        request: CreateDeploymentRequest,
    ) -> Result<Deployment> {
        let deployment_id = DeploymentId::new(self.state.0.next_deployment_id().await?)?;
        let mut deployment = Deployment::new(
            deployment_id.clone(),
            request.name,
            mappers::artifact_ref_from_string(&request.artifact_ref)?,
            request.replicas_desired,
            mappers::requirement_from_dto(&request.requirement)?,
        );
        let mut replicas = Vec::with_capacity(
            usize::try_from(deployment.replicas_desired()).unwrap_or(usize::MAX),
        );
        for _ in 0..deployment.replicas_desired() {
            let replica_id = ReplicaId::new(self.state.0.next_replica_id().await?)?;
            replicas.push(Replica::new_pending(replica_id, deployment_id.clone()));
        }
        deployment.refresh_status(&replicas);
        self.state
            .0
            .create_deployment_with_replicas(deployment.clone(), replicas)
            .await?;
        Ok(deployment)
    }

    pub(super) async fn get_deployment(
        &self,
        deployment_id: &DeploymentId,
    ) -> Result<Option<(Deployment, Vec<Replica>)>> {
        self.refresh_deployment_status(deployment_id).await?;
        let deployment_record = self.state.0.deployment(deployment_id).await?;
        let Some(deployment) = deployment_record else {
            return Ok(None);
        };
        let replicas = self.state.0.replicas_for_deployment(deployment_id).await?;
        Ok(Some((deployment, replicas)))
    }

    pub(super) async fn list_replicas(
        &self,
        deployment_id: Option<&DeploymentId>,
    ) -> Result<Vec<Replica>> {
        self.state.0.list_replicas(deployment_id).await
    }

    pub(super) async fn list_workers(&self) -> Result<Vec<Worker>> {
        self.refresh_worker_summaries().await?;
        let worker_ids = self.state.0.worker_ids().await?;
        let mut workers = Vec::with_capacity(worker_ids.len());
        for worker_id in worker_ids {
            if let Some(worker) = self.state.0.worker(&worker_id).await? {
                workers.push(worker);
            }
        }
        Ok(workers)
    }

    pub(super) async fn register_worker(&self, request: RegisterWorkerRequest) -> Result<Worker> {
        let worker_id = WorkerId::new(request.worker_id)?;
        let capabilities = request
            .capabilities
            .into_iter()
            .map(|dto| mappers::capability_from_dto(&dto))
            .collect::<dlp_domain::DomainResult<Vec<_>>>()?;
        let worker = Worker::new(worker_id.clone(), request.display_name, capabilities);
        self.state
            .0
            .register_worker(worker, "worker restarted before replica completed")
            .await?;
        self.refresh_worker_summaries().await?;
        self.state
            .0
            .worker(&worker_id)
            .await?
            .ok_or_else(|| ControlPlaneError::UnknownEntity {
                entity: "worker",
                id:     worker_id.to_string(),
            })
    }

    pub(super) async fn heartbeat_worker(
        &self,
        worker_id: &WorkerId,
        worker_state: WorkerState,
    ) -> Result<Option<(Worker, Vec<WorkerAssignmentDto>)>> {
        let result = self
            .state
            .0
            .heartbeat_worker(worker_id, worker_state)
            .await?;
        if result.is_none() {
            Ok(None)
        } else {
            self.reconcile_once().await?;
            let assignments = self
                .state
                .0
                .take_worker_assignments(worker_id)
                .await?
                .unwrap_or_default();
            let worker_record = self.state.0.worker(worker_id).await?;
            Ok(worker_record.map(|worker| (worker, assignments)))
        }
    }

    pub(super) async fn update_replica_status(
        &self,
        replica_id: &ReplicaId,
        request: UpdateReplicaStatusRequest,
    ) -> StdResult<Replica, UpdateReplicaStatusError> {
        let request_lease_id = LeaseId::new(request.lease_id.clone())
            .map_err(|error| UpdateReplicaStatusError::LeaseConflict(error.to_string()))?;
        match self
            .state
            .0
            .update_replica_status(
                replica_id,
                &request_lease_id,
                mappers::replica_state_from_dto(&request.state),
                request.status_message,
            )
            .await?
        {
            UpdateReplicaStatusResult::Success(replica) => Ok(replica),
            UpdateReplicaStatusResult::UnknownReplica => {
                Err(UpdateReplicaStatusError::UnknownReplica)
            }
            UpdateReplicaStatusResult::LeaseConflict(message) => {
                Err(UpdateReplicaStatusError::LeaseConflict(message))
            }
        }
    }

    pub(super) async fn reconcile_once(&self) -> Result<()> {
        let lost_workers = self
            .state
            .0
            .lost_worker_ids(DEFAULT_WORKER_LOST_TIMEOUT)
            .await?;
        for worker_id in lost_workers {
            self.state
                .0
                .expire_worker(&worker_id, "worker lost before replica completed")
                .await?;
        }

        let deployment_ids = self.state.0.deployment_ids().await?;
        for deployment_id in &deployment_ids {
            self.ensure_deployment_capacity(deployment_id).await?;
            self.refresh_deployment_status(deployment_id).await?;
        }

        let pending_replica_ids = self.state.0.pending_replica_ids().await?;
        for replica_id in &pending_replica_ids {
            self.try_assign_replica(replica_id).await?;
        }

        self.refresh_worker_summaries().await?;
        let deployment_ids = self.state.0.deployment_ids().await?;
        for deployment_id in &deployment_ids {
            self.refresh_deployment_status(deployment_id).await?;
        }
        Ok(())
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Only integration tests drive this helper today.")
    )]
    pub(super) async fn force_last_heartbeat_age(
        &self,
        worker_id: &WorkerId,
        elapsed: Duration,
    ) -> Result<bool> {
        self.state
            .0
            .force_last_heartbeat_age(worker_id, elapsed)
            .await
    }

    async fn ensure_deployment_capacity(&self, deployment_id: &DeploymentId) -> Result<()> {
        let Some(mut deployment) = self.state.0.deployment(deployment_id).await? else {
            return Ok(());
        };
        let replicas = self.state.0.replicas_for_deployment(deployment_id).await?;
        let active_replicas_count = replicas
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
        if active_replicas >= deployment.replicas_desired() {
            return Ok(());
        }
        let mut current_replicas = replicas;
        for _ in active_replicas..deployment.replicas_desired() {
            let replica_id = ReplicaId::new(self.state.0.next_replica_id().await?)?;
            let replica = Replica::new_pending(replica_id, deployment_id.clone());
            self.state.0.save_replica(replica.clone()).await?;
            current_replicas.push(replica);
        }
        deployment.refresh_status(&current_replicas);
        self.state.0.save_deployment(deployment).await
    }

    async fn refresh_deployment_status(&self, deployment_id: &DeploymentId) -> Result<()> {
        let Some(mut deployment) = self.state.0.deployment(deployment_id).await? else {
            return Ok(());
        };
        let replicas = self.state.0.replicas_for_deployment(deployment_id).await?;
        deployment.refresh_status(&replicas);
        self.state.0.save_deployment(deployment).await
    }

    async fn refresh_worker_summaries(&self) -> Result<()> {
        let worker_ids = self.state.0.worker_ids().await?;
        for worker_id in worker_ids {
            let active_leases = self.state.0.active_leases_for_worker(&worker_id).await?;
            if let Some(worker) = self.state.0.worker(&worker_id).await? {
                let available_slots = worker
                    .capabilities()
                    .iter()
                    .filter_map(|capability| {
                        available_capacity_for_requirement(
                            &worker,
                            &WorkloadRequirement::new(WorkloadRequirementSpec::new(
                                WorkloadProfile::new(
                                    capability.framework().clone(),
                                    capability.mode().clone(),
                                    capability.device().clone(),
                                    capability.accelerator_runtime().clone(),
                                    capability.architecture_family().clone(),
                                ),
                                0,
                                0,
                            )),
                            &active_leases,
                        )
                        .map(|(slots, _)| slots)
                    })
                    .fold(0, u32::saturating_add);
                self.state
                    .0
                    .touch_worker_capacity(
                        &worker_id,
                        u32::try_from(active_leases.len()).unwrap_or(u32::MAX),
                        available_slots,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn select_worker(&self, requirement: &WorkloadRequirement) -> Result<Option<WorkerId>> {
        for worker in self.state.0.ready_workers().await? {
            let leases = self.state.0.active_leases_for_worker(worker.id()).await?;
            if worker_is_eligible(&worker, requirement, &leases) {
                return Ok(Some(worker.id().clone()));
            }
        }
        Ok(None)
    }

    async fn try_assign_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        let Some(replica) = self.state.0.replica(replica_id).await? else {
            return Ok(());
        };
        let Some(deployment) = self.state.0.deployment(replica.deployment_id()).await? else {
            return Ok(());
        };
        let Some(worker_id) = self.select_worker(deployment.requirement()).await? else {
            return Ok(());
        };
        let lease_id = LeaseId::new(self.state.0.next_lease_id().await?)?;
        let lease = Lease::new(
            lease_id.clone(),
            worker_id.clone(),
            deployment.id().clone(),
            replica.id().clone(),
            deployment.requirement().clone(),
        );
        let assignment = mappers::assignment_to_dto(&deployment, &lease, &replica);
        let assigned = self
            .state
            .0
            .assign_replica(replica_id, &worker_id, lease, assignment)
            .await?;
        if assigned {
            self.refresh_deployment_status(deployment.id()).await?;
            self.refresh_worker_summaries().await?;
        }
        Ok(())
    }
}
