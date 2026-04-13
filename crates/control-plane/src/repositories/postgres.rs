use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    ArchitectureFamily, ArtifactRef, Deployment, DeploymentId, DeploymentStatusSummary,
    DeviceClass, DomainResult, Framework, Lease, LeaseId, LeaseState, Replica, ReplicaId,
    ReplicaState, RuntimeName, Worker, WorkerCapability, WorkerId, WorkerState, WorkloadMode,
    WorkloadRequirement,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::Value;

use super::{StorageBackend, UpdateReplicaStatusResult};
use crate::domain_services::scheduler::available_capacity_for_requirement;

mod ids {
    use anyhow::{Result, anyhow};
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    const DEPLOYMENT_SEQUENCE: &str = "deployment_id_seq";
    const REPLICA_SEQUENCE: &str = "replica_id_seq";
    const LEASE_SEQUENCE: &str = "lease_id_seq";

    pub(super) async fn next_deployment_id<C>(connection: &C) -> Result<String>
    where
        C: ConnectionTrait,
    {
        next_prefixed_id(connection, "deployment", DEPLOYMENT_SEQUENCE).await
    }

    pub(super) async fn next_replica_id<C>(connection: &C) -> Result<String>
    where
        C: ConnectionTrait,
    {
        next_prefixed_id(connection, "replica", REPLICA_SEQUENCE).await
    }

    pub(super) async fn next_lease_id<C>(connection: &C) -> Result<String>
    where
        C: ConnectionTrait,
    {
        next_prefixed_id(connection, "lease", LEASE_SEQUENCE).await
    }

    async fn next_prefixed_id<C>(connection: &C, prefix: &str, sequence: &str) -> Result<String>
    where
        C: ConnectionTrait,
    {
        // PostgreSQL sequence advancement is a backend-native primitive that SeaORM
        // does not model directly, so the adapter keeps this single raw SQL
        // call behind a typed helper.
        let statement = Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT nextval('{sequence}')::bigint AS value"),
        );
        let row = connection
            .query_one(statement)
            .await?
            .ok_or_else(|| anyhow!("sequence {sequence} returned no row"))?;
        let value: i64 = row.try_get("", "value")?;
        Ok(format!("{prefix}-{value}"))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresStorage {
    db: DatabaseConnection,
}

impl PostgresStorage {
    #[must_use]
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[must_use]
    pub(crate) fn connection(&self) -> &DatabaseConnection {
        &self.db
    }
}

#[async_trait]
impl StorageBackend for PostgresStorage {
    async fn next_deployment_id(&self) -> Result<String> {
        ids::next_deployment_id(&self.db).await
    }

    async fn next_replica_id(&self) -> Result<String> {
        ids::next_replica_id(&self.db).await
    }

    async fn next_lease_id(&self) -> Result<String> {
        ids::next_lease_id(&self.db).await
    }

    async fn create_deployment_with_replicas(
        &self,
        deployment: Deployment,
        replicas: Vec<Replica>,
    ) -> Result<()> {
        self.db
            .transaction::<_, (), anyhow::Error>(|txn| {
                Box::pin(async move {
                    let now = Utc::now();
                    deployment::ActiveModel::from_domain(&deployment, now)?
                        .insert(txn)
                        .await?;
                    for replica in &replicas {
                        replica_entity::ActiveModel::from_domain(replica, now)
                            .insert(txn)
                            .await?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(Into::into)
    }

    async fn deployment(&self, deployment_id: &DeploymentId) -> Result<Option<Deployment>> {
        let model = deployment::Entity::find_by_id(deployment_id.to_string())
            .one(&self.db)
            .await?;
        Ok(model.map(deployment::Model::into_domain).transpose()?)
    }

    async fn deployment_ids(&self) -> Result<Vec<DeploymentId>> {
        deployment::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|model| DeploymentId::new(model.id))
            .collect::<DomainResult<Vec<_>>>()
            .map_err(Into::into)
    }

    async fn save_deployment(&self, deployment: Deployment) -> Result<()> {
        let existing = deployment::Entity::find_by_id(deployment.id().to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("unknown deployment: {}", deployment.id()))?;
        let mut active = existing.into_active_model();
        let now = Utc::now();
        active.name = Set(deployment.name().to_owned());
        active.artifact_ref = Set(deployment.artifact_ref().to_string());
        active.replicas_desired = Set(to_i32(deployment.replicas_desired())?);
        set_requirement_on_deployment(&mut active, deployment.requirement())?;
        set_status_on_deployment(&mut active, deployment.status())?;
        active.updated_at = Set(now);
        active.update(&self.db).await?;
        Ok(())
    }

    async fn replica(&self, replica_id: &ReplicaId) -> Result<Option<Replica>> {
        let model = replica_entity::Entity::find_by_id(replica_id.to_string())
            .one(&self.db)
            .await?;
        Ok(model.map(replica_entity::Model::into_domain).transpose()?)
    }

    async fn save_replica(&self, replica: Replica) -> Result<()> {
        let existing = replica_entity::Entity::find_by_id(replica.id().to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("unknown replica: {}", replica.id()))?;
        let mut active = existing.into_active_model();
        active.state = Set(replica.state().to_string());
        active.assigned_worker_id = Set(replica.worker_id().map(ToString::to_string));
        active.lease_id = Set(replica.lease_id().map(ToString::to_string));
        active.status_message = Set(replica.status_message().map(str::to_owned));
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await?;
        Ok(())
    }

    async fn replicas_for_deployment(&self, deployment_id: &DeploymentId) -> Result<Vec<Replica>> {
        replica_entity::Entity::find()
            .filter(replica_entity::Column::DeploymentId.eq(deployment_id.to_string()))
            .all(&self.db)
            .await?
            .into_iter()
            .map(replica_entity::Model::into_domain)
            .collect()
    }

    async fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Result<Vec<Replica>> {
        let mut query = replica_entity::Entity::find();
        if let Some(deployment_id) = deployment_id {
            query =
                query.filter(replica_entity::Column::DeploymentId.eq(deployment_id.to_string()));
        }
        query
            .all(&self.db)
            .await?
            .into_iter()
            .map(replica_entity::Model::into_domain)
            .collect()
    }

    async fn pending_replica_ids(&self) -> Result<Vec<ReplicaId>> {
        replica_entity::Entity::find()
            .filter(replica_entity::Column::State.eq(ReplicaState::Pending.to_string()))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|model| ReplicaId::new(model.id))
            .collect::<DomainResult<Vec<_>>>()
            .map_err(Into::into)
    }

    async fn lease(&self, lease_id: &LeaseId) -> Result<Option<Lease>> {
        let model = lease_entity::Entity::find_by_id(lease_id.to_string())
            .one(&self.db)
            .await?;
        Ok(model.map(lease_entity::Model::into_domain).transpose()?)
    }

    async fn save_lease(&self, lease: Lease) -> Result<()> {
        let existing = lease_entity::Entity::find_by_id(lease.id().to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("unknown lease: {}", lease.id()))?;
        let mut active = existing.into_active_model();
        active.state = Set(lease.state().to_string());
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await?;
        Ok(())
    }

    async fn active_leases_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<Lease>> {
        lease_entity::Entity::find()
            .filter(lease_entity::Column::WorkerId.eq(worker_id.to_string()))
            .filter(lease_entity::Column::State.eq(LeaseState::Active.to_string()))
            .all(&self.db)
            .await?
            .into_iter()
            .map(lease_entity::Model::into_domain)
            .collect()
    }

    async fn active_lease_ids_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<LeaseId>> {
        lease_entity::Entity::find()
            .filter(lease_entity::Column::WorkerId.eq(worker_id.to_string()))
            .filter(lease_entity::Column::State.eq(LeaseState::Active.to_string()))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|model| LeaseId::new(model.id))
            .collect::<DomainResult<Vec<_>>>()
            .map_err(Into::into)
    }

    async fn worker(&self, worker_id: &WorkerId) -> Result<Option<Worker>> {
        let worker = worker_entity::Entity::find_by_id(worker_id.to_string())
            .one(&self.db)
            .await?;
        let Some(worker) = worker else {
            return Ok(None);
        };
        let capabilities = load_worker_capabilities(&self.db, &worker.id).await?;
        Ok(Some(worker.into_domain(capabilities)?))
    }

    async fn worker_ids(&self) -> Result<Vec<WorkerId>> {
        worker_entity::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|model| WorkerId::new(model.id))
            .collect::<DomainResult<Vec<_>>>()
            .map_err(Into::into)
    }

    async fn ready_workers(&self) -> Result<Vec<Worker>> {
        let workers = worker_entity::Entity::find()
            .filter(worker_entity::Column::State.eq(WorkerState::Ready.to_string()))
            .all(&self.db)
            .await?;
        let mut output = Vec::with_capacity(workers.len());
        for worker in workers {
            let capabilities = load_worker_capabilities(&self.db, &worker.id).await?;
            output.push(worker.into_domain(capabilities)?);
        }
        Ok(output)
    }

    async fn register_worker(&self, worker: Worker, restart_message: &str) -> Result<()> {
        let restart_message = restart_message.to_owned();
        self.db
            .transaction::<_, (), anyhow::Error>(|txn| {
                Box::pin(async move {
                    let worker_id = worker.id().to_string();
                    if worker_entity::Entity::find_by_id(worker_id.clone())
                        .one(txn)
                        .await?
                        .is_some()
                    {
                        expire_worker_in_txn(txn, worker.id(), &restart_message).await?;
                        worker_capability::Entity::delete_many()
                            .filter(worker_capability::Column::WorkerId.eq(worker_id.clone()))
                            .exec(txn)
                            .await?;
                        worker_entity::Entity::delete_by_id(worker_id.clone())
                            .exec(txn)
                            .await?;
                    }
                    let now = Utc::now();
                    worker_entity::ActiveModel::from_domain(&worker, now)
                        .insert(txn)
                        .await?;
                    for capability in worker.capabilities() {
                        worker_capability::ActiveModel::from_domain(worker.id(), capability, now)?
                            .insert(txn)
                            .await?;
                    }
                    recompute_worker_summary(txn, worker.id()).await?;
                    Ok(())
                })
            })
            .await
            .map_err(Into::into)
    }

    async fn heartbeat_worker(
        &self,
        worker_id: &WorkerId,
        worker_state: WorkerState,
    ) -> Result<Option<(Worker, Vec<WorkerAssignmentDto>)>> {
        let worker_id = worker_id.clone();
        self.db
            .transaction::<_, Option<(Worker, Vec<WorkerAssignmentDto>)>, anyhow::Error>(|txn| {
                Box::pin(async move {
                    let Some(worker_model) =
                        worker_entity::Entity::find_by_id(worker_id.to_string())
                            .one(txn)
                            .await?
                    else {
                        return Ok(None);
                    };

                    let assignment_models = worker_assignment::Entity::find()
                        .filter(worker_assignment::Column::WorkerId.eq(worker_id.to_string()))
                        .filter(worker_assignment::Column::DeliveryState.eq("queued"))
                        .order_by_asc(worker_assignment::Column::CreatedAt)
                        .order_by_asc(worker_assignment::Column::Id)
                        .all(txn)
                        .await?;
                    let assignments = assignment_models
                        .iter()
                        .map(worker_assignment::Model::payload)
                        .collect::<Result<Vec<_>>>()?;
                    worker_assignment::Entity::delete_many()
                        .filter(worker_assignment::Column::WorkerId.eq(worker_id.to_string()))
                        .exec(txn)
                        .await?;

                    let mut active_worker = worker_model.into_active_model();
                    active_worker.state = Set(worker_state.to_string());
                    let now = Utc::now();
                    active_worker.last_heartbeat_at = Set(now);
                    active_worker.updated_at = Set(now);
                    active_worker.update(txn).await?;

                    let capabilities = load_worker_capabilities(txn, worker_id.as_str()).await?;
                    let worker = worker_entity::Entity::find_by_id(worker_id.to_string())
                        .one(txn)
                        .await?
                        .ok_or_else(|| anyhow!("worker disappeared during heartbeat: {worker_id}"))?
                        .into_domain(capabilities)?;
                    Ok(Some((worker, assignments)))
                })
            })
            .await
            .map_err(Into::into)
    }

    async fn expire_worker(&self, worker_id: &WorkerId, message: &str) -> Result<()> {
        let worker_id = worker_id.clone();
        let message = message.to_owned();
        self.db
            .transaction::<_, (), anyhow::Error>(|txn| {
                Box::pin(async move { expire_worker_in_txn(txn, &worker_id, &message).await })
            })
            .await
            .map_err(Into::into)
    }

    async fn assign_replica(
        &self,
        replica_id: &ReplicaId,
        worker_id: &WorkerId,
        lease: Lease,
        assignment: WorkerAssignmentDto,
    ) -> Result<bool> {
        let replica_id = replica_id.clone();
        let worker_id = worker_id.clone();
        self.db
            .transaction::<_, bool, anyhow::Error>(|txn| {
                Box::pin(async move {
                    let Some(replica_model) =
                        replica_entity::Entity::find_by_id(replica_id.to_string())
                            .one(txn)
                            .await?
                    else {
                        return Ok(false);
                    };
                    if replica_model.state != ReplicaState::Pending.to_string() {
                        return Ok(false);
                    }
                    let Some(deployment_model) =
                        deployment::Entity::find_by_id(replica_model.deployment_id.clone())
                            .one(txn)
                            .await?
                    else {
                        return Ok(false);
                    };
                    let worker_capabilities =
                        load_worker_capabilities(txn, worker_id.as_str()).await?;
                    let Some(worker_model) =
                        worker_entity::Entity::find_by_id(worker_id.to_string())
                            .one(txn)
                            .await?
                    else {
                        return Ok(false);
                    };
                    let worker = worker_model.into_domain(worker_capabilities)?;
                    let deployment = deployment_model.into_domain()?;
                    let active_leases = load_active_leases_for_worker(txn, &worker_id).await?;
                    let eligible = crate::domain_services::scheduler::worker_is_eligible(
                        &worker,
                        deployment.requirement(),
                        &active_leases,
                    );
                    if !eligible {
                        return Ok(false);
                    }

                    lease_entity::ActiveModel::from_domain(&lease, Utc::now())?
                        .insert(txn)
                        .await?;
                    let mut replica_active = replica_model.into_active_model();
                    replica_active.state = Set(ReplicaState::Assigned.to_string());
                    replica_active.assigned_worker_id = Set(Some(worker_id.to_string()));
                    replica_active.lease_id = Set(Some(lease.id().to_string()));
                    replica_active.status_message = Set(Some(format!("assigned to {worker_id}")));
                    replica_active.updated_at = Set(Utc::now());
                    replica_active.update(txn).await?;
                    worker_assignment::ActiveModel::from_payload(
                        &worker_id,
                        &replica_id,
                        lease.id(),
                        &assignment,
                        Utc::now(),
                    )?
                    .insert(txn)
                    .await?;
                    recompute_deployment_summary(txn, deployment.id()).await?;
                    recompute_worker_summary(txn, &worker_id).await?;
                    Ok(true)
                })
            })
            .await
            .map_err(Into::into)
    }

    async fn update_replica_status(
        &self,
        replica_id: &ReplicaId,
        lease_id: &LeaseId,
        state: ReplicaState,
        status_message: Option<String>,
    ) -> Result<UpdateReplicaStatusResult> {
        let replica_id = replica_id.clone();
        let lease_id = lease_id.clone();
        self.db
            .transaction::<_, UpdateReplicaStatusResult, anyhow::Error>(|txn| {
                Box::pin(async move {
                    let Some(replica_model) =
                        replica_entity::Entity::find_by_id(replica_id.to_string())
                            .one(txn)
                            .await?
                    else {
                        return Ok(UpdateReplicaStatusResult::UnknownReplica);
                    };
                    let mut replica = replica_model.clone().into_domain()?;
                    let Some(current_lease_id) = replica.lease_id().cloned() else {
                        return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                            "replica {replica_id} is not owned by an active lease"
                        )));
                    };
                    if current_lease_id != lease_id {
                        return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                            "replica {replica_id} is owned by lease {current_lease_id}, not \
                             {lease_id}"
                        )));
                    }
                    let Some(lease_model) = lease_entity::Entity::find_by_id(lease_id.to_string())
                        .one(txn)
                        .await?
                    else {
                        return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                            "unknown lease for replica {replica_id}: {lease_id}"
                        )));
                    };
                    let mut lease = lease_model.clone().into_domain()?;
                    if lease.replica_id() != &replica_id {
                        return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                            "lease {lease_id} does not belong to replica {replica_id}"
                        )));
                    }
                    if lease.state() != &LeaseState::Active {
                        return Ok(UpdateReplicaStatusResult::LeaseConflict(format!(
                            "lease {lease_id} is no longer active"
                        )));
                    }
                    if let Err(error) = replica.update_status(state, status_message) {
                        return Ok(UpdateReplicaStatusResult::LeaseConflict(error.to_string()));
                    }
                    let now = Utc::now();
                    let mut replica_active = replica_model.into_active_model();
                    replica_active.state = Set(replica.state().to_string());
                    replica_active.status_message =
                        Set(replica.status_message().map(str::to_owned));
                    replica_active.updated_at = Set(now);
                    replica_active.update(txn).await?;
                    if matches!(
                        replica.state(),
                        ReplicaState::Failed | ReplicaState::Stopped
                    ) {
                        lease.release();
                        let mut lease_active = lease_model.into_active_model();
                        lease_active.state = Set(LeaseState::Released.to_string());
                        lease_active.updated_at = Set(now);
                        lease_active.update(txn).await?;
                        recompute_worker_summary(txn, lease.worker_id()).await?;
                    }
                    recompute_deployment_summary(txn, replica.deployment_id()).await?;
                    Ok(UpdateReplicaStatusResult::Success(replica))
                })
            })
            .await
            .map_err(Into::into)
    }

    async fn touch_worker_capacity(
        &self,
        worker_id: &WorkerId,
        assigned_replicas: u32,
        available_slots: u32,
    ) -> Result<()> {
        let existing = worker_entity::Entity::find_by_id(worker_id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("unknown worker: {worker_id}"))?;
        let mut active = existing.into_active_model();
        active.assigned_replicas = Set(to_i32(assigned_replicas)?);
        active.available_slots = Set(to_i32(available_slots)?);
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await?;
        Ok(())
    }

    async fn lost_worker_ids(&self, lost_timeout: Duration) -> Result<Vec<WorkerId>> {
        let threshold = Utc::now() - chrono::Duration::from_std(lost_timeout)?;
        worker_entity::Entity::find()
            .filter(worker_entity::Column::State.ne(WorkerState::Lost.to_string()))
            .filter(worker_entity::Column::LastHeartbeatAt.lte(threshold))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|model| WorkerId::new(model.id))
            .collect::<DomainResult<Vec<_>>>()
            .map_err(Into::into)
    }

    async fn force_last_heartbeat_age(
        &self,
        worker_id: &WorkerId,
        elapsed: Duration,
    ) -> Result<bool> {
        let Some(worker) = worker_entity::Entity::find_by_id(worker_id.to_string())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut active = worker.into_active_model();
        let adjusted = Utc::now() - chrono::Duration::from_std(elapsed)?;
        active.last_heartbeat_at = Set(adjusted);
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await?;
        Ok(true)
    }
}

async fn expire_worker_in_txn(
    txn: &DatabaseTransaction,
    worker_id: &WorkerId,
    message: &str,
) -> Result<()> {
    worker_assignment::Entity::delete_many()
        .filter(worker_assignment::Column::WorkerId.eq(worker_id.to_string()))
        .exec(txn)
        .await?;

    if let Some(worker) = worker_entity::Entity::find_by_id(worker_id.to_string())
        .one(txn)
        .await?
    {
        let mut active = worker.into_active_model();
        active.state = Set(WorkerState::Lost.to_string());
        active.updated_at = Set(Utc::now());
        active.update(txn).await?;
    }

    let leases = lease_entity::Entity::find()
        .filter(lease_entity::Column::WorkerId.eq(worker_id.to_string()))
        .filter(lease_entity::Column::State.eq(LeaseState::Active.to_string()))
        .all(txn)
        .await?;
    for lease_model in leases {
        let lease = lease_model.clone().into_domain()?;
        let mut lease_active = lease_model.into_active_model();
        lease_active.state = Set(LeaseState::Expired.to_string());
        lease_active.updated_at = Set(Utc::now());
        lease_active.update(txn).await?;

        if let Some(replica_model) =
            replica_entity::Entity::find_by_id(lease.replica_id().to_string())
                .one(txn)
                .await?
        {
            let mut replica = replica_model.clone().into_domain()?;
            replica.mark_stopped(message.to_owned());
            let mut replica_active = replica_model.into_active_model();
            replica_active.state = Set(replica.state().to_string());
            replica_active.status_message = Set(replica.status_message().map(str::to_owned));
            replica_active.updated_at = Set(Utc::now());
            replica_active.update(txn).await?;
            recompute_deployment_summary(txn, replica.deployment_id()).await?;
        }
    }
    recompute_worker_summary(txn, worker_id).await?;
    Ok(())
}

async fn recompute_deployment_summary(
    txn: &DatabaseTransaction,
    deployment_id: &DeploymentId,
) -> Result<()> {
    let replicas = replica_entity::Entity::find()
        .filter(replica_entity::Column::DeploymentId.eq(deployment_id.to_string()))
        .all(txn)
        .await?;
    let replicas = replicas
        .into_iter()
        .map(replica_entity::Model::into_domain)
        .collect::<Result<Vec<_>>>()?;
    let summary = DeploymentStatusSummary::from_replicas(replicas.iter());
    let existing = deployment::Entity::find_by_id(deployment_id.to_string())
        .one(txn)
        .await?
        .ok_or_else(|| anyhow!("unknown deployment for summary refresh: {deployment_id}"))?;
    let mut active = existing.into_active_model();
    set_status_on_deployment(&mut active, &summary)?;
    active.updated_at = Set(Utc::now());
    active.update(txn).await?;
    Ok(())
}

async fn recompute_worker_summary(txn: &DatabaseTransaction, worker_id: &WorkerId) -> Result<()> {
    let Some(worker_model) = worker_entity::Entity::find_by_id(worker_id.to_string())
        .one(txn)
        .await?
    else {
        return Ok(());
    };
    let capabilities = load_worker_capabilities(txn, worker_id.as_str()).await?;
    let worker = worker_model.clone().into_domain(capabilities)?;
    let leases = load_active_leases_for_worker(txn, worker_id).await?;
    let available_slots = worker
        .capabilities()
        .iter()
        .filter_map(|capability| {
            available_capacity_for_requirement(
                &worker,
                &WorkloadRequirement::new(
                    capability.framework().clone(),
                    capability.mode().clone(),
                    capability.device().clone(),
                    capability.accelerator_runtime().clone(),
                    capability.architecture_family().clone(),
                    0,
                    0,
                ),
                &leases,
            )
            .map(|(slots, _)| slots)
        })
        .fold(0, u32::saturating_add);
    let mut active = worker_model.into_active_model();
    active.assigned_replicas = Set(to_i32(leases.len() as u32)?);
    active.available_slots = Set(to_i32(available_slots)?);
    active.updated_at = Set(Utc::now());
    active.update(txn).await?;
    Ok(())
}

async fn load_active_leases_for_worker<C>(db: &C, worker_id: &WorkerId) -> Result<Vec<Lease>>
where
    C: ConnectionTrait,
{
    lease_entity::Entity::find()
        .filter(lease_entity::Column::WorkerId.eq(worker_id.to_string()))
        .filter(lease_entity::Column::State.eq(LeaseState::Active.to_string()))
        .all(db)
        .await?
        .into_iter()
        .map(lease_entity::Model::into_domain)
        .collect()
}

async fn load_worker_capabilities<C>(db: &C, worker_id: &str) -> Result<Vec<WorkerCapability>>
where
    C: ConnectionTrait,
{
    worker_capability::Entity::find()
        .filter(worker_capability::Column::WorkerId.eq(worker_id))
        .order_by_asc(worker_capability::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(worker_capability::Model::into_domain)
        .collect()
}

fn to_i32(value: u32) -> Result<i32> {
    i32::try_from(value).context("value does not fit into i32")
}

fn to_i64(value: u64) -> Result<i64> {
    i64::try_from(value).context("value does not fit into i64")
}

fn parse_framework(value: &str) -> Result<Framework> {
    value
        .parse()
        .map_err(|_| anyhow!("invalid framework: {value}"))
}

fn parse_mode(value: &str) -> Result<WorkloadMode> {
    value
        .parse()
        .map_err(|_| anyhow!("invalid workload mode: {value}"))
}

fn parse_device(value: &str) -> Result<DeviceClass> {
    value
        .parse()
        .map_err(|_| anyhow!("invalid device class: {value}"))
}

fn parse_worker_state(value: &str) -> Result<WorkerState> {
    match value {
        "draining" => Ok(WorkerState::Draining),
        "lost" => Ok(WorkerState::Lost),
        "ready" => Ok(WorkerState::Ready),
        "starting" => Ok(WorkerState::Starting),
        "unhealthy" => Ok(WorkerState::Unhealthy),
        _ => Err(anyhow!("invalid worker state: {value}")),
    }
}

fn parse_replica_state(value: &str) -> Result<ReplicaState> {
    match value {
        "assigned" => Ok(ReplicaState::Assigned),
        "failed" => Ok(ReplicaState::Failed),
        "pending" => Ok(ReplicaState::Pending),
        "pulling" => Ok(ReplicaState::Pulling),
        "ready" => Ok(ReplicaState::Ready),
        "starting" => Ok(ReplicaState::Starting),
        "stopped" => Ok(ReplicaState::Stopped),
        _ => Err(anyhow!("invalid replica state: {value}")),
    }
}

fn parse_lease_state(value: &str) -> Result<LeaseState> {
    match value {
        "active" => Ok(LeaseState::Active),
        "expired" => Ok(LeaseState::Expired),
        "released" => Ok(LeaseState::Released),
        _ => Err(anyhow!("invalid lease state: {value}")),
    }
}

fn set_requirement_on_deployment(
    active: &mut deployment::ActiveModel,
    requirement: &WorkloadRequirement,
) -> Result<()> {
    active.framework = Set(requirement.framework().to_string());
    active.mode = Set(requirement.mode().to_string());
    active.device = Set(requirement.device().to_string());
    active.accelerator_runtime = Set(requirement.accelerator_runtime().to_string());
    active.architecture_family = Set(requirement.architecture_family().to_string());
    active.memory_requirement_bytes = Set(to_i64(requirement.memory_requirement_bytes())?);
    active.concurrency_requirement = Set(to_i32(requirement.concurrency_requirement())?);
    Ok(())
}

fn set_status_on_deployment(
    active: &mut deployment::ActiveModel,
    status: &DeploymentStatusSummary,
) -> Result<()> {
    active.pending_replicas = Set(to_i32(status.pending_replicas())?);
    active.assigned_replicas = Set(to_i32(status.assigned_replicas())?);
    active.pulling_replicas = Set(to_i32(status.pulling_replicas())?);
    active.starting_replicas = Set(to_i32(status.starting_replicas())?);
    active.ready_replicas = Set(to_i32(status.ready_replicas())?);
    active.failed_replicas = Set(to_i32(status.failed_replicas())?);
    active.stopped_replicas = Set(to_i32(status.stopped_replicas())?);
    Ok(())
}

pub mod deployment {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "deployments")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub artifact_ref: String,
        pub replicas_desired: i32,
        pub framework: String,
        pub mode: String,
        pub device: String,
        pub accelerator_runtime: String,
        pub architecture_family: String,
        pub memory_requirement_bytes: i64,
        pub concurrency_requirement: i32,
        pub pending_replicas: i32,
        pub assigned_replicas: i32,
        pub pulling_replicas: i32,
        pub starting_replicas: i32,
        pub ready_replicas: i32,
        pub failed_replicas: i32,
        pub stopped_replicas: i32,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod replica_entity {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "replicas")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                 String,
        pub deployment_id:      String,
        pub state:              String,
        pub assigned_worker_id: Option<String>,
        pub lease_id:           Option<String>,
        pub status_message:     Option<String>,
        pub created_at:         DateTime<Utc>,
        pub updated_at:         DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod lease_entity {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "leases")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub worker_id: String,
        pub deployment_id: String,
        pub replica_id: String,
        pub state: String,
        pub framework: String,
        pub mode: String,
        pub device: String,
        pub accelerator_runtime: String,
        pub architecture_family: String,
        pub memory_requirement_bytes: i64,
        pub concurrency_requirement: i32,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod worker_entity {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "workers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                String,
        pub display_name:      String,
        pub state:             String,
        pub assigned_replicas: i32,
        pub available_slots:   i32,
        pub last_heartbeat_at: DateTime<Utc>,
        pub created_at:        DateTime<Utc>,
        pub updated_at:        DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod worker_capability {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "worker_capabilities")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id:                     i64,
        pub worker_id:              String,
        pub framework:              String,
        pub mode:                   String,
        pub device:                 String,
        pub accelerator_runtime:    String,
        pub architecture_family:    String,
        pub available_memory_bytes: i64,
        pub concurrency_slots:      i32,
        pub created_at:             DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod worker_assignment {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "worker_assignments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id:             i64,
        pub worker_id:      String,
        pub replica_id:     String,
        pub lease_id:       String,
        pub payload:        Json,
        pub delivery_state: String,
        pub created_at:     DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

impl deployment::ActiveModel {
    fn from_domain(deployment: &Deployment, now: DateTime<Utc>) -> Result<Self> {
        Ok(Self {
            id: Set(deployment.id().to_string()),
            name: Set(deployment.name().to_owned()),
            artifact_ref: Set(deployment.artifact_ref().to_string()),
            replicas_desired: Set(to_i32(deployment.replicas_desired())?),
            framework: Set(deployment.requirement().framework().to_string()),
            mode: Set(deployment.requirement().mode().to_string()),
            device: Set(deployment.requirement().device().to_string()),
            accelerator_runtime: Set(deployment.requirement().accelerator_runtime().to_string()),
            architecture_family: Set(deployment.requirement().architecture_family().to_string()),
            memory_requirement_bytes: Set(to_i64(
                deployment.requirement().memory_requirement_bytes(),
            )?),
            concurrency_requirement: Set(to_i32(
                deployment.requirement().concurrency_requirement(),
            )?),
            pending_replicas: Set(to_i32(deployment.status().pending_replicas())?),
            assigned_replicas: Set(to_i32(deployment.status().assigned_replicas())?),
            pulling_replicas: Set(to_i32(deployment.status().pulling_replicas())?),
            starting_replicas: Set(to_i32(deployment.status().starting_replicas())?),
            ready_replicas: Set(to_i32(deployment.status().ready_replicas())?),
            failed_replicas: Set(to_i32(deployment.status().failed_replicas())?),
            stopped_replicas: Set(to_i32(deployment.status().stopped_replicas())?),
            created_at: Set(now),
            updated_at: Set(now),
        })
    }
}

impl deployment::Model {
    fn into_domain(self) -> Result<Deployment> {
        let requirement = WorkloadRequirement::new(
            parse_framework(&self.framework)?,
            parse_mode(&self.mode)?,
            parse_device(&self.device)?,
            RuntimeName::new(self.accelerator_runtime)?,
            ArchitectureFamily::new(self.architecture_family)?,
            u64::try_from(self.memory_requirement_bytes)?,
            u32::try_from(self.concurrency_requirement)?,
        );
        Ok(Deployment::rehydrate(
            DeploymentId::new(self.id)?,
            self.name,
            ArtifactRef::new(self.artifact_ref)?,
            u32::try_from(self.replicas_desired)?,
            requirement,
            DeploymentStatusSummary::from_counts(
                u32::try_from(self.pending_replicas)?,
                u32::try_from(self.assigned_replicas)?,
                u32::try_from(self.pulling_replicas)?,
                u32::try_from(self.starting_replicas)?,
                u32::try_from(self.ready_replicas)?,
                u32::try_from(self.failed_replicas)?,
                u32::try_from(self.stopped_replicas)?,
            ),
        ))
    }
}

impl replica_entity::ActiveModel {
    fn from_domain(replica: &Replica, now: DateTime<Utc>) -> Self {
        Self {
            id:                 Set(replica.id().to_string()),
            deployment_id:      Set(replica.deployment_id().to_string()),
            state:              Set(replica.state().to_string()),
            assigned_worker_id: Set(replica.worker_id().map(ToString::to_string)),
            lease_id:           Set(replica.lease_id().map(ToString::to_string)),
            status_message:     Set(replica.status_message().map(str::to_owned)),
            created_at:         Set(now),
            updated_at:         Set(now),
        }
    }
}

impl replica_entity::Model {
    fn into_domain(self) -> Result<Replica> {
        Ok(Replica::rehydrate(
            ReplicaId::new(self.id)?,
            DeploymentId::new(self.deployment_id)?,
            self.lease_id.map(LeaseId::new).transpose()?,
            parse_replica_state(&self.state)?,
            self.status_message,
            self.assigned_worker_id.map(WorkerId::new).transpose()?,
        ))
    }
}

impl lease_entity::ActiveModel {
    fn from_domain(lease: &Lease, now: DateTime<Utc>) -> Result<Self> {
        Ok(Self {
            id: Set(lease.id().to_string()),
            worker_id: Set(lease.worker_id().to_string()),
            deployment_id: Set(lease.deployment_id().to_string()),
            replica_id: Set(lease.replica_id().to_string()),
            state: Set(lease.state().to_string()),
            framework: Set(lease.requirement().framework().to_string()),
            mode: Set(lease.requirement().mode().to_string()),
            device: Set(lease.requirement().device().to_string()),
            accelerator_runtime: Set(lease.requirement().accelerator_runtime().to_string()),
            architecture_family: Set(lease.requirement().architecture_family().to_string()),
            memory_requirement_bytes: Set(to_i64(lease.requirement().memory_requirement_bytes())?),
            concurrency_requirement: Set(to_i32(lease.requirement().concurrency_requirement())?),
            created_at: Set(now),
            updated_at: Set(now),
        })
    }
}

impl lease_entity::Model {
    fn into_domain(self) -> Result<Lease> {
        Ok(Lease::rehydrate(
            LeaseId::new(self.id)?,
            WorkerId::new(self.worker_id)?,
            DeploymentId::new(self.deployment_id)?,
            ReplicaId::new(self.replica_id)?,
            WorkloadRequirement::new(
                parse_framework(&self.framework)?,
                parse_mode(&self.mode)?,
                parse_device(&self.device)?,
                RuntimeName::new(self.accelerator_runtime)?,
                ArchitectureFamily::new(self.architecture_family)?,
                u64::try_from(self.memory_requirement_bytes)?,
                u32::try_from(self.concurrency_requirement)?,
            ),
            parse_lease_state(&self.state)?,
        ))
    }
}

impl worker_entity::ActiveModel {
    fn from_domain(worker: &Worker, now: DateTime<Utc>) -> Self {
        Self {
            id:                Set(worker.id().to_string()),
            display_name:      Set(worker.display_name().to_owned()),
            state:             Set(worker.state().to_string()),
            assigned_replicas: Set(i32::try_from(worker.assigned_replicas()).unwrap_or(i32::MAX)),
            available_slots:   Set(i32::try_from(worker.available_slots()).unwrap_or(i32::MAX)),
            last_heartbeat_at: Set(now),
            created_at:        Set(now),
            updated_at:        Set(now),
        }
    }
}

impl worker_entity::Model {
    fn into_domain(self, capabilities: Vec<WorkerCapability>) -> Result<Worker> {
        Ok(Worker::rehydrate(
            WorkerId::new(self.id)?,
            self.display_name,
            capabilities,
            parse_worker_state(&self.state)?,
            u32::try_from(self.assigned_replicas)?,
            u32::try_from(self.available_slots)?,
        ))
    }
}

impl worker_capability::ActiveModel {
    fn from_domain(
        worker_id: &WorkerId,
        capability: &WorkerCapability,
        now: DateTime<Utc>,
    ) -> Result<Self> {
        Ok(Self {
            id:                     sea_orm::NotSet,
            worker_id:              Set(worker_id.to_string()),
            framework:              Set(capability.framework().to_string()),
            mode:                   Set(capability.mode().to_string()),
            device:                 Set(capability.device().to_string()),
            accelerator_runtime:    Set(capability.accelerator_runtime().to_string()),
            architecture_family:    Set(capability.architecture_family().to_string()),
            available_memory_bytes: Set(to_i64(capability.available_memory_bytes())?),
            concurrency_slots:      Set(to_i32(capability.concurrency_slots())?),
            created_at:             Set(now),
        })
    }
}

impl worker_capability::Model {
    fn into_domain(self) -> Result<WorkerCapability> {
        Ok(WorkerCapability::new(
            parse_framework(&self.framework)?,
            parse_mode(&self.mode)?,
            parse_device(&self.device)?,
            RuntimeName::new(self.accelerator_runtime)?,
            ArchitectureFamily::new(self.architecture_family)?,
            u64::try_from(self.available_memory_bytes)?,
            u32::try_from(self.concurrency_slots)?,
        ))
    }
}

impl worker_assignment::ActiveModel {
    fn from_payload(
        worker_id: &WorkerId,
        replica_id: &ReplicaId,
        lease_id: &LeaseId,
        payload: &WorkerAssignmentDto,
        now: DateTime<Utc>,
    ) -> Result<Self> {
        Ok(Self {
            id:             sea_orm::NotSet,
            worker_id:      Set(worker_id.to_string()),
            replica_id:     Set(replica_id.to_string()),
            lease_id:       Set(lease_id.to_string()),
            payload:        Set(Value::from(serde_json::to_value(payload)?)),
            delivery_state: Set("queued".to_owned()),
            created_at:     Set(now),
        })
    }
}

impl worker_assignment::Model {
    fn payload(&self) -> Result<WorkerAssignmentDto> {
        serde_json::from_value(self.payload.clone()).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, sync::OnceLock};

    use sea_orm::{ConnectionTrait as _, Database, DbBackend, Statement};
    use sea_orm_migration::MigratorTrait as _;
    use tokio::sync::Mutex;

    use super::{PostgresStorage, ids};
    use crate::repositories::{StorageBackend as _, migration::Migrator};

    static POSTGRES_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn postgres_test_database_url() -> Option<String> {
        env::var("DLP_POSTGRES_TEST_DATABASE_URL").ok()
    }

    async fn connect_test_database() -> Option<sea_orm::DatabaseConnection> {
        let database_url = postgres_test_database_url()?;
        Database::connect(database_url).await.ok()
    }

    async fn sequence_exists(
        connection: &sea_orm::DatabaseConnection,
        sequence_name: &str,
    ) -> anyhow::Result<bool> {
        let statement = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT to_regclass($1) IS NOT NULL AS present",
            [sequence_name.into()],
        );
        let row = connection
            .query_one(statement)
            .await?
            .ok_or_else(|| anyhow::anyhow!("sequence existence query returned no row"))?;
        let present: bool = row.try_get("", "present")?;
        Ok(present)
    }

    #[tokio::test]
    async fn sequence_backed_ids_are_prefixed_and_unique() {
        let Some(connection) = connect_test_database().await else {
            return;
        };
        let _guard = POSTGRES_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .await;
        if let Err(error) = Migrator::up(&connection, None).await {
            panic!("failed to apply migrations: {error}");
        }
        let storage = PostgresStorage::new(connection);

        let deployment_a = storage
            .next_deployment_id()
            .await
            .unwrap_or_else(|error| panic!("failed to allocate deployment id: {error}"));
        let deployment_b = storage
            .next_deployment_id()
            .await
            .unwrap_or_else(|error| panic!("failed to allocate deployment id: {error}"));
        let replica_a = storage
            .next_replica_id()
            .await
            .unwrap_or_else(|error| panic!("failed to allocate replica id: {error}"));
        let replica_b = storage
            .next_replica_id()
            .await
            .unwrap_or_else(|error| panic!("failed to allocate replica id: {error}"));
        let lease_a = storage
            .next_lease_id()
            .await
            .unwrap_or_else(|error| panic!("failed to allocate lease id: {error}"));
        let lease_b = storage
            .next_lease_id()
            .await
            .unwrap_or_else(|error| panic!("failed to allocate lease id: {error}"));

        assert!(deployment_a.starts_with("deployment-"));
        assert!(deployment_b.starts_with("deployment-"));
        assert_ne!(deployment_a, deployment_b);
        assert!(replica_a.starts_with("replica-"));
        assert!(replica_b.starts_with("replica-"));
        assert_ne!(replica_a, replica_b);
        assert!(lease_a.starts_with("lease-"));
        assert!(lease_b.starts_with("lease-"));
        assert_ne!(lease_a, lease_b);
    }

    #[tokio::test]
    async fn migrations_create_required_sequences() {
        let Some(connection) = connect_test_database().await else {
            return;
        };
        let _guard = POSTGRES_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .await;
        if let Err(error) = Migrator::up(&connection, None).await {
            panic!("failed to apply migrations: {error}");
        }

        for sequence in ["deployment_id_seq", "replica_id_seq", "lease_id_seq"] {
            let exists = sequence_exists(&connection, sequence)
                .await
                .unwrap_or_else(|error| panic!("failed to check sequence {sequence}: {error}"));
            assert!(exists);
        }
    }

    #[tokio::test]
    async fn migrations_down_remove_required_sequences() {
        let Some(connection) = connect_test_database().await else {
            return;
        };
        let _guard = POSTGRES_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .await;
        if let Err(error) = Migrator::up(&connection, None).await {
            panic!("failed to apply migrations: {error}");
        }
        if let Err(error) = Migrator::down(&connection, None).await {
            panic!("failed to roll migrations down: {error}");
        }

        for sequence in ["deployment_id_seq", "replica_id_seq", "lease_id_seq"] {
            let exists = sequence_exists(&connection, sequence)
                .await
                .unwrap_or_else(|error| panic!("failed to check sequence {sequence}: {error}"));
            assert!(!exists);
        }

        if let Err(error) = Migrator::up(&connection, None).await {
            panic!("failed to re-apply migrations: {error}");
        }
    }

    #[tokio::test]
    async fn typed_id_helpers_are_the_only_sequence_entrypoints() {
        let Some(connection) = connect_test_database().await else {
            return;
        };
        let _guard = POSTGRES_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .await;
        if let Err(error) = Migrator::up(&connection, None).await {
            panic!("failed to apply migrations: {error}");
        }

        let deployment_id = ids::next_deployment_id(&connection)
            .await
            .unwrap_or_else(|error| panic!("failed to allocate deployment id: {error}"));
        assert!(deployment_id.starts_with("deployment-"));
        let replica_id = ids::next_replica_id(&connection)
            .await
            .unwrap_or_else(|error| panic!("failed to allocate replica id: {error}"));
        assert!(replica_id.starts_with("replica-"));
        let lease_id = ids::next_lease_id(&connection)
            .await
            .unwrap_or_else(|error| panic!("failed to allocate lease id: {error}"));
        assert!(lease_id.starts_with("lease-"));
    }
}
