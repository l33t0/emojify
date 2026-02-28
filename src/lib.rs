//! Emojify: generate platform-compatible custom emoji images from text, images,
//! or stdin.
//!
//! This crate provides the core library for the `emojify` CLI tool, including
//! text rendering, image compositing, platform-specific constraints, and upload
//! support for Slack and Discord.

pub mod cli;
pub mod config;
pub mod error;
pub mod platform;
pub mod render;
pub mod tui;
pub mod upload;

pub use error::{Error, Result};
pub use platform::{OutputFormat, Platform};

/// Parse a hex colour string (e.g. `"#FF0000"` or `"FF0000"`) into an RGBA pixel.
///
/// Accepts 6-digit (`RRGGBB`) or 8-digit (`RRGGBBAA`) hex, with optional `#` prefix.
///
/// # Errors
///
/// Returns [`error::RenderError::InvalidInput`] if the string is not valid hex.
pub fn parse_color(value: &str) -> std::result::Result<image::Rgba<u8>, error::RenderError> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    match hex.len() {
        6 => {
            let red = parse_hex_byte(&hex[0..2])?;
            let green = parse_hex_byte(&hex[2..4])?;
            let blue = parse_hex_byte(&hex[4..6])?;
            Ok(image::Rgba([red, green, blue, 255]))
        }
        8 => {
            let red = parse_hex_byte(&hex[0..2])?;
            let green = parse_hex_byte(&hex[2..4])?;
            let blue = parse_hex_byte(&hex[4..6])?;
            let alpha = parse_hex_byte(&hex[6..8])?;
            Ok(image::Rgba([red, green, blue, alpha]))
        }
        _ => Err(error::RenderError::InvalidInput(format!(
            "invalid colour: '{value}' (expected 6 or 8 hex digits)"
        ))),
    }
}

fn parse_hex_byte(hex_pair: &str) -> std::result::Result<u8, error::RenderError> {
    u8::from_str_radix(hex_pair, 16).map_err(|parse_error| {
        error::RenderError::InvalidInput(format!("invalid hex byte '{hex_pair}': {parse_error}"))
    })
}
