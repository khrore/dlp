use std::{io::Error as IoError, num::TryFromIntError, result::Result as StdResult};

use chrono::OutOfRangeError;
use dlp_config::ConfigError;
use dlp_domain::DomainError;
use sea_orm::{DbErr, TransactionError};

/// Result alias for control-plane operations.
pub type Result<T> = StdResult<T, ControlPlaneError>;

/// Errors produced by the control-plane application and storage layers.
#[derive(Debug, thiserror::Error)]
pub enum ControlPlaneError {
    /// Configuration loading failed.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Domain validation failed.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Database access failed.
    #[error(transparent)]
    Database(#[from] DbErr),
    /// JSON serialization or deserialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Socket or stream IO failed.
    #[error(transparent)]
    Io(#[from] IoError),
    /// Chrono could not represent a duration conversion.
    #[error(transparent)]
    DurationRange(#[from] OutOfRangeError),
    /// A numeric conversion failed.
    #[error(transparent)]
    IntConversion(#[from] TryFromIntError),
    /// `PostgreSQL` storage requires a DSN.
    #[error("control_plane.storage.database_url is required for postgres backend")]
    MissingDatabaseUrl,
    /// A repository query returned no row when a row was required.
    #[error("{operation} returned no row")]
    MissingRow {
        /// The operation or query that unexpectedly returned zero rows.
        operation: &'static str,
    },
    /// An expected persisted entity was not present.
    #[error("unknown {entity}: {id}")]
    UnknownEntity {
        /// The logical entity name.
        entity: &'static str,
        /// The missing identifier.
        id:     String,
    },
    /// A persisted enum or string value could not be converted.
    #[error("invalid {entity}: {value}")]
    InvalidValue {
        /// The value category that failed to parse.
        entity: &'static str,
        /// The invalid string representation.
        value:  String,
    },
    /// A worker row vanished during a heartbeat transaction.
    #[error("worker disappeared during heartbeat: {worker_id}")]
    WorkerDisappearedDuringHeartbeat {
        /// The worker identifier that vanished.
        worker_id: String,
    },
}

impl ControlPlaneError {
    /// Returns the wrapped domain error when this failure originated from the
    /// domain layer.
    #[must_use]
    pub const fn domain_error(&self) -> Option<&DomainError> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Config(_)
            | Self::Database(_)
            | Self::Json(_)
            | Self::Io(_)
            | Self::DurationRange(_)
            | Self::IntConversion(_)
            | Self::MissingDatabaseUrl
            | Self::MissingRow { .. }
            | Self::UnknownEntity { .. }
            | Self::InvalidValue { .. }
            | Self::WorkerDisappearedDuringHeartbeat { .. } => None,
        }
    }
}

impl From<TransactionError<Self>> for ControlPlaneError {
    fn from(error: TransactionError<Self>) -> Self {
        match error {
            TransactionError::Connection(error) => Self::Database(error),
            TransactionError::Transaction(error) => error,
        }
    }
}
