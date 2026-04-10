//! Shared models and HTTP client helpers for DLP components.
#![doc(hidden)]

use std::{
    error::Error,
    fmt::{Display as FmtDisplay, Formatter, Result as FmtResult},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub service: String,
    pub status:  String,
}

impl HealthResponse {
    #[inline]
    pub fn ok<Service>(service: Service) -> Self
    where
        Service: Into<String>,
    {
        Self {
            service: service.into(),
            status:  "ok".to_owned(),
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum Framework {
    Max,
    Pytorch,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkloadMode {
    Inference,
    Training,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "kebab-case", ascii_case_insensitive)]
pub enum DeviceClass {
    AppleGpu,
    Cpu,
    Cuda,
    Rocm,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkerState {
    Draining,
    Lost,
    Ready,
    Starting,
    Unhealthy,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ReplicaState {
    Assigned,
    Failed,
    Pending,
    Pulling,
    Ready,
    Starting,
    Stopped,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum LeaseState {
    Active,
    Expired,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCapability {
    pub accelerator_runtime:    String,
    pub architecture_family:    String,
    pub available_memory_bytes: u64,
    pub concurrency_slots:      u32,
    pub device:                 DeviceClass,
    pub framework:              Framework,
    pub mode:                   WorkloadMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkloadRequirement {
    pub accelerator_runtime:      String,
    pub architecture_family:      String,
    pub concurrency_requirement:  u32,
    pub device:                   DeviceClass,
    pub framework:                Framework,
    pub memory_requirement_bytes: u64,
    pub mode:                     WorkloadMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentStatusSummary {
    pub assigned_replicas: u32,
    pub failed_replicas:   u32,
    pub pending_replicas:  u32,
    pub pulling_replicas:  u32,
    pub ready_replicas:    u32,
    pub starting_replicas: u32,
    pub stopped_replicas:  u32,
}

impl DeploymentStatusSummary {
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            pending_replicas:  0,
            assigned_replicas: 0,
            pulling_replicas:  0,
            starting_replicas: 0,
            ready_replicas:    0,
            failed_replicas:   0,
            stopped_replicas:  0,
        }
    }
}

impl Default for DeploymentStatusSummary {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelDeployment {
    pub artifact_ref:     String,
    pub id:               String,
    pub name:             String,
    pub replicas_desired: u32,
    pub requirement:      WorkloadRequirement,
    pub status:           DeploymentStatusSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelReplica {
    pub deployment_id:  String,
    pub id:             String,
    pub lease_id:       Option<String>,
    pub state:          ReplicaState,
    pub status_message: Option<String>,
    pub worker_id:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerLease {
    pub deployment_id: String,
    pub id:            String,
    pub replica_id:    String,
    pub requirement:   WorkloadRequirement,
    pub state:         LeaseState,
    pub worker_id:     String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerAssignment {
    pub artifact_ref:  String,
    pub deployment_id: String,
    pub lease_id:      String,
    pub replica_id:    String,
    pub requirement:   WorkloadRequirement,
    pub worker_id:     String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Worker {
    pub assigned_replicas: u32,
    pub available_slots:   u32,
    pub capabilities:      Vec<WorkerCapability>,
    pub display_name:      String,
    pub id:                String,
    pub state:             WorkerState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerRequest {
    pub capabilities: Vec<WorkerCapability>,
    pub display_name: String,
    pub worker_id:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerResponse {
    pub worker: Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeatRequest {
    pub state: WorkerState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeatResponse {
    pub acknowledged: bool,
    pub assignments:  Vec<WorkerAssignment>,
    pub worker:       Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentRequest {
    pub artifact_ref:     String,
    pub name:             String,
    pub replicas_desired: u32,
    pub requirement:      WorkloadRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentResponse {
    pub deployment: ModelDeployment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetDeploymentResponse {
    pub deployment: ModelDeployment,
    pub replicas:   Vec<ModelReplica>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListWorkersResponse {
    pub workers: Vec<Worker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReplicasResponse {
    pub replicas: Vec<ModelReplica>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateReplicaStatusRequest {
    pub lease_id:       String,
    pub state:          ReplicaState,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    HttpStatus {
        code:            u16,
        body:            String,
        body_read_error: Option<String>,
    },
    Transport(String),
}

impl FmtDisplay for ClientError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::HttpStatus {
                code,
                body,
                body_read_error,
            } => {
                if !body.is_empty() {
                    write!(f, "request failed with status {code}: {body}")
                } else if let Some(error) = body_read_error {
                    write!(
                        f,
                        "request failed with status {code} (failed to read error body: {error})"
                    )
                } else {
                    write!(f, "request failed with status {code}")
                }
            }
        }
    }
}

#[expect(
    clippy::missing_trait_methods,
    reason = "The Error trait has default methods that are intentionally inherited."
)]
impl Error for ClientError {}

#[derive(Debug, Clone)]
pub struct DlpClient {
    base_url: String,
}

impl DlpClient {
    #[must_use]
    #[inline]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError> {
        self.post_json(self.deployments_url(), request).await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError> {
        self.post_json(self.deployments_url(), request).await
    }

    #[must_use]
    #[inline]
    pub fn deployment_url(&self, deployment_id: &str) -> String {
        format!("{}/{}", self.deployments_url(), deployment_id)
    }

    #[must_use]
    #[inline]
    pub fn deployments_url(&self) -> String {
        format!("{}/deployments", self.base_url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError> {
        self.get_json(self.deployment_url(deployment_id)).await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError> {
        self.get_json(self.deployment_url(deployment_id)).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn get_json<Response>(&self, url: String) -> Result<Response, ClientError>
    where
        Response: DeserializeOwned,
    {
        let response = reqwest::Client::new()
            .get(url)
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        parse_reqwest_response(response).await
    }

    #[cfg(target_arch = "wasm32")]
    async fn get_json<Response>(&self, url: String) -> Result<Response, ClientError>
    where
        Response: DeserializeOwned,
    {
        let response = gloo_net::http::Request::get(&url)
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        parse_gloo_response(response).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.get_json(self.health_url()).await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.get_json(self.health_url()).await
    }

    #[must_use]
    #[inline]
    pub fn health_url(&self) -> String {
        format!("{}/health", self.base_url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError> {
        self.post_json(self.worker_heartbeat_url(worker_id), request)
            .await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError> {
        self.post_json(self.worker_heartbeat_url(worker_id), request)
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn list_replicas(
        &self,
        deployment_id: Option<&str>,
    ) -> Result<ListReplicasResponse, ClientError> {
        let url = deployment_id.map_or_else(
            || self.replicas_url(),
            |value| format!("{}?deployment_id={value}", self.replicas_url()),
        );
        self.get_json(url).await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn list_replicas(
        &self,
        deployment_id: Option<&str>,
    ) -> Result<ListReplicasResponse, ClientError> {
        let url = deployment_id.map_or_else(
            || self.replicas_url(),
            |value| format!("{}?deployment_id={value}", self.replicas_url()),
        );
        self.get_json(url).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError> {
        self.get_json(self.workers_url()).await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError> {
        self.get_json(self.workers_url()).await
    }

    #[inline]
    pub fn new<BaseUrl>(base_url: BaseUrl) -> Self
    where
        BaseUrl: Into<String>,
    {
        let normalized_base_url = base_url.into();
        Self {
            base_url: normalize_base_url(&normalized_base_url),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn post_json<Request, Response>(
        &self,
        url: String,
        request: &Request,
    ) -> Result<Response, ClientError>
    where
        Request: Serialize + Sync + ?Sized,
        Response: DeserializeOwned,
    {
        let response = reqwest::Client::new()
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        parse_reqwest_response(response).await
    }

    #[cfg(target_arch = "wasm32")]
    async fn post_json<Request, Response>(
        &self,
        url: String,
        request: &Request,
    ) -> Result<Response, ClientError>
    where
        Request: Serialize + ?Sized,
        Response: DeserializeOwned,
    {
        let request_builder = gloo_net::http::Request::post(&url)
            .json(request)
            .map_err(|error| ClientError::Transport(error.to_string()))?;
        let response = request_builder
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        parse_gloo_response(response).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError> {
        self.post_json(self.register_worker_url(), request).await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError> {
        self.post_json(self.register_worker_url(), request).await
    }

    #[must_use]
    #[inline]
    pub fn register_worker_url(&self) -> String {
        format!("{}/register", self.workers_url())
    }

    #[must_use]
    #[inline]
    pub fn replica_status_url(&self, replica_id: &str) -> String {
        format!("{}/{replica_id}/status", self.replicas_url())
    }

    #[must_use]
    #[inline]
    pub fn replicas_url(&self) -> String {
        format!("{}/replicas", self.base_url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ModelReplica, ClientError> {
        self.post_json(self.replica_status_url(replica_id), request)
            .await
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ModelReplica, ClientError> {
        self.post_json(self.replica_status_url(replica_id), request)
            .await
    }

    #[must_use]
    #[inline]
    pub fn worker_heartbeat_url(&self, worker_id: &str) -> String {
        format!("{}/{worker_id}/heartbeat", self.workers_url())
    }

    #[must_use]
    #[inline]
    pub fn workers_url(&self) -> String {
        format!("{}/workers", self.base_url)
    }
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_owned()
}

#[cfg(not(target_arch = "wasm32"))]
async fn parse_reqwest_response<Response>(
    response: reqwest::Response,
) -> Result<Response, ClientError>
where
    Response: DeserializeOwned,
{
    let status = response.status();
    if !status.is_success() {
        let (body, body_read_error) = match response.text().await {
            Ok(body) => (body, None),
            Err(error) => (String::new(), Some(error.to_string())),
        };
        return Err(ClientError::HttpStatus {
            code: status.as_u16(),
            body,
            body_read_error,
        });
    }

    response
        .json::<Response>()
        .await
        .map_err(|error| ClientError::Transport(error.to_string()))
}

#[cfg(target_arch = "wasm32")]
async fn parse_gloo_response<Response>(
    response: gloo_net::http::Response,
) -> Result<Response, ClientError>
where
    Response: DeserializeOwned,
{
    let status = response.status();
    if !(200..300).contains(&status) {
        let (body, body_read_error) = match response.text().await {
            Ok(body) => (body, None),
            Err(error) => (String::new(), Some(error.to_string())),
        };
        return Err(ClientError::HttpStatus {
            code: status,
            body,
            body_read_error,
        });
    }

    response
        .json::<Response>()
        .await
        .map_err(|error| ClientError::Transport(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        ClientError, CreateDeploymentRequest, DeploymentStatusSummary, DeviceClass, DlpClient,
        Framework, HealthResponse, ModelDeployment, ModelReplica, ReplicaState,
        UpdateReplicaStatusRequest, Worker, WorkerCapability, WorkerState, WorkloadMode,
        WorkloadRequirement,
    };

    fn sample_requirement() -> WorkloadRequirement {
        WorkloadRequirement {
            framework:                Framework::Pytorch,
            mode:                     WorkloadMode::Training,
            device:                   DeviceClass::Cpu,
            accelerator_runtime:      "cpu".to_owned(),
            architecture_family:      "generic".to_owned(),
            memory_requirement_bytes: 1024,
            concurrency_requirement:  1,
        }
    }

    #[test]
    fn trims_trailing_slashes_from_base_url() {
        let client = DlpClient::new("http://127.0.0.1:3000///");
        assert_eq!(client.base_url(), "http://127.0.0.1:3000");
        assert_eq!(client.health_url(), "http://127.0.0.1:3000/health");
        assert_eq!(
            client.register_worker_url(),
            "http://127.0.0.1:3000/workers/register"
        );
    }

    #[test]
    fn health_ok_response_uses_expected_defaults() {
        assert_eq!(HealthResponse::ok("control-plane"), HealthResponse {
            service: "control-plane".to_owned(),
            status:  "ok".to_owned(),
        });
    }

    #[test]
    fn http_status_display_includes_error_body() {
        let error = ClientError::HttpStatus {
            code:            500,
            body:            "backend unavailable".to_owned(),
            body_read_error: None,
        };

        assert_eq!(
            error.to_string(),
            "request failed with status 500: backend unavailable"
        );
    }

    #[test]
    fn http_status_display_reports_body_read_failure() {
        let error = ClientError::HttpStatus {
            code:            502,
            body:            String::new(),
            body_read_error: Some("connection reset".to_owned()),
        };

        assert_eq!(
            error.to_string(),
            "request failed with status 502 (failed to read error body: connection reset)"
        );
    }

    #[test]
    fn parses_and_formats_enums() {
        let framework = "pytorch".parse::<Framework>();
        let mode = "training".parse::<WorkloadMode>();
        let device = "apple-gpu".parse::<DeviceClass>();

        assert_eq!(framework.ok(), Some(Framework::Pytorch));
        assert_eq!(mode.ok(), Some(WorkloadMode::Training));
        assert_eq!(device.ok(), Some(DeviceClass::AppleGpu));
        assert_eq!(Framework::Max.to_string(), "max");
        assert_eq!(WorkloadMode::Inference.to_string(), "inference");
        assert_eq!(DeviceClass::Rocm.to_string(), "rocm");
    }

    #[test]
    fn serializes_shared_models() {
        let deployment = ModelDeployment {
            id:               "deployment-1".to_owned(),
            name:             "trainer".to_owned(),
            artifact_ref:     "s3://artifacts/model".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(),
            status:           DeploymentStatusSummary {
                pending_replicas: 1,
                ..DeploymentStatusSummary::default()
            },
        };
        let replica = ModelReplica {
            id:             "replica-1".to_owned(),
            deployment_id:  deployment.id.clone(),
            worker_id:      Some("worker-1".to_owned()),
            lease_id:       Some("lease-1".to_owned()),
            state:          ReplicaState::Ready,
            status_message: Some("ready".to_owned()),
        };
        let worker = Worker {
            id:                "worker-1".to_owned(),
            display_name:      "trainer-1".to_owned(),
            state:             WorkerState::Ready,
            capabilities:      vec![WorkerCapability {
                framework:              Framework::Pytorch,
                mode:                   WorkloadMode::Training,
                device:                 DeviceClass::Cpu,
                accelerator_runtime:    "cpu".to_owned(),
                architecture_family:    "generic".to_owned(),
                available_memory_bytes: 4096,
                concurrency_slots:      2,
            }],
            assigned_replicas: 1,
            available_slots:   1,
        };

        let deployment_json =
            serde_json::to_string(&deployment).expect("deployment serialization succeeds");
        let replica_json = serde_json::to_string(&replica).expect("replica serialization succeeds");
        let worker_json = serde_json::to_string(&worker).expect("worker serialization succeeds");

        assert!(!deployment_json.is_empty());
        assert!(!replica_json.is_empty());
        assert!(!worker_json.is_empty());
    }

    #[test]
    fn serializes_requests() {
        let create_request = CreateDeploymentRequest {
            name:             "deploy".to_owned(),
            artifact_ref:     "artifact".to_owned(),
            replicas_desired: 1,
            requirement:      sample_requirement(),
        };
        let update_request = UpdateReplicaStatusRequest {
            lease_id:       "lease-1".to_owned(),
            state:          ReplicaState::Starting,
            status_message: Some("booting".to_owned()),
        };

        let create_json =
            serde_json::to_string(&create_request).expect("create request serialization succeeds");
        let update_json =
            serde_json::to_string(&update_request).expect("update request serialization succeeds");

        assert!(!create_json.is_empty());
        assert!(!update_json.is_empty());
    }
}
