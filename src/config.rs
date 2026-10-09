//! Configuration du service, lue depuis les variables d'environnement `INKPDF_*`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

/// Format des logs sur stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(Self::Json),
            "pretty" => Ok(Self::Pretty),
            other => Err(format!("expected `json` or `pretty`, got `{other}`")),
        }
    }
}

/// Configuration complète du service.
///
/// Les tests construisent cette structure directement (`Config { .., ..Config::default() }`) ;
/// seul `main.rs` passe par [`Config::from_env`].
#[derive(Debug, Clone)]
pub struct Config {
    pub templates_dir: PathBuf,
    pub listen: SocketAddr,
    pub max_body_bytes: usize,
    pub render_timeout: Duration,
    pub max_concurrent_renders: usize,
    pub queue_timeout: Duration,
    pub rescan_interval: Duration,
    pub max_template_bytes: u64,
    pub log_format: LogFormat,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            templates_dir: PathBuf::from("/templates"),
            listen: SocketAddr::from(([0, 0, 0, 0], 3000)),
            max_body_bytes: 5 * 1024 * 1024,
            render_timeout: Duration::from_secs(30),
            max_concurrent_renders: num_cpus::get().max(1),
            queue_timeout: Duration::from_secs(10),
            rescan_interval: Duration::from_secs(2),
            max_template_bytes: 50 * 1024 * 1024,
            log_format: LogFormat::Json,
        }
    }
}

/// Variable d'environnement mal formée.
#[derive(Debug, thiserror::Error)]
#[error("invalid value for {name}: {message}")]
pub struct ConfigError {
    pub name: &'static str,
    pub message: String,
}

impl Config {
    /// Part des valeurs par défaut et applique les variables `INKPDF_*` définies.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Variante testable de [`Config::from_env`] : `lookup` remplace l'environnement.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let var = |name: &'static str| lookup(name).filter(|v| !v.is_empty());

        if let Some(v) = var("INKPDF_TEMPLATES_DIR") {
            config.templates_dir = PathBuf::from(v);
        }
        if let Some(v) = var("INKPDF_LISTEN") {
            config.listen = parse("INKPDF_LISTEN", &v)?;
        }
        if let Some(v) = var("INKPDF_MAX_BODY_BYTES") {
            config.max_body_bytes = parse_positive("INKPDF_MAX_BODY_BYTES", &v)?;
        }
        if let Some(v) = var("INKPDF_RENDER_TIMEOUT_SECS") {
            config.render_timeout = parse_secs("INKPDF_RENDER_TIMEOUT_SECS", &v)?;
        }
        if let Some(v) = var("INKPDF_MAX_CONCURRENT_RENDERS") {
            config.max_concurrent_renders = parse_positive("INKPDF_MAX_CONCURRENT_RENDERS", &v)?;
        }
        if let Some(v) = var("INKPDF_QUEUE_TIMEOUT_SECS") {
            config.queue_timeout = parse_secs("INKPDF_QUEUE_TIMEOUT_SECS", &v)?;
        }
        if let Some(v) = var("INKPDF_RESCAN_INTERVAL_SECS") {
            config.rescan_interval = parse_secs("INKPDF_RESCAN_INTERVAL_SECS", &v)?;
        }
        if let Some(v) = var("INKPDF_MAX_TEMPLATE_BYTES") {
            config.max_template_bytes = parse_positive("INKPDF_MAX_TEMPLATE_BYTES", &v)?;
        }
        if let Some(v) = var("INKPDF_LOG_FORMAT") {
            config.log_format = parse("INKPDF_LOG_FORMAT", &v)?;
        }
        Ok(config)
    }
}

fn parse<T>(name: &'static str, value: &str) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    value.trim().parse().map_err(|e: T::Err| ConfigError {
        name,
        message: format!("`{value}`: {e}"),
    })
}

fn parse_positive<T>(name: &'static str, value: &str) -> Result<T, ConfigError>
where
    T: FromStr + Default + PartialEq,
    T::Err: std::fmt::Display,
{
    let parsed: T = parse(name, value)?;
    if parsed == T::default() {
        return Err(ConfigError {
            name,
            message: "must be greater than 0".into(),
        });
    }
    Ok(parsed)
}

fn parse_secs(name: &'static str, value: &str) -> Result<Duration, ConfigError> {
    parse_positive::<u64>(name, value).map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| vars.get(name).cloned()
    }

    #[test]
    fn defaults_match_the_plan() {
        let config = Config::from_lookup(lookup(&[])).unwrap();
        assert_eq!(config.templates_dir, PathBuf::from("/templates"));
        assert_eq!(config.listen, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(config.max_body_bytes, 5_242_880);
        assert_eq!(config.render_timeout, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_renders, num_cpus::get());
        assert_eq!(config.queue_timeout, Duration::from_secs(10));
        assert_eq!(config.rescan_interval, Duration::from_secs(2));
        assert_eq!(config.max_template_bytes, 52_428_800);
        assert_eq!(config.log_format, LogFormat::Json);
    }

    #[test]
    fn variables_override_defaults() {
        let config = Config::from_lookup(lookup(&[
            ("INKPDF_TEMPLATES_DIR", "/tmp/t"),
            ("INKPDF_LISTEN", "127.0.0.1:8080"),
            ("INKPDF_RENDER_TIMEOUT_SECS", "5"),
            ("INKPDF_LOG_FORMAT", "pretty"),
        ]))
        .unwrap();
        assert_eq!(config.templates_dir, PathBuf::from("/tmp/t"));
        assert_eq!(config.listen.port(), 8080);
        assert_eq!(config.render_timeout, Duration::from_secs(5));
        assert_eq!(config.log_format, LogFormat::Pretty);
    }

    #[test]
    fn malformed_value_is_an_explicit_error() {
        let err = Config::from_lookup(lookup(&[("INKPDF_MAX_BODY_BYTES", "lots")])).unwrap_err();
        assert_eq!(err.name, "INKPDF_MAX_BODY_BYTES");
        assert!(err.to_string().contains("lots"));

        let err = Config::from_lookup(lookup(&[("INKPDF_RENDER_TIMEOUT_SECS", "0")])).unwrap_err();
        assert_eq!(err.name, "INKPDF_RENDER_TIMEOUT_SECS");
    }
}
