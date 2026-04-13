//! Domain errors.

use std::{error::Error, fmt};

/// Result alias for domain operations.
pub type DomainResult<T> = Result<T, DomainError>;

/// Domain validation and state-transition failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// A required string field was blank after trimming.
    EmptyValue(&'static str),
    /// An entity attempted to move between incompatible lifecycle states.
    InvalidStateTransition {
        /// The logical entity whose state transition failed.
        entity: &'static str,
        /// The previous state value.
        from:   String,
        /// The requested next state value.
        to:     String,
    },
    /// Two leases conflicted with each other or with the owning entity.
    LeaseConflict(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue(field) => write!(f, "{field} cannot be empty"),
            Self::InvalidStateTransition { entity, from, to } => {
                write!(f, "invalid {entity} state transition from {from} to {to}")
            }
            Self::LeaseConflict(message) => write!(f, "{message}"),
        }
    }
}

#[expect(
    clippy::missing_trait_methods,
    reason = "The default std::error::Error methods are sufficient for this value type."
)]
impl Error for DomainError {}
