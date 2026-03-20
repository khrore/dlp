mod db;

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
use db::SeaOrmStore;
use dlp_shared::{
    ClaimJobResponse, JobId, JobKind, JobRecord, JobResultRequest, JobStatus, SubmitJobRequest,
    WorkerId, WorkerRecord, WorkerRegistration, WorkerStatus,
};
use serde_json::json;
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

    let store = Arc::new(SeaOrmStore::connect(&database_url).await?);
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
pub struct ApiError {
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

    fn from_db(error: impl std::fmt::Display) -> Self {
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
