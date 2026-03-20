use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use dlp_shared::{
    ClaimJobResponse, JobId, JobKind, JobRecord, JobResultRequest, JobStatus, SubmitJobRequest,
    WorkerId, WorkerRecord, WorkerRegistration, WorkerStatus,
};
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let bind_addr = std::env::var("DLP_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let database_url = std::env::var("DLP_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/dlp".to_string());

    let store = Arc::new(PostgresStore::connect(&database_url).await?);
    let app = app(store);
    let addr: SocketAddr = bind_addr.parse().context("invalid DLP_BIND_ADDR")?;

    info!("control-plane listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dlp_control_plane=info,tower_http=info".into()),
        )
        .init();
}

fn app(store: Arc<dyn JobStore>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/jobs", post(submit_job))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/result", post(post_result))
        .route("/workers/register", post(register_worker))
        .route("/workers/{id}/claim", post(claim_job))
        .with_state(AppState { store })
}

#[derive(Clone)]
struct AppState {
    store: Arc<dyn JobStore>,
}

async fn health() -> &'static str {
    "ok"
}

async fn submit_job(
    State(state): State<AppState>,
    Json(request): Json<SubmitJobRequest>,
) -> Result<Json<JobRecord>, ApiError> {
    let job = state.store.create_job(request).await?;
    Ok(Json(job))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<JobId>,
) -> Result<Json<JobRecord>, ApiError> {
    let job = state.store.get_job(&id).await?;
    Ok(Json(job))
}

async fn register_worker(
    State(state): State<AppState>,
    Json(request): Json<WorkerRegistration>,
) -> Result<Json<WorkerRecord>, ApiError> {
    let worker = state.store.register_worker(request).await?;
    Ok(Json(worker))
}

async fn claim_job(
    State(state): State<AppState>,
    Path(worker_id): Path<WorkerId>,
) -> Result<Json<ClaimJobResponse>, ApiError> {
    let job = state.store.claim_job(&worker_id).await?;
    Ok(Json(ClaimJobResponse { job }))
}

async fn post_result(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
    Json(request): Json<JobResultRequest>,
) -> Result<Json<JobRecord>, ApiError> {
    let job = state.store.complete_job(&job_id, request).await?;
    Ok(Json(job))
}

#[async_trait]
trait JobStore: Send + Sync {
    async fn create_job(&self, request: SubmitJobRequest) -> Result<JobRecord, ApiError>;
    async fn get_job(&self, id: &str) -> Result<JobRecord, ApiError>;
    async fn register_worker(&self, request: WorkerRegistration) -> Result<WorkerRecord, ApiError>;
    async fn claim_job(&self, worker_id: &str) -> Result<Option<JobRecord>, ApiError>;
    async fn complete_job(
        &self,
        job_id: &str,
        request: JobResultRequest,
    ) -> Result<JobRecord, ApiError>;
}

struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .context("failed to connect to PostgreSQL")?;

        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    async fn init_schema(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                job_id TEXT PRIMARY KEY,
                job_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                required_capabilities TEXT[] NOT NULL DEFAULT '{}',
                payload JSONB NOT NULL DEFAULT '{}'::jsonb,
                assigned_worker TEXT,
                result JSONB,
                error TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                started_at TIMESTAMPTZ,
                finished_at TIMESTAMPTZ
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workers (
                worker_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                capabilities TEXT[] NOT NULL DEFAULT '{}',
                status TEXT NOT NULL,
                last_seen_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl JobStore for PostgresStore {
    async fn create_job(&self, request: SubmitJobRequest) -> Result<JobRecord, ApiError> {
        let job = JobRecord {
            job_id: Uuid::new_v4().to_string(),
            job_kind: request.job_kind,
            status: JobStatus::Queued,
            required_capabilities: request.required_capabilities,
            payload: request.payload,
            assigned_worker: None,
            result: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        };

        sqlx::query(
            r#"
            INSERT INTO jobs (
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
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(&job.job_id)
        .bind(job_kind_to_db(&job.job_kind))
        .bind(job_status_to_db(&job.status))
        .bind(&job.required_capabilities)
        .bind(&job.payload)
        .bind(&job.assigned_worker)
        .bind(&job.result)
        .bind(&job.error)
        .bind(job.created_at)
        .bind(job.started_at)
        .bind(job.finished_at)
        .execute(&self.pool)
        .await
        .map_err(ApiError::from_db)?;

        Ok(job)
    }

    async fn get_job(&self, id: &str) -> Result<JobRecord, ApiError> {
        let row = sqlx::query(
            r#"
            SELECT
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
            FROM jobs
            WHERE job_id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(ApiError::from_db)?;

        row.map(map_job_row)
            .transpose()?
            .ok_or_else(|| ApiError::not_found(format!("job {id} not found")))
    }

    async fn register_worker(&self, request: WorkerRegistration) -> Result<WorkerRecord, ApiError> {
        let worker = WorkerRecord {
            worker_id: request.worker_id,
            name: request.name,
            capabilities: request.capabilities,
            status: WorkerStatus::Online,
            last_seen_at: Utc::now(),
        };

        sqlx::query(
            r#"
            INSERT INTO workers (worker_id, name, capabilities, status, last_seen_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (worker_id) DO UPDATE
            SET
                name = EXCLUDED.name,
                capabilities = EXCLUDED.capabilities,
                status = EXCLUDED.status,
                last_seen_at = EXCLUDED.last_seen_at
            "#,
        )
        .bind(&worker.worker_id)
        .bind(&worker.name)
        .bind(&worker.capabilities)
        .bind(worker_status_to_db(&worker.status))
        .bind(worker.last_seen_at)
        .execute(&self.pool)
        .await
        .map_err(ApiError::from_db)?;

        Ok(worker)
    }

    async fn claim_job(&self, worker_id: &str) -> Result<Option<JobRecord>, ApiError> {
        let mut tx = self.pool.begin().await.map_err(ApiError::from_db)?;
        let worker_row = sqlx::query(
            r#"
            SELECT worker_id, name, capabilities, status, last_seen_at
            FROM workers
            WHERE worker_id = $1
            "#,
        )
        .bind(worker_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::from_db)?;

        let worker = worker_row
            .map(map_worker_row)
            .transpose()?
            .ok_or_else(|| ApiError::not_found(format!("worker {worker_id} not found")))?;

        let job_row = sqlx::query(
            r#"
            SELECT
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
            FROM jobs
            WHERE status = 'queued'
              AND required_capabilities <@ $1::text[]
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
        )
        .bind(&worker.capabilities)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::from_db)?;

        let Some(mut job) = job_row.map(map_job_row).transpose()? else {
            tx.commit().await.map_err(ApiError::from_db)?;
            return Ok(None);
        };

        job.status = JobStatus::Running;
        job.assigned_worker = Some(worker.worker_id);
        job.started_at = Some(Utc::now());

        sqlx::query(
            r#"
            UPDATE jobs
            SET status = $2, assigned_worker = $3, started_at = $4
            WHERE job_id = $1
            "#,
        )
        .bind(&job.job_id)
        .bind(job_status_to_db(&job.status))
        .bind(&job.assigned_worker)
        .bind(job.started_at)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from_db)?;

        tx.commit().await.map_err(ApiError::from_db)?;
        Ok(Some(job))
    }

    async fn complete_job(
        &self,
        job_id: &str,
        request: JobResultRequest,
    ) -> Result<JobRecord, ApiError> {
        let mut job = self.get_job(job_id).await?;
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
        job.finished_at = Some(Utc::now());

        sqlx::query(
            r#"
            UPDATE jobs
            SET status = $2, result = $3, error = $4, finished_at = $5
            WHERE job_id = $1
            "#,
        )
        .bind(&job.job_id)
        .bind(job_status_to_db(&job.status))
        .bind(&job.result)
        .bind(&job.error)
        .bind(job.finished_at)
        .execute(&self.pool)
        .await
        .map_err(ApiError::from_db)?;

        Ok(job)
    }
}

fn map_job_row(row: sqlx::postgres::PgRow) -> Result<JobRecord, ApiError> {
    Ok(JobRecord {
        job_id: row.try_get("job_id").map_err(ApiError::from_db)?,
        job_kind: parse_job_kind(
            &row.try_get::<String, _>("job_kind")
                .map_err(ApiError::from_db)?,
        )?,
        status: parse_job_status(
            &row.try_get::<String, _>("status")
                .map_err(ApiError::from_db)?,
        )?,
        required_capabilities: row
            .try_get("required_capabilities")
            .map_err(ApiError::from_db)?,
        payload: row.try_get("payload").map_err(ApiError::from_db)?,
        assigned_worker: row.try_get("assigned_worker").map_err(ApiError::from_db)?,
        result: row.try_get("result").map_err(ApiError::from_db)?,
        error: row.try_get("error").map_err(ApiError::from_db)?,
        created_at: row.try_get("created_at").map_err(ApiError::from_db)?,
        started_at: row.try_get("started_at").map_err(ApiError::from_db)?,
        finished_at: row.try_get("finished_at").map_err(ApiError::from_db)?,
    })
}

fn map_worker_row(row: sqlx::postgres::PgRow) -> Result<WorkerRecord, ApiError> {
    Ok(WorkerRecord {
        worker_id: row.try_get("worker_id").map_err(ApiError::from_db)?,
        name: row.try_get("name").map_err(ApiError::from_db)?,
        capabilities: row.try_get("capabilities").map_err(ApiError::from_db)?,
        status: parse_worker_status(
            &row.try_get::<String, _>("status")
                .map_err(ApiError::from_db)?,
        )?,
        last_seen_at: row.try_get("last_seen_at").map_err(ApiError::from_db)?,
    })
}

fn job_kind_to_db(kind: &JobKind) -> &'static str {
    match kind {
        JobKind::DummyInference => "dummy_inference",
    }
}

fn parse_job_kind(value: &str) -> Result<JobKind, ApiError> {
    match value {
        "dummy_inference" => Ok(JobKind::DummyInference),
        _ => Err(ApiError::database_message(format!(
            "unknown job kind stored in database: {value}"
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

fn worker_status_to_db(status: &WorkerStatus) -> &'static str {
    match status {
        WorkerStatus::Online => "online",
        WorkerStatus::Offline => "offline",
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

#[derive(Debug, Error)]
enum ApiErrorKind {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("database")]
    Database,
}

#[derive(Debug, Error)]
#[error("{message}")]
struct ApiError {
    kind: ApiErrorKind,
    message: String,
}

impl ApiError {
    fn not_found(message: String) -> Self {
        Self {
            kind: ApiErrorKind::NotFound,
            message,
        }
    }

    fn conflict(message: String) -> Self {
        Self {
            kind: ApiErrorKind::Conflict,
            message,
        }
    }

    fn database_message(message: String) -> Self {
        Self {
            kind: ApiErrorKind::Database,
            message,
        }
    }

    fn from_db(error: sqlx::Error) -> Self {
        Self {
            kind: ApiErrorKind::Database,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.kind {
            ApiErrorKind::NotFound => StatusCode::NOT_FOUND,
            ApiErrorKind::Conflict => StatusCode::CONFLICT,
            ApiErrorKind::Database => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({ "error": self.message }));
        (status, body).into_response()
    }
}

#[cfg(test)]
#[derive(Default)]
struct MemoryStore {
    jobs: Mutex<HashMap<String, JobRecord>>,
    workers: Mutex<HashMap<String, WorkerRecord>>,
}

#[cfg(test)]
#[async_trait]
impl JobStore for MemoryStore {
    async fn create_job(&self, request: SubmitJobRequest) -> Result<JobRecord, ApiError> {
        let job = JobRecord {
            job_id: Uuid::new_v4().to_string(),
            job_kind: request.job_kind,
            status: JobStatus::Queued,
            required_capabilities: request.required_capabilities,
            payload: request.payload,
            assigned_worker: None,
            result: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        };

        self.jobs
            .lock()
            .await
            .insert(job.job_id.clone(), job.clone());
        Ok(job)
    }

    async fn get_job(&self, id: &str) -> Result<JobRecord, ApiError> {
        self.jobs
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::not_found(format!("job {id} not found")))
    }

    async fn register_worker(&self, request: WorkerRegistration) -> Result<WorkerRecord, ApiError> {
        let worker = WorkerRecord {
            worker_id: request.worker_id,
            name: request.name,
            capabilities: request.capabilities,
            status: WorkerStatus::Online,
            last_seen_at: Utc::now(),
        };

        self.workers
            .lock()
            .await
            .insert(worker.worker_id.clone(), worker.clone());
        Ok(worker)
    }

    async fn claim_job(&self, worker_id: &str) -> Result<Option<JobRecord>, ApiError> {
        let worker = self
            .workers
            .lock()
            .await
            .get(worker_id)
            .cloned()
            .ok_or_else(|| ApiError::not_found(format!("worker {worker_id} not found")))?;

        let mut jobs = self.jobs.lock().await;
        let mut queued: Vec<_> = jobs.values().cloned().collect();
        queued.sort_by_key(|job| job.created_at);

        let Some(mut job) = queued.into_iter().find(|job| {
            job.status == JobStatus::Queued
                && job
                    .required_capabilities
                    .iter()
                    .all(|capability| worker.capabilities.iter().any(|owned| owned == capability))
        }) else {
            return Ok(None);
        };

        job.status = JobStatus::Running;
        job.assigned_worker = Some(worker.worker_id);
        job.started_at = Some(Utc::now());
        jobs.insert(job.job_id.clone(), job.clone());
        Ok(Some(job))
    }

    async fn complete_job(
        &self,
        job_id: &str,
        request: JobResultRequest,
    ) -> Result<JobRecord, ApiError> {
        let mut jobs = self.jobs.lock().await;
        let job = jobs
            .get_mut(job_id)
            .ok_or_else(|| ApiError::not_found(format!("job {job_id} not found")))?;

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
        job.finished_at = Some(Utc::now());

        Ok(job.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn queued_job_is_claimed_and_completed() {
        let store = MemoryStore::default();

        let created = store
            .create_job(SubmitJobRequest {
                job_kind: JobKind::DummyInference,
                required_capabilities: vec!["cpu".to_string()],
                payload: json!({ "prompt": "hello" }),
            })
            .await
            .unwrap();

        store
            .register_worker(WorkerRegistration {
                worker_id: "worker-a".to_string(),
                name: "worker-a".to_string(),
                capabilities: vec!["cpu".to_string()],
            })
            .await
            .unwrap();

        let claimed = store.claim_job("worker-a").await.unwrap().unwrap();
        assert_eq!(claimed.job_id, created.job_id);
        assert_eq!(claimed.status, JobStatus::Running);

        let completed = store
            .complete_job(
                &created.job_id,
                JobResultRequest {
                    worker_id: "worker-a".to_string(),
                    success: true,
                    result: Some(json!({ "message": "ok" })),
                    error: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(completed.status, JobStatus::Completed);
        assert_eq!(completed.result, Some(json!({ "message": "ok" })));
    }

    #[tokio::test]
    async fn unmatched_capability_stays_queued() {
        let store = MemoryStore::default();

        store
            .create_job(SubmitJobRequest {
                job_kind: JobKind::DummyInference,
                required_capabilities: vec!["cuda".to_string()],
                payload: json!({}),
            })
            .await
            .unwrap();

        store
            .register_worker(WorkerRegistration {
                worker_id: "worker-a".to_string(),
                name: "worker-a".to_string(),
                capabilities: vec!["cpu".to_string()],
            })
            .await
            .unwrap();

        let claimed = store.claim_job("worker-a").await.unwrap();
        assert!(claimed.is_none());
    }
}
