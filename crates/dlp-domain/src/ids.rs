//! Identifier newtypes.

use std::fmt;

use crate::errors::{DomainError, DomainResult};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new_unchecked(value: String) -> Self {
                Self(value)
            }

            pub fn new(value: impl Into<String>) -> DomainResult<Self> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(DomainError::EmptyValue(stringify!($name)));
                }

                Ok(Self(value))
            }

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

string_id!(DeploymentId);
string_id!(ReplicaId);
string_id!(WorkerId);
string_id!(LeaseId);
