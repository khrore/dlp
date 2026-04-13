use std::{io::Error as IoError, result::Result as StdResult};

use app_config::ConfigError;
use dlp_client::ClientError;

/// Result alias for CLI operations.
pub type Result<T> = StdResult<T, CliError>;

/// Errors produced by the DLP CLI and REPL.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// Config loading failed.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// API client request failed.
    #[error(transparent)]
    Client(#[from] ClientError),
    /// Terminal IO failed.
    #[error(transparent)]
    Io(#[from] IoError),
    /// The interactive command was blank.
    #[error("enter a command")]
    EmptyInteractiveCommand,
    /// The interactive command was not recognized.
    #[error("unknown command: {0}")]
    UnknownInteractiveCommand(String),
}
