//! Top-level error types and result alias for the emojify application.

use thiserror::Error;

use std::path::PathBuf;

/// Top-level application error.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An error originating from the rendering pipeline.
    #[error(transparent)]
    Render(#[from] RenderError),

    /// An error originating from the upload pipeline.
    #[error(transparent)]
    Upload(#[from] UploadError),

    /// An error originating from configuration loading.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// A catch-all error for everything else.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Errors from the rendering pipeline.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The input text or parameters are invalid for rendering.
    #[error("can't render text: {0}")]
    InvalidInput(String),

    /// Failed to load or parse the font file.
    #[error("can't load font: {0}")]
    FontError(String),

    /// Failed to process an image through the `image` crate.
    #[error("can't process image: {0}")]
    ImageError(#[from] image::ImageError),

    /// An I/O error occurred while reading source files.
    #[error("can't read file: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to encode the final output image.
    #[error("can't encode output: {0}")]
    EncodingError(String),

    /// Failed to load or composite an overlay image.
    #[error("can't load overlay: {0}")]
    OverlayError(String),

    /// Failed to apply a gradient effect.
    #[error("can't apply gradient: {0}")]
    GradientError(String),
}

/// Errors from the upload pipeline.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum UploadError {
    /// Authentication with the platform API failed.
    #[error("can't authenticate: {0}")]
    AuthenticationFailed(String),

    /// The file exceeds the platform's maximum allowed size.
    #[error("can't upload: file size {size} exceeds maximum {max}")]
    FileTooLarge {
        /// Actual file size in bytes.
        size: u64,
        /// Maximum allowed size in bytes.
        max: u64,
    },

    /// The platform API returned a non-success status code.
    #[error("can't upload: API returned {status}: {message}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Error message from the API response body.
        message: String,
    },

    /// A network-level error prevented the upload.
    #[error("can't connect: {0}")]
    NetworkError(String),

    /// An I/O error occurred while reading the file to upload.
    #[error("can't upload: {0}")]
    IoError(#[from] std::io::Error),
}

/// Errors from configuration loading.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Failed to read the configuration file from disk.
    #[error("can't read config: {0}")]
    IoError(#[from] std::io::Error),

    /// The configuration file contains invalid TOML or unexpected structure.
    #[error("can't parse config: {0}")]
    ParseError(String),

    /// A configuration value is present but invalid.
    #[error("can't use config value: {0}")]
    InvalidValue(String),

    /// The configuration file has overly permissive file permissions.
    #[error("config file {path} is world-readable, tokens may be exposed")]
    InsecurePermissions {
        /// Path to the insecure configuration file.
        path: PathBuf,
    },
}

/// Convenience result alias used throughout the application.
pub type Result<T> = std::result::Result<T, Error>;
