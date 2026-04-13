//! Shared configuration loading for DLP binaries and build scripts.

use std::{
    env,
    io::Error as IoError,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

const DEFAULT_HTTP_SCHEME: &str = "http";
const DEFAULT_LOCALHOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 3000;

/// Error type returned when configuration extraction fails.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Reading the current working directory failed.
    #[error("failed to read current working directory")]
    CurrentDir(#[source] IoError),
    /// Figment could not extract the requested configuration.
    #[error("failed to load configuration")]
    Extract(#[from] Box<figment::Error>),
}

/// Host and port for a socket listener.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostPortConfig {
    /// Host interface to bind.
    pub host: IpAddr,
    /// TCP port to bind.
    pub port: u16,
}

impl Default for HostPortConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_PORT,
        }
    }
}

impl HostPortConfig {
    /// Builds a socket address from the configured host and port.
    #[must_use]
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::from((self.host, self.port))
    }
}

/// Endpoint configuration for HTTP clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointConfig {
    /// Hostname or IP address.
    #[serde(default = "default_localhost")]
    pub host:   String,
    /// TCP port.
    #[serde(default = "default_port")]
    pub port:   u16,
    /// URL scheme.
    #[serde(default = "default_http_scheme")]
    pub scheme: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            host:   default_localhost(),
            port:   default_port(),
            scheme: default_http_scheme(),
        }
    }
}

impl EndpointConfig {
    /// Returns the normalized base URL for the configured endpoint.
    #[must_use]
    pub fn base_url(&self) -> String {
        let host = match self.host.parse::<IpAddr>() {
            Ok(IpAddr::V6(_)) => format!("[{}]", self.host),
            Ok(IpAddr::V4(_)) | Err(_) => self.host.clone(),
        };

        format!("{}://{}:{}", self.scheme, host, self.port)
    }
}

/// Control-plane specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ControlPlaneConfig {
    /// Bind address for the HTTP server.
    pub server:  HostPortConfig,
    /// Durable metadata storage configuration.
    pub storage: ControlPlaneStorageConfig,
}

/// Supported control-plane storage backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend {
    /// In-memory storage, primarily for tests and local-only runs.
    Memory,
    /// PostgreSQL-backed metadata storage.
    #[default]
    Postgres,
}

/// `PostgreSQL` connection pool settings for the control plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PostgresPoolConfig {
    /// Maximum open connections.
    pub max_connections: u32,
    /// Minimum idle connections to keep ready.
    pub min_connections: u32,
}

impl Default for PostgresPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 1,
        }
    }
}

/// Storage settings for the control plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ControlPlaneStorageConfig {
    /// Selected metadata backend.
    pub backend:      StorageBackend,
    /// DSN-first `PostgreSQL` connection string.
    pub database_url: Option<String>,
    /// Connection pool settings.
    pub pool:         PostgresPoolConfig,
}

/// CLI client configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct DlpConfig {
    /// API endpoint for the control plane.
    pub api: EndpointConfig,
}

/// UI configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct UiConfig {
    /// API endpoint for the control plane.
    pub api: EndpointConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
struct RootConfig {
    control_plane: ControlPlaneConfig,
    dlp:           DlpConfig,
    ui:            UiConfig,
}

/// Loads the control-plane server configuration from the current directory
/// context.
///
/// # Errors
///
/// Returns an error if the current working directory cannot be read or if
/// configuration extraction fails.
pub fn load_control_plane_config() -> Result<ControlPlaneConfig, ConfigError> {
    extract_root_config().map(|config| config.control_plane)
}

/// Loads the CLI client configuration from the current directory context.
///
/// # Errors
///
/// Returns an error if the current working directory cannot be read or if
/// configuration extraction fails.
pub fn load_dlp_config() -> Result<DlpConfig, ConfigError> {
    extract_root_config().map(|config| config.dlp)
}

/// Loads the UI configuration from the current directory context.
///
/// # Errors
///
/// Returns an error if the current working directory cannot be read or if
/// configuration extraction fails.
pub fn load_ui_config() -> Result<UiConfig, ConfigError> {
    extract_root_config().map(|config| config.ui)
}

/// Loads the UI configuration starting from an explicit directory.
///
/// # Errors
///
/// Returns an error if configuration extraction fails.
pub fn load_ui_config_from_dir(start_dir: &Path) -> Result<UiConfig, ConfigError> {
    extract_root_config_from_dir(start_dir).map(|config| config.ui)
}

/// Finds the nearest `config.toml` visible from `start_dir`.
#[must_use]
pub fn find_config_path_from_dir(start_dir: &Path) -> Option<PathBuf> {
    find_config_path(start_dir)
}

fn base_figment(start_dir: &Path) -> Figment {
    let defaults = Figment::from(Serialized::defaults(RootConfig::default()));
    let with_file = if let Some(config_path) = find_config_path_from_dir(start_dir) {
        defaults.merge(Toml::file(config_path))
    } else {
        defaults
    };

    with_file
        .merge(env_provider(
            "DLP_CONTROL_PLANE_SERVER_",
            "control_plane.server",
        ))
        .merge(env_provider(
            "DLP_CONTROL_PLANE_STORAGE_",
            "control_plane.storage",
        ))
        .merge(env_provider("DLP_DLP_API_", "dlp.api"))
        .merge(env_provider("DLP_UI_API_", "ui.api"))
}

fn default_http_scheme() -> String {
    DEFAULT_HTTP_SCHEME.to_owned()
}

const fn default_port() -> u16 {
    DEFAULT_PORT
}

fn default_localhost() -> String {
    DEFAULT_LOCALHOST.to_owned()
}

fn env_provider(prefix: &str, section: &str) -> Env {
    let section_name = section.to_owned();
    Env::prefixed(prefix).map(move |key| {
        let field = key.as_str().to_ascii_lowercase();
        format!("{section_name}.{field}").into()
    })
}

fn extract_from_figment(figment: &Figment) -> Result<RootConfig, ConfigError> {
    figment
        .extract::<RootConfig>()
        .map_err(Box::new)
        .map_err(Into::into)
}

fn extract_root_config() -> Result<RootConfig, ConfigError> {
    let current_dir = env::current_dir().map_err(ConfigError::CurrentDir)?;
    extract_root_config_from_dir(&current_dir)
}

fn extract_root_config_from_dir(start_dir: &Path) -> Result<RootConfig, ConfigError> {
    let figment = base_figment(start_dir);
    extract_from_figment(&figment)
}

fn find_config_path(start_dir: &Path) -> Option<PathBuf> {
    find_config_path_with_override(env::var_os("DLP_CONFIG_PATH").map(PathBuf::from), start_dir)
}

fn find_config_path_with_override(
    config_path_override: Option<PathBuf>,
    start_dir: &Path,
) -> Option<PathBuf> {
    if let Some(config_path) = config_path_override {
        return Some(config_path);
    }

    start_dir
        .ancestors()
        .map(|dir| dir.join("config.toml"))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs,
        net::{IpAddr, Ipv4Addr},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use figment::{Figment, providers::Serialized};

    use super::{
        ControlPlaneConfig, ControlPlaneStorageConfig, DlpConfig, EndpointConfig, HostPortConfig,
        PostgresPoolConfig, RootConfig, StorageBackend, UiConfig, extract_from_figment,
        find_config_path_from_dir, find_config_path_with_override,
    };

    #[test]
    fn endpoint_base_url_uses_structured_fields() {
        let config = EndpointConfig {
            host:   "dlp.example.com".to_owned(),
            port:   443,
            scheme: "https".to_owned(),
        };

        assert_eq!(config.base_url(), "https://dlp.example.com:443");
    }

    #[test]
    fn endpoint_base_url_preserves_ipv4_formatting() {
        let config = EndpointConfig {
            host:   "127.0.0.1".to_owned(),
            port:   3000,
            scheme: "http".to_owned(),
        };

        assert_eq!(config.base_url(), "http://127.0.0.1:3000");
    }

    #[test]
    fn endpoint_base_url_brackets_ipv6_hosts() {
        let config = EndpointConfig {
            host:   "::1".to_owned(),
            port:   3000,
            scheme: "http".to_owned(),
        };

        assert_eq!(config.base_url(), "http://[::1]:3000");
    }

    #[test]
    fn host_port_socket_addr_uses_host_and_port() {
        let config = HostPortConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 3000,
        };

        assert_eq!(config.socket_addr().to_string(), "127.0.0.1:3000");
    }

    #[test]
    fn figment_merges_nested_overrides() {
        let defaults = RootConfig::default();
        let overrides = RootConfig {
            control_plane: ControlPlaneConfig {
                server:  HostPortConfig {
                    host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    port: 4000,
                },
                storage: ControlPlaneStorageConfig {
                    backend:      StorageBackend::Memory,
                    database_url: Some("postgres://localhost/dlp".to_owned()),
                    pool:         PostgresPoolConfig {
                        max_connections: 8,
                        min_connections: 2,
                    },
                },
            },
            dlp:           DlpConfig {
                api: EndpointConfig {
                    host:   "api.example.com".to_owned(),
                    port:   8443,
                    scheme: "https".to_owned(),
                },
            },
            ui:            UiConfig::default(),
        };

        let merged =
            Figment::from(Serialized::defaults(defaults)).merge(Serialized::defaults(overrides));
        let config = extract_from_figment(&merged).expect("nested config extracts");

        assert_eq!(config.control_plane.server.port, 4000);
        assert_eq!(config.control_plane.storage.backend, StorageBackend::Memory);
        assert_eq!(
            config.control_plane.storage.database_url.as_deref(),
            Some("postgres://localhost/dlp")
        );
        assert_eq!(config.dlp.api.base_url(), "https://api.example.com:8443");
        assert_eq!(config.ui.api.base_url(), "http://127.0.0.1:3000");
    }

    #[test]
    fn config_path_prefers_environment_override() {
        let test_dir = make_temp_dir("env_override");
        let env_config = test_dir.join("override.toml");
        fs::write(&env_config, "").unwrap_or_else(|error| {
            panic!("failed to create config override fixture: {error}");
        });

        assert_eq!(
            find_config_path_with_override(Some(env_config.clone()), &test_dir),
            Some(env_config)
        );

        fs::remove_dir_all(&test_dir).unwrap_or_else(|error| {
            panic!("failed to remove temp dir: {error}");
        });
    }

    #[test]
    fn config_path_walks_ancestor_directories() {
        let test_dir = make_temp_dir("ancestor_lookup");
        let nested_dir = test_dir.join("nested").join("more");
        let config_path = test_dir.join("config.toml");

        fs::create_dir_all(&nested_dir).unwrap_or_else(|error| {
            panic!("failed to create nested fixture dirs: {error}");
        });
        fs::write(&config_path, "").unwrap_or_else(|error| {
            panic!("failed to create ancestor config fixture: {error}");
        });

        assert_eq!(find_config_path_from_dir(&nested_dir), Some(config_path));

        fs::remove_dir_all(&test_dir).unwrap_or_else(|error| {
            panic!("failed to remove temp dir: {error}");
        });
    }

    fn make_temp_dir(suffix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = temp_dir().join(format!("dlp-app-config-{suffix}-{unique}"));
        fs::create_dir_all(&path).unwrap_or_else(|error| {
            panic!("failed to create temp dir: {error}");
        });
        path
    }
}
