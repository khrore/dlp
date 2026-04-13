//! HTTP client for the DLP API.

mod deployments;
mod errors;
mod replicas;
mod transport;
mod workers;

pub use deployments::Client as DeploymentsClient;
pub use errors::ClientError;
pub use replicas::Client as ReplicasClient;
pub use transport::DlpClient;
pub use workers::Client as WorkersClient;
