mod entities;

use anyhow::Context;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait,
    IntoActiveModel, Statement, TransactionTrait, Value, entity::Set,
};

use self::entities::{jobs, workers};
use crate::{ApiError, JobStore};
use dlp_migration::{Migrator, MigratorTrait};
use dlp_shared::{
    JobKind, JobRecord, JobResultRequest, JobStatus, SubmitJobRequest, WorkerRecord,
    WorkerRegistration, WorkerStatus,
};

pub struct SeaOrmStore {
    db: DatabaseConnection,
}

impl SeaOrmStore {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let db = Database::connect(database_url)
            .await
            .context("failed to connect to PostgreSQL")?;
        Migrator::up(&db, None)
            .await
            .context("failed to apply SeaORM migrations")?;
        Ok(Self { db })
    }
}

#[async_trait::async_trait]
impl JobStore for SeaOrmStore {
    async fn create_job(&self, request: SubmitJobRequest) -> Result<JobRecord, ApiError> {
        let record = JobRecord {
            job_id: uuid::Uuid::new_v4().to_string(),
            job_kind: request.job_kind,
            status: JobStatus::Queued,
            required_capabilities: request.required_capabilities,
            payload: request.payload,
            assigned_worker: None,
            result: None,
            error: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
        };

        jobs::ActiveModel::from_record(record.clone())
            .insert(&self.db)
            .await
            .map_err(ApiError::from_db)?;

        Ok(record)
    }

    async fn get_job(&self, id: &str) -> Result<JobRecord, ApiError> {
        jobs::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map_err(ApiError::from_db)?
            .map(TryInto::try_into)
            .transpose()?
            .ok_or_else(|| ApiError::not_found(format!("job {id} not found")))
    }

    async fn register_worker(&self, request: WorkerRegistration) -> Result<WorkerRecord, ApiError> {
        let now = chrono::Utc::now();
        let worker = WorkerRecord {
            worker_id: request.worker_id,
            name: request.name,
            capabilities: request.capabilities,
            status: WorkerStatus::Online,
            last_seen_at: now,
        };

        if let Some(existing) = workers::Entity::find_by_id(worker.worker_id.clone())
            .one(&self.db)
            .await
            .map_err(ApiError::from_db)?
        {
            let mut active = existing.into_active_model();
            active.name = Set(worker.name.clone());
            active.capabilities = Set(worker.capabilities.clone());
            active.status = Set(worker_status_to_db(&worker.status).to_string());
            active.last_seen_at = Set(worker.last_seen_at);
            active.update(&self.db).await.map_err(ApiError::from_db)?;
        } else {
            workers::ActiveModel::from_record(worker.clone())
                .insert(&self.db)
                .await
                .map_err(ApiError::from_db)?;
        }

        Ok(worker)
    }

    async fn claim_job(&self, worker_id: &str) -> Result<Option<JobRecord>, ApiError> {
        let tx = self.db.begin().await.map_err(ApiError::from_db)?;

        let worker: WorkerRecord = workers::Entity::find_by_id(worker_id.to_owned())
            .one(&tx)
            .await
            .map_err(ApiError::from_db)?
            .map(TryInto::try_into)
            .transpose()?
            .ok_or_else(|| ApiError::not_found(format!("worker {worker_id} not found")))?;

        let started_at = chrono::Utc::now();
        let capabilities = serde_json::to_value(&worker.capabilities).map_err(ApiError::from_db)?;
        let statement = Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
            WITH candidate AS (
                SELECT job_id
                FROM jobs
                WHERE status = $1
                  AND required_capabilities <@ COALESCE(
                    ARRAY(SELECT jsonb_array_elements_text($2::jsonb)),
                    ARRAY[]::text[]
                  )
                ORDER BY created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            UPDATE jobs
            SET status = $3, assigned_worker = $4, started_at = $5
            WHERE job_id = (SELECT job_id FROM candidate)
            RETURNING
                job_id,
                job_kind,
                status,
                required_capabilities,
                payload,
                assigned_worker,
                result,
                error,
                created_at,
                started_at,
                finished_at
            "#,
            vec![
                Value::String(Some(Box::new(
                    job_status_to_db(&JobStatus::Queued).to_string(),
                ))),
                Value::Json(Some(Box::new(capabilities))),
                Value::String(Some(Box::new(
                    job_status_to_db(&JobStatus::Running).to_string(),
                ))),
                worker.worker_id.clone().into(),
                started_at.into(),
            ],
        );

        let Some(row) = tx.query_one(statement).await.map_err(ApiError::from_db)? else {
            tx.commit().await.map_err(ApiError::from_db)?;
            return Ok(None);
        };

        let job = map_claimed_job(row)?;
        tx.commit().await.map_err(ApiError::from_db)?;
        Ok(Some(job))
    }

    async fn complete_job(
        &self,
        job_id: &str,
        request: JobResultRequest,
    ) -> Result<JobRecord, ApiError> {
        let model = jobs::Entity::find_by_id(job_id.to_owned())
            .one(&self.db)
            .await
            .map_err(ApiError::from_db)?
            .ok_or_else(|| ApiError::not_found(format!("job {job_id} not found")))?;
        let mut job: JobRecord = model.try_into()?;

        if job.assigned_worker.as_deref() != Some(request.worker_id.as_str()) {
            return Err(ApiError::conflict(format!(
                "job {job_id} is not assigned to worker {}",
                request.worker_id
            )));
        }
        if matches!(job.status, JobStatus::Completed | JobStatus::Failed) {
            return Err(ApiError::conflict(format!(
                "job {job_id} is already terminal"
            )));
        }

        job.status = if request.success {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };
        job.result = request.result;
        job.error = request.error;
        job.finished_at = Some(chrono::Utc::now());

        jobs::ActiveModel::from_record(job.clone())
            .update(&self.db)
            .await
            .map_err(ApiError::from_db)?;

        Ok(job)
    }
}

fn map_claimed_job(row: sea_orm::QueryResult) -> Result<JobRecord, ApiError> {
    Ok(JobRecord {
        job_id: row.try_get("", "job_id").map_err(ApiError::from_db)?,
        job_kind: parse_job_kind(
            &row.try_get::<String>("", "job_kind")
                .map_err(ApiError::from_db)?,
        )?,
        status: parse_job_status(
            &row.try_get::<String>("", "status")
                .map_err(ApiError::from_db)?,
        )?,
        required_capabilities: row
            .try_get("", "required_capabilities")
            .map_err(ApiError::from_db)?,
        payload: row.try_get("", "payload").map_err(ApiError::from_db)?,
        assigned_worker: row
            .try_get("", "assigned_worker")
            .map_err(ApiError::from_db)?,
        result: row.try_get("", "result").map_err(ApiError::from_db)?,
        error: row.try_get("", "error").map_err(ApiError::from_db)?,
        created_at: row.try_get("", "created_at").map_err(ApiError::from_db)?,
        started_at: row.try_get("", "started_at").map_err(ApiError::from_db)?,
        finished_at: row.try_get("", "finished_at").map_err(ApiError::from_db)?,
    })
}

fn parse_job_kind(value: &str) -> Result<JobKind, ApiError> {
    match value {
        "dummy_inference" => Ok(JobKind::DummyInference),
        _ => Err(ApiError::database_message(format!(
            "unknown job kind stored in database: {value}"
        ))),
    }
}

fn job_kind_to_db(kind: &JobKind) -> &'static str {
    match kind {
        JobKind::DummyInference => "dummy_inference",
    }
}

fn parse_job_status(value: &str) -> Result<JobStatus, ApiError> {
    match value {
        "queued" => Ok(JobStatus::Queued),
        "running" => Ok(JobStatus::Running),
        "completed" => Ok(JobStatus::Completed),
        "failed" => Ok(JobStatus::Failed),
        _ => Err(ApiError::database_message(format!(
            "unknown job status stored in database: {value}"
        ))),
    }
}

fn job_status_to_db(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    }
}

fn parse_worker_status(value: &str) -> Result<WorkerStatus, ApiError> {
    match value {
        "online" => Ok(WorkerStatus::Online),
        "offline" => Ok(WorkerStatus::Offline),
        _ => Err(ApiError::database_message(format!(
            "unknown worker status stored in database: {value}"
        ))),
    }
}

fn worker_status_to_db(status: &WorkerStatus) -> &'static str {
    match status {
        WorkerStatus::Online => "online",
        WorkerStatus::Offline => "offline",
    }
}

impl From<JobRecord> for jobs::ActiveModel {
    fn from(value: JobRecord) -> Self {
        jobs::ActiveModel::from_record(value)
    }
}

impl From<WorkerRecord> for workers::ActiveModel {
    fn from(value: WorkerRecord) -> Self {
        workers::ActiveModel::from_record(value)
    }
}
