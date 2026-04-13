//! Domain errors.

/// Result alias for domain operations.
pub type DomainResult<T> = Result<T, DomainError>;

/// Domain validation and state-transition failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A required string field was blank after trimming.
    #[error("{0} cannot be empty")]
    EmptyValue(&'static str),
    /// An entity attempted to move between incompatible lifecycle states.
    #[error("invalid {entity} state transition from {from} to {to}")]
    InvalidStateTransition {
        /// The logical entity whose state transition failed.
        entity: &'static str,
        /// The previous state value.
        from:   String,
        /// The requested next state value.
        to:     String,
    },
    /// Two leases conflicted with each other or with the owning entity.
    #[error("{0}")]
    LeaseConflict(String),
}
