//! Animated GIF encoding and pulse animation generation.
//!
//! Provides functions to encode multiple RGBA frames as an animated GIF and
//! to auto-generate a two-frame "pulse" animation from a single image.

use crate::error::RenderError;

use image::codecs::gif::{GifEncoder, Repeat};
use image::imageops::FilterType;
use image::{Delay, Frame, RgbaImage};

use std::io::Cursor;

/// Configuration for animated GIF output.
#[derive(Debug, Clone)]
pub struct GifOptions {
    /// Delay between frames in milliseconds.
    pub frame_delay_ms: u32,
    /// Square canvas size in pixels.
    pub canvas_size: u32,
}

/// Encode a sequence of RGBA frames as an animated GIF.
///
/// Each frame is displayed for [`GifOptions::frame_delay_ms`] milliseconds.
/// The animation loops infinitely.
///
/// # Errors
///
/// Returns [`RenderError::EncodingError`] if the GIF encoder fails, or
/// [`RenderError::InvalidInput`] if the frames slice is empty.
pub fn encode_animated_gif(
    frames: &[RgbaImage],
    options: &GifOptions,
) -> std::result::Result<Vec<u8>, RenderError> {
    if frames.is_empty() {
        return Err(RenderError::InvalidInput(
            "at least one frame is required for GIF encoding".to_string(),
        ));
    }

    // GIF delay is in centiseconds (1/100ths of a second).
    let delay_cs = (options.frame_delay_ms / 10).max(1);
    let delay = Delay::from_numer_denom_ms(delay_cs * 10, 1);

    let mut buffer = Vec::new();
    {
        let cursor = Cursor::new(&mut buffer);
        let mut encoder = GifEncoder::new_with_speed(cursor, 10);
        encoder.set_repeat(Repeat::Infinite).map_err(|error| {
            RenderError::EncodingError(format!("failed to set GIF repeat mode: {error}"))
        })?;

        for (index, rgba_image) in frames.iter().enumerate() {
            let frame = Frame::from_parts(rgba_image.clone(), 0, 0, delay);
            encoder.encode_frame(frame).map_err(|error| {
                RenderError::EncodingError(format!("failed to encode GIF frame {index}: {error}"))
            })?;
        }
    }

    tracing::debug!(
        frame_count = frames.len(),
        frame_delay_ms = options.frame_delay_ms,
        output_bytes = buffer.len(),
        "encoded animated GIF"
    );

    Ok(buffer)
}

/// Generate a two-frame pulse animation from a single image.
///
/// The first frame is the original image at 100% scale, and the second frame
/// is scaled down to 90% and re-centered on a canvas of the original size.
/// This creates a simple "breathing" effect when looped.
///
/// # Errors
///
/// Returns errors from the underlying GIF encoder.
pub fn generate_pulse_animation(
    base: &RgbaImage,
    options: &GifOptions,
) -> std::result::Result<Vec<u8>, RenderError> {
    let width = base.width();
    let height = base.height();

    // Frame 1: original image at full size.
    let frame_full = base.clone();

    // Frame 2: image scaled to 90%, centered on a transparent canvas of the
    // same dimensions.
    let scaled_width = ((width as f32 * 0.9).round() as u32).max(1);
    let scaled_height = ((height as f32 * 0.9).round() as u32).max(1);
    let scaled = image::imageops::resize(base, scaled_width, scaled_height, FilterType::Lanczos3);

    let mut frame_small = RgbaImage::new(width, height);
    let offset_x = (width.saturating_sub(scaled_width)) / 2;
    let offset_y = (height.saturating_sub(scaled_height)) / 2;

    image::imageops::overlay(&mut frame_small, &scaled, offset_x as i64, offset_y as i64);

    tracing::debug!(
        original_size = %format!("{width}x{height}"),
        scaled_size = %format!("{scaled_width}x{scaled_height}"),
        frame_delay_ms = options.frame_delay_ms,
        "generating pulse animation"
    );

    encode_animated_gif(&[frame_full, frame_small], options)
}
