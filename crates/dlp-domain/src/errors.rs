//! Domain errors.

use std::{error::Error, fmt};

/// Result alias for domain operations.
pub type DomainResult<T> = Result<T, DomainError>;

/// Domain validation and state-transition failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyValue(&'static str),
    InvalidStateTransition {
        entity: &'static str,
        from:   String,
        to:     String,
    },
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

impl Error for DomainError {}
