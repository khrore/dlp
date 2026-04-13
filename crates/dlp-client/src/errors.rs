/// Errors returned by the DLP API client.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClientError {
    /// The control plane returned a non-success HTTP status.
    #[error(
        "request failed with status {code}{suffix}",
        suffix = http_status_suffix(.body, .body_read_error.as_deref())
    )]
    HttpStatus {
        /// HTTP status code returned by the server.
        code:            u16,
        /// Response body returned by the server, if it was read successfully.
        body:            String,
        /// Error encountered while attempting to read the response body.
        body_read_error: Option<String>,
    },
    /// A transport-level failure occurred before a valid response was decoded.
    #[error("transport error: {0}")]
    Transport(String),
}

fn http_status_suffix(body: &str, body_read_error: Option<&str>) -> String {
    if !body.is_empty() {
        return format!(": {body}");
    }
    if let Some(error) = body_read_error {
        return format!(" (failed to read error body: {error})");
    }
    String::new()
}
