//! HTTP client for the DLP API.

mod errors;
mod transport;

pub use errors::ClientError;
pub use transport::{DeploymentsClient, DlpClient, ReplicasClient, WorkersClient};
