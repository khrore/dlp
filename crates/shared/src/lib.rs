use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type JobId = String;
pub type WorkerId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    DummyInference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    Online,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobRequest {
    pub job_kind: JobKind,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: JobId,
    pub job_kind: JobKind,
    pub status: JobStatus,
    pub required_capabilities: Vec<String>,
    pub payload: Value,
    pub assigned_worker: Option<WorkerId>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistration {
    pub worker_id: WorkerId,
    pub name: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRecord {
    pub worker_id: WorkerId,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: WorkerStatus,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimJobResponse {
    pub job: Option<JobRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultRequest {
    pub worker_id: WorkerId,
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}
