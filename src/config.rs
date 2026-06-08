use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use tracing::Level;

const DEFAULT_APP_HOST: &str = "127.0.0.1";
const DEFAULT_APP_PORT: u16 = 3000;
const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_LOG_LEVEL: Level = Level::DEBUG;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub tracing: TracingConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracingConfig {
    pub level: Level,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingRequiredEnv { name: &'static str },
    InvalidEnvValue { name: &'static str, value: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredEnv { name } => {
                write!(f, "missing required environment variable {name}")
            }
            Self::InvalidEnvValue { name, value } => {
                write!(f, "invalid value for environment variable {name}: {value}")
            }
        }
    }
}

impl Error for ConfigError {}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_map(std::env::vars())
    }

    fn from_env_map(
        values: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, ConfigError> {
        let values = values.into_iter().collect::<HashMap<_, _>>();

        let database_url = required_string(&values, "DATABASE_URL")?;
        let database_max_connections = optional_positive_u32(
            &values,
            "DATABASE_MAX_CONNECTIONS",
            DEFAULT_DATABASE_MAX_CONNECTIONS,
        )?;
        let host = optional_string(&values, "APP_HOST", DEFAULT_APP_HOST);
        let port = optional_u16(&values, "APP_PORT", DEFAULT_APP_PORT)?;
        let log_level = optional_log_level(&values, "LOG_LEVEL", DEFAULT_LOG_LEVEL)?;

        Ok(Self {
            server: ServerConfig { host, port },
            database: DatabaseConfig {
                url: database_url,
                max_connections: database_max_connections,
            },
            tracing: TracingConfig { level: log_level },
        })
    }
}

fn required_string(
    values: &HashMap<String, String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    values
        .get(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or(ConfigError::MissingRequiredEnv { name })
}

fn optional_string(values: &HashMap<String, String>, name: &'static str, default: &str) -> String {
    values
        .get(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn optional_u16(
    values: &HashMap<String, String>,
    name: &'static str,
    default: u16,
) -> Result<u16, ConfigError> {
    match values.get(name).filter(|value| !value.trim().is_empty()) {
        Some(value) => value.parse().map_err(|_| ConfigError::InvalidEnvValue {
            name,
            value: value.clone(),
        }),
        None => Ok(default),
    }
}

fn optional_positive_u32(
    values: &HashMap<String, String>,
    name: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    match values.get(name).filter(|value| !value.trim().is_empty()) {
        Some(value) => {
            let parsed = value.parse().map_err(|_| ConfigError::InvalidEnvValue {
                name,
                value: value.clone(),
            })?;
            if parsed == 0 {
                return Err(ConfigError::InvalidEnvValue {
                    name,
                    value: value.clone(),
                });
            }
            Ok(parsed)
        }
        None => Ok(default),
    }
}

fn optional_log_level(
    values: &HashMap<String, String>,
    name: &'static str,
    default: Level,
) -> Result<Level, ConfigError> {
    match values.get(name).filter(|value| !value.trim().is_empty()) {
        Some(value) => value.parse().map_err(|_| ConfigError::InvalidEnvValue {
            name,
            value: value.clone(),
        }),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn uses_defaults_when_only_database_url_is_provided() {
        let config = AppConfig::from_env_map(env(&[(
            "DATABASE_URL",
            "postgres://postgres:postgres@localhost:5432/postgres_db",
        )]))
        .unwrap();

        assert_eq!(
            config.database.url,
            "postgres://postgres:postgres@localhost:5432/postgres_db"
        );
        assert_eq!(config.database.max_connections, 5);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.addr(), "127.0.0.1:3000");
        assert_eq!(config.tracing.level, tracing::Level::DEBUG);
    }

    #[test]
    fn reads_custom_values() {
        let config = AppConfig::from_env_map(env(&[
            ("DATABASE_URL", "postgres://example"),
            ("DATABASE_MAX_CONNECTIONS", "12"),
            ("APP_HOST", "0.0.0.0"),
            ("APP_PORT", "8080"),
            ("LOG_LEVEL", "info"),
        ]))
        .unwrap();

        assert_eq!(config.database.url, "postgres://example");
        assert_eq!(config.database.max_connections, 12);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.addr(), "0.0.0.0:8080");
        assert_eq!(config.tracing.level, tracing::Level::INFO);
    }

    #[test]
    fn requires_database_url() {
        let error = AppConfig::from_env_map(env(&[])).unwrap_err();

        assert!(error.to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn rejects_invalid_port() {
        let error = AppConfig::from_env_map(env(&[
            ("DATABASE_URL", "postgres://example"),
            ("APP_PORT", "not-a-port"),
        ]))
        .unwrap_err();

        assert!(error.to_string().contains("APP_PORT"));
    }

    #[test]
    fn rejects_non_positive_database_max_connections() {
        let error = AppConfig::from_env_map(env(&[
            ("DATABASE_URL", "postgres://example"),
            ("DATABASE_MAX_CONNECTIONS", "0"),
        ]))
        .unwrap_err();

        assert!(error.to_string().contains("DATABASE_MAX_CONNECTIONS"));
    }

    #[test]
    fn rejects_invalid_log_level() {
        let error = AppConfig::from_env_map(env(&[
            ("DATABASE_URL", "postgres://example"),
            ("LOG_LEVEL", "verbose"),
        ]))
        .unwrap_err();

        assert!(error.to_string().contains("LOG_LEVEL"));
    }
}
