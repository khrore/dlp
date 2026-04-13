//! Identifier newtypes.

use std::fmt;

use crate::errors::{DomainError, DomainResult};

macro_rules! string_id {
    ($name:ident, $field:literal) => {
        #[doc = concat!("Identifier value for `", stringify!($name), "`.")]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Validates and stores a `", stringify!($name), "` value.")]
            /// # Errors
            ///
            /// Returns [`DomainError::EmptyValue`] when the provided identifier is
            /// blank or [`DomainError::InvalidValue`] when it contains
            /// transport-unsafe characters.
            pub fn new<Value>(value: Value) -> DomainResult<Self>
            where
                Value: Into<String>,
            {
                let identifier = value.into();
                if identifier.trim().is_empty() {
                    return Err(DomainError::EmptyValue($field));
                }
                if !identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
                {
                    return Err(DomainError::InvalidValue {
                        field: $field,
                        value: identifier,
                    });
                }

                Ok(Self(identifier))
            }

            #[doc = concat!("Returns the raw `", stringify!($name), "` string.")]
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(DeploymentId, "deployment_id");
string_id!(ReplicaId, "replica_id");
string_id!(WorkerId, "worker_id");
string_id!(LeaseId, "lease_id");

#[cfg(test)]
mod tests {
    use super::{DeploymentId, LeaseId, ReplicaId, WorkerId};
    use crate::errors::DomainError;

    #[test]
    fn accepts_url_safe_identifier_characters() {
        assert_eq!(
            DeploymentId::new("deployment.alpha").map(|value| value.to_string()),
            Ok("deployment.alpha".to_owned())
        );
        assert_eq!(
            ReplicaId::new("replica_2").map(|value| value.to_string()),
            Ok("replica_2".to_owned())
        );
        assert_eq!(
            WorkerId::new("worker-1").map(|value| value.to_string()),
            Ok("worker-1".to_owned())
        );
        assert_eq!(
            LeaseId::new("lease.3_4-5").map(|value| value.to_string()),
            Ok("lease.3_4-5".to_owned())
        );
    }

    #[test]
    fn rejects_blank_identifiers() {
        assert_eq!(
            WorkerId::new("   "),
            Err(DomainError::EmptyValue("worker_id"))
        );
    }

    #[test]
    fn rejects_transport_unsafe_identifier_characters() {
        for invalid in ["worker/1", "worker?1", "worker#1", "worker 1"] {
            assert_eq!(
                WorkerId::new(invalid),
                Err(DomainError::InvalidValue {
                    field: "worker_id",
                    value: invalid.to_owned(),
                })
            );
        }
    }
}
