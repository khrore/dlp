use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

/// Errors returned by the DLP API client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The control plane returned a non-success HTTP status.
    HttpStatus {
        /// HTTP status code returned by the server.
        code:            u16,
        /// Response body returned by the server, if it was read successfully.
        body:            String,
        /// Error encountered while attempting to read the response body.
        body_read_error: Option<String>,
    },
    /// A transport-level failure occurred before a valid response was decoded.
    Transport(String),
}

impl Display for ClientError {
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
    reason = "The default std::error::Error methods are sufficient for this value type."
)]
impl Error for ClientError {}
