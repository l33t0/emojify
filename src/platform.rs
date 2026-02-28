//! Platform definitions and per-platform constraints.
//!
//! Each target platform (Slack, Discord) has specific limits on image dimensions,
//! file sizes, and supported output formats. This module encodes those constraints.

use serde::{Deserialize, Serialize};

/// Output image format for the generated emoji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OutputFormat {
    /// Portable Network Graphics.
    Png,
    /// WebP image format.
    Webp,
    /// Graphics Interchange Format (animated or static).
    Gif,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Png => write!(formatter, "png"),
            OutputFormat::Webp => write!(formatter, "webp"),
            OutputFormat::Gif => write!(formatter, "gif"),
        }
    }
}

/// Target platform for emoji generation and upload.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Platform {
    /// Slack workspace custom emoji.
    #[default]
    Slack,
    /// Discord server custom emoji.
    Discord,
}

impl Platform {
    /// Maximum image dimension in pixels (width and height).
    pub fn max_dimension(&self) -> u32 {
        match self {
            Platform::Slack | Platform::Discord => 128,
        }
    }

    /// Maximum file size in bytes for the target platform.
    pub fn max_filesize_bytes(&self) -> u64 {
        match self {
            Platform::Slack => 1_000_000,
            Platform::Discord => 262_144, // 256 KiB
        }
    }

    /// Supported output formats for the target platform.
    pub fn supported_formats(&self) -> &[OutputFormat] {
        match self {
            Platform::Slack => &[OutputFormat::Png, OutputFormat::Gif, OutputFormat::Webp],
            Platform::Discord => &[OutputFormat::Png, OutputFormat::Gif, OutputFormat::Webp],
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Slack => write!(formatter, "slack"),
            Platform::Discord => write!(formatter, "discord"),
        }
    }
}
