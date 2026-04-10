use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    HttpStatus {
        code:            u16,
        body:            String,
        body_read_error: Option<String>,
    },
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

impl Error for ClientError {}
