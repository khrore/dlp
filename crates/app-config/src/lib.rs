use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

type ConfigError = Box<figment::Error>;

const DEFAULT_HTTP_SCHEME: &str = "http";
const DEFAULT_LOCALHOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 3000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostPortConfig {
    pub host: IpAddr,
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
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::from((self.host, self.port))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointConfig {
    #[serde(default = "default_http_scheme")]
    pub scheme: String,
    #[serde(default = "default_localhost")]
    pub host:   String,
    #[serde(default = "default_port")]
    pub port:   u16,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            scheme: default_http_scheme(),
            host:   default_localhost(),
            port:   default_port(),
        }
    }
}

impl EndpointConfig {
    pub fn base_url(&self) -> String {
        let host = match self.host.parse::<IpAddr>() {
            Ok(IpAddr::V6(_)) => format!("[{}]", self.host),
            Ok(IpAddr::V4(_)) | Err(_) => self.host.clone(),
        };

        format!("{}://{}:{}", self.scheme, host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ControlPlaneConfig {
    #[serde(default)]
    pub server: HostPortConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DlpConfig {
    #[serde(default)]
    pub api: EndpointConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UiConfig {
    #[serde(default)]
    pub api: EndpointConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
struct RootConfig {
    #[serde(default)]
    control_plane: ControlPlaneConfig,
    #[serde(default)]
    dlp:           DlpConfig,
    #[serde(default)]
    ui:            UiConfig,
}

pub fn load_control_plane_config() -> Result<ControlPlaneConfig, ConfigError> {
    extract_root_config().map(|config| config.control_plane)
}

pub fn load_dlp_config() -> Result<DlpConfig, ConfigError> {
    extract_root_config().map(|config| config.dlp)
}

pub fn load_ui_config() -> Result<UiConfig, ConfigError> {
    extract_root_config().map(|config| config.ui)
}

pub fn load_ui_config_from_dir(start_dir: &Path) -> Result<UiConfig, ConfigError> {
    extract_root_config_from_dir(start_dir).map(|config| config.ui)
}

pub fn find_config_path_from_dir(start_dir: &Path) -> Option<PathBuf> {
    find_config_path(start_dir)
}

fn default_http_scheme() -> String {
    DEFAULT_HTTP_SCHEME.to_string()
}

fn default_localhost() -> String {
    DEFAULT_LOCALHOST.to_string()
}

const fn default_port() -> u16 {
    DEFAULT_PORT
}

fn extract_root_config() -> Result<RootConfig, ConfigError> {
    let current_dir =
        env::current_dir().map_err(|error| Box::new(figment::Error::from(error.to_string())))?;
    extract_root_config_from_dir(&current_dir)
}

fn extract_root_config_from_dir(start_dir: &Path) -> Result<RootConfig, ConfigError> {
    extract_from_figment(base_figment(start_dir))
}

fn base_figment(start_dir: &Path) -> Figment {
    let figment = Figment::from(Serialized::defaults(RootConfig::default()));
    let figment = if let Some(config_path) = find_config_path_from_dir(start_dir) {
        figment.merge(Toml::file(config_path))
    } else {
        figment
    };

    figment
        .merge(env_provider(
            "DLP_CONTROL_PLANE_SERVER_",
            "control_plane.server",
        ))
        .merge(env_provider("DLP_DLP_API_", "dlp.api"))
        .merge(env_provider("DLP_UI_API_", "ui.api"))
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

fn env_provider(prefix: &str, section: &str) -> Env {
    let section = section.to_string();
    Env::prefixed(prefix).map(move |key| {
        let field = key.as_str().to_ascii_lowercase();
        format!("{section}.{field}").into()
    })
}

fn extract_from_figment(figment: Figment) -> Result<RootConfig, ConfigError> {
    figment.extract::<RootConfig>().map_err(Box::new)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        net::{IpAddr, Ipv4Addr},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use figment::{Figment, providers::Serialized};

    use super::{
        ControlPlaneConfig, DlpConfig, EndpointConfig, HostPortConfig, RootConfig, UiConfig,
        extract_from_figment, find_config_path_from_dir, find_config_path_with_override,
    };

    #[test]
    fn endpoint_base_url_uses_structured_fields() {
        let config = EndpointConfig {
            scheme: "https".to_string(),
            host:   "dlp.example.com".to_string(),
            port:   443,
        };

        assert_eq!(config.base_url(), "https://dlp.example.com:443");
    }

    #[test]
    fn endpoint_base_url_preserves_ipv4_formatting() {
        let config = EndpointConfig {
            scheme: "http".to_owned(),
            host:   "127.0.0.1".to_owned(),
            port:   3000,
        };

        assert_eq!(config.base_url(), "http://127.0.0.1:3000");
    }

    #[test]
    fn endpoint_base_url_brackets_ipv6_hosts() {
        let config = EndpointConfig {
            scheme: "http".to_owned(),
            host:   "::1".to_owned(),
            port:   3000,
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
                server: HostPortConfig {
                    host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    port: 4000,
                },
            },
            dlp:           DlpConfig {
                api: EndpointConfig {
                    scheme: "https".to_owned(),
                    host:   "api.example.com".to_owned(),
                    port:   8443,
                },
            },
            ui:            UiConfig::default(),
        };

        let config = extract_from_figment(
            Figment::from(Serialized::defaults(defaults)).merge(Serialized::defaults(overrides)),
        )
        .expect("nested config extracts");

        assert_eq!(config.control_plane.server.port, 4000);
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
            Some(env_config.clone())
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

        assert_eq!(
            find_config_path_from_dir(&nested_dir),
            Some(config_path.clone())
        );

        fs::remove_dir_all(&test_dir).unwrap_or_else(|error| {
            panic!("failed to remove temp dir: {error}");
        });
    }

    fn make_temp_dir(suffix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!("dlp-app-config-{suffix}-{unique}"));
        fs::create_dir_all(&path).unwrap_or_else(|error| {
            panic!("failed to create temp dir: {error}");
        });
        path
    }
}
