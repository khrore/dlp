//! Identifier newtypes.

use std::fmt;

use crate::errors::{DomainError, DomainResult};

macro_rules! string_id {
    ($name:ident) => {
        #[doc = concat!("Identifier value for `", stringify!($name), "`.")]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Validates and stores a `", stringify!($name), "` value.")]
            ///
            /// # Errors
            ///
            /// Returns [`DomainError::EmptyValue`] when the provided identifier is blank.
            pub fn new<Value>(value: Value) -> DomainResult<Self>
            where
                Value: Into<String>,
            {
                let identifier = value.into();
                if identifier.trim().is_empty() {
                    return Err(DomainError::EmptyValue(stringify!($name)));
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

string_id!(DeploymentId);
string_id!(ReplicaId);
string_id!(WorkerId);
string_id!(LeaseId);
