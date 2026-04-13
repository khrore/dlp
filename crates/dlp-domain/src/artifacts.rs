//! Artifact references.

use std::fmt;

use crate::errors::{DomainError, DomainResult};

/// Reference to an artifact that can be deployed by the control plane.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactRef(String);

impl ArtifactRef {
    /// Validates and stores an artifact reference.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::EmptyValue`] when the provided reference is
    /// blank.
    pub fn new<Value>(value: Value) -> DomainResult<Self>
    where
        Value: Into<String>,
    {
        let artifact_ref = value.into();
        if artifact_ref.trim().is_empty() {
            return Err(DomainError::EmptyValue("artifact_ref"));
        }

        Ok(Self(artifact_ref))
    }

    /// Returns the raw artifact reference string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
