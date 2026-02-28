//! Application configuration loaded from `~/.config/emojify/config.toml`.
//!
//! Provides default values that CLI flags can override. Handles sensitive token
//! fields through the [`SecretString`] wrapper to prevent accidental logging.

use crate::error::ConfigError;
use crate::platform::Platform;

use serde::Deserialize;
use tracing::warn;

use std::path::PathBuf;

/// A string wrapper that redacts its contents in `Debug` and `Display` output
/// to prevent accidental exposure of secrets in logs or error messages.
#[derive(Clone)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    /// Create a new secret string from a raw value.
    pub fn new(value: String) -> Self {
        Self { inner: value }
    }

    /// Access the underlying secret value.
    pub fn expose(&self) -> &str {
        &self.inner
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "SecretString(***)")
    }
}

impl std::fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "***")
    }
}

/// Intermediate deserialization target that uses plain strings for token fields
/// before converting them into [`SecretString`] values.
#[derive(Debug, Deserialize)]
struct RawConfig {
    /// Default target platform.
    platform: Option<Platform>,
    /// Default output directory for generated images.
    output_dir: Option<PathBuf>,
    /// Default font size in pixels.
    font_size: Option<u32>,
    /// Slack API token.
    slack_token: Option<String>,
    /// Discord API token.
    discord_token: Option<String>,
}

/// Application configuration with defaults that CLI flags can override.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Default target platform for emoji generation.
    pub platform: Option<Platform>,
    /// Default output directory for generated images.
    pub output_dir: Option<PathBuf>,
    /// Default font size in pixels.
    pub font_size: Option<u32>,
    /// Slack API token (redacted in debug output).
    pub slack_token: Option<SecretString>,
    /// Discord API token (redacted in debug output).
    pub discord_token: Option<SecretString>,
}

impl Config {
    /// Load configuration from `~/.config/emojify/config.toml`.
    ///
    /// Returns a default configuration if the file does not exist. Emits a
    /// tracing warning if the file has world-readable permissions on Unix
    /// systems, since it may contain API tokens.
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = match config_file_path() {
            Some(path) => path,
            None => return Ok(Config::default()),
        };

        if !config_path.exists() {
            return Ok(Config::default());
        }

        #[cfg(unix)]
        check_permissions(&config_path)?;

        let contents = std::fs::read_to_string(&config_path)?;
        let raw: RawConfig = toml::from_str(&contents)
            .map_err(|error| ConfigError::ParseError(error.to_string()))?;

        Ok(Config {
            platform: raw.platform,
            output_dir: raw.output_dir,
            font_size: raw.font_size,
            slack_token: raw.slack_token.map(SecretString::new),
            discord_token: raw.discord_token.map(SecretString::new),
        })
    }
}

/// Resolve the full path to the configuration file.
fn config_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|directory| directory.join("emojify").join("config.toml"))
}

/// On Unix systems, check whether the config file is world-readable and emit
/// a warning if so. Returns an error only if the metadata cannot be read.
#[cfg(unix)]
fn check_permissions(path: &PathBuf) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mode = metadata.permissions().mode();

    // Check if the file is world-readable (others read bit set).
    if mode & 0o004 != 0 {
        warn!(
            path = %path.display(),
            "config file is world-readable; tokens may be exposed"
        );
    }

    Ok(())
}
