//! HTTP client for the DLP API.

pub mod deployments;
pub mod errors;
pub mod replicas;
pub mod transport;
pub mod workers;

pub use deployments::DeploymentsClientExt;
pub use errors::ClientError;
pub use replicas::ReplicasClientExt;
pub use transport::DlpClient;
pub use workers::WorkersClientExt;
