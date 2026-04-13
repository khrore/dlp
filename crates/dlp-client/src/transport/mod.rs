//! Shared transport client and endpoint traits.

use serde::{Serialize, de::DeserializeOwned};

use crate::ClientError;

mod deployments;
mod replicas;
mod workers;

pub use self::{
    deployments::Client as DeploymentsClient,
    replicas::Client as ReplicasClient,
    workers::Client as WorkersClient,
};

/// HTTP client used by the CLI, UI, and workers to call the control plane API.
#[derive(Debug, Clone)]
pub struct DlpClient {
    base_url: String,
}

impl DlpClient {
    /// Creates a client bound to the provided API base URL.
    #[must_use]
    pub fn new<BaseUrl>(base_url: BaseUrl) -> Self
    where
        BaseUrl: Into<String>,
    {
        let normalized_base_url = base_url.into();
        Self {
            base_url: normalized_base_url.trim_end_matches('/').to_owned(),
        }
    }

    /// Returns the normalized API base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
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
