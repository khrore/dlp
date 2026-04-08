use std::{
    error::Error,
    fmt::{Display, Formatter},
    str::FromStr,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status:  String,
    pub service: String,
}

impl HealthResponse {
    pub fn ok(service: impl Into<String>) -> Self {
        Self {
            status:  "ok".to_string(),
            service: service.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Framework {
    Pytorch,
    Max,
}

impl Framework {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pytorch => "pytorch",
            Self::Max => "max",
        }
    }
}

impl Display for Framework {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Framework {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pytorch" => Ok(Self::Pytorch),
            "max" => Ok(Self::Max),
            other => Err(ParseEnumError::new("framework", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadMode {
    Training,
    Inference,
}

impl WorkloadMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Inference => "inference",
        }
    }
}

impl Display for WorkloadMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkloadMode {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "training" => Ok(Self::Training),
            "inference" => Ok(Self::Inference),
            other => Err(ParseEnumError::new("mode", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    Cpu,
    Cuda,
    Rocm,
    AppleGpu,
}

impl DeviceClass {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::AppleGpu => "apple-gpu",
        }
    }
}

impl Display for DeviceClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DeviceClass {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cpu" => Ok(Self::Cpu),
            "cuda" => Ok(Self::Cuda),
            "rocm" => Ok(Self::Rocm),
            "apple-gpu" => Ok(Self::AppleGpu),
            other => Err(ParseEnumError::new("device", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerState {
    Starting,
    Ready,
    Draining,
    Unhealthy,
    Lost,
}

impl Display for WorkerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Draining => "draining",
            Self::Unhealthy => "unhealthy",
            Self::Lost => "lost",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaState {
    Pending,
    Assigned,
    Pulling,
    Starting,
    Ready,
    Failed,
    Stopped,
}

impl Display for ReplicaState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Pending => "pending",
            Self::Assigned => "assigned",
            Self::Pulling => "pulling",
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Active,
    Released,
    Expired,
}

impl Display for LeaseState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Active => "active",
            Self::Released => "released",
            Self::Expired => "expired",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCapability {
    pub framework:              Framework,
    pub mode:                   WorkloadMode,
    pub device:                 DeviceClass,
    pub accelerator_runtime:    String,
    pub architecture_family:    String,
    pub available_memory_bytes: u64,
    pub concurrency_slots:      u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkloadRequirement {
    pub framework:                Framework,
    pub mode:                     WorkloadMode,
    pub device:                   DeviceClass,
    pub accelerator_runtime:      String,
    pub architecture_family:      String,
    pub memory_requirement_bytes: u64,
    pub concurrency_requirement:  u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentStatusSummary {
    pub pending_replicas:  u32,
    pub assigned_replicas: u32,
    pub pulling_replicas:  u32,
    pub starting_replicas: u32,
    pub ready_replicas:    u32,
    pub failed_replicas:   u32,
    pub stopped_replicas:  u32,
}

impl DeploymentStatusSummary {
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
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelDeployment {
    pub id:               String,
    pub name:             String,
    pub artifact_ref:     String,
    pub replicas_desired: u32,
    pub requirement:      WorkloadRequirement,
    pub status:           DeploymentStatusSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelReplica {
    pub id:             String,
    pub deployment_id:  String,
    pub worker_id:      Option<String>,
    pub lease_id:       Option<String>,
    pub state:          ReplicaState,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerLease {
    pub id:            String,
    pub worker_id:     String,
    pub deployment_id: String,
    pub replica_id:    String,
    pub state:         LeaseState,
    pub requirement:   WorkloadRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerAssignment {
    pub worker_id:     String,
    pub deployment_id: String,
    pub replica_id:    String,
    pub lease_id:      String,
    pub artifact_ref:  String,
    pub requirement:   WorkloadRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Worker {
    pub id:                String,
    pub display_name:      String,
    pub state:             WorkerState,
    pub capabilities:      Vec<WorkerCapability>,
    pub assigned_replicas: u32,
    pub available_slots:   u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerRequest {
    pub worker_id:    String,
    pub display_name: String,
    pub capabilities: Vec<WorkerCapability>,
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
    pub worker:       Worker,
    pub assignments:  Vec<WorkerAssignment>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateDeploymentRequest {
    pub name:             String,
    pub artifact_ref:     String,
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
pub struct ParseEnumError {
    kind:  &'static str,
    value: String,
}

impl ParseEnumError {
    fn new(kind: &'static str, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }
}

impl Display for ParseEnumError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid {}: {}", self.kind, self.value)
    }
}

impl Error for ParseEnumError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    Transport(String),
    HttpStatus {
        code:            u16,
        body:            String,
        body_read_error: Option<String>,
    },
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl Error for ClientError {}

#[derive(Debug, Clone)]
pub struct DlpClient {
    base_url: String,
}

impl DlpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn health_url(&self) -> String {
        format!("{}/health", self.base_url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.get_json(self.health_url()).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.get_json(self.health_url()).await
    }

    pub fn workers_url(&self) -> String {
        format!("{}/workers", self.base_url)
    }

    pub fn register_worker_url(&self) -> String {
        format!("{}/register", self.workers_url())
    }

    pub fn worker_heartbeat_url(&self, worker_id: &str) -> String {
        format!("{}/{worker_id}/heartbeat", self.workers_url())
    }

    pub fn deployments_url(&self) -> String {
        format!("{}/deployments", self.base_url)
    }

    pub fn deployment_url(&self, deployment_id: &str) -> String {
        format!("{}/{}", self.deployments_url(), deployment_id)
    }

    pub fn replicas_url(&self) -> String {
        format!("{}/replicas", self.base_url)
    }

    pub fn replica_status_url(&self, replica_id: &str) -> String {
        format!("{}/{replica_id}/status", self.replicas_url())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError> {
        self.post_json(self.register_worker_url(), request).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError> {
        self.post_json(self.register_worker_url(), request).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError> {
        self.post_json(self.worker_heartbeat_url(worker_id), request)
            .await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError> {
        self.post_json(self.worker_heartbeat_url(worker_id), request)
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError> {
        self.get_json(self.workers_url()).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError> {
        self.get_json(self.workers_url()).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError> {
        self.post_json(self.deployments_url(), request).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError> {
        self.post_json(self.deployments_url(), request).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError> {
        self.get_json(self.deployment_url(deployment_id)).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError> {
        self.get_json(self.deployment_url(deployment_id)).await
    }

    #[cfg(not(target_arch = "wasm32"))]
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
    pub async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ModelReplica, ClientError> {
        self.post_json(self.replica_status_url(replica_id), request)
            .await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ModelReplica, ClientError> {
        self.post_json(self.replica_status_url(replica_id), request)
            .await
    }
}

fn normalize_base_url(base_url: String) -> String {
    base_url.trim_end_matches('/').to_string()
}

#[cfg(not(target_arch = "wasm32"))]
impl DlpClient {
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

    async fn post_json<Request, Response>(
        &self,
        url: String,
        request: &Request,
    ) -> Result<Response, ClientError>
    where
        Request: Serialize + ?Sized,
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
impl DlpClient {
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
            accelerator_runtime:      "cpu".to_string(),
            architecture_family:      "generic".to_string(),
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
            status:  "ok".to_string(),
            service: "control-plane".to_string(),
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
            id:               "deployment-1".to_string(),
            name:             "trainer".to_string(),
            artifact_ref:     "s3://artifacts/model".to_string(),
            replicas_desired: 1,
            requirement:      sample_requirement(),
            status:           DeploymentStatusSummary {
                pending_replicas: 1,
                ..DeploymentStatusSummary::default()
            },
        };
        let replica = ModelReplica {
            id:             "replica-1".to_string(),
            deployment_id:  deployment.id.clone(),
            worker_id:      Some("worker-1".to_string()),
            lease_id:       Some("lease-1".to_string()),
            state:          ReplicaState::Ready,
            status_message: Some("ready".to_string()),
        };
        let worker = Worker {
            id:                "worker-1".to_string(),
            display_name:      "trainer-1".to_string(),
            state:             WorkerState::Ready,
            capabilities:      vec![WorkerCapability {
                framework:              Framework::Pytorch,
                mode:                   WorkloadMode::Training,
                device:                 DeviceClass::Cpu,
                accelerator_runtime:    "cpu".to_string(),
                architecture_family:    "generic".to_string(),
                available_memory_bytes: 4096,
                concurrency_slots:      2,
            }],
            assigned_replicas: 1,
            available_slots:   1,
        };

        let deployment_json = serde_json::to_string(&deployment);
        let replica_json = serde_json::to_string(&replica);
        let worker_json = serde_json::to_string(&worker);

        assert!(deployment_json.is_ok());
        assert!(replica_json.is_ok());
        assert!(worker_json.is_ok());
    }

    #[test]
    fn serializes_requests() {
        let create_request = CreateDeploymentRequest {
            name:             "deploy".to_string(),
            artifact_ref:     "artifact".to_string(),
            replicas_desired: 1,
            requirement:      sample_requirement(),
        };
        let update_request = UpdateReplicaStatusRequest {
            lease_id:       "lease-1".to_string(),
            state:          ReplicaState::Starting,
            status_message: Some("booting".to_string()),
        };

        let create_json = serde_json::to_string(&create_request);
        let update_json = serde_json::to_string(&update_request);

        assert!(create_json.is_ok());
        assert!(update_json.is_ok());
    }
}
