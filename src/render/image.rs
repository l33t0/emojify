//! Image loading, resizing, and output encoding.
//!
//! Handles loading images from file paths or raw bytes, resizing them to fit
//! platform constraints while preserving aspect ratio, and encoding final
//! output in PNG, WebP, or GIF format.

use super::gif::{GifOptions, encode_animated_gif};
use crate::error::RenderError;
use crate::platform::{OutputFormat, Platform};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::imageops::FilterType as ResizeFilter;
use image::{DynamicImage, ImageEncoder, RgbaImage};

use std::io::Cursor;
use std::path::Path;

/// Load an image from a file path and resize it to fit within a square canvas.
///
/// The image's aspect ratio is preserved. The longest side is scaled to
/// `canvas_size` and the shorter side is scaled proportionally.
///
/// # Errors
///
/// Returns [`RenderError::IoError`] if the file cannot be read, or
/// [`RenderError::ImageError`] if decoding fails.
pub fn load_and_resize_image(
    path: &Path,
    canvas_size: u32,
) -> std::result::Result<RgbaImage, RenderError> {
    tracing::debug!(path = %path.display(), canvas_size, "loading image from file");
    let img = image::open(path)?;
    Ok(resize_image_to_fit(img, canvas_size))
}

/// Resize a dynamic image so that its longest side equals `canvas_size`, preserving
/// aspect ratio. Uses Lanczos3 filtering for high-quality downscaling.
pub fn resize_image_to_fit(img: DynamicImage, canvas_size: u32) -> RgbaImage {
    let (original_width, original_height) = (img.width(), img.height());

    if original_width == 0 || original_height == 0 {
        return RgbaImage::new(canvas_size, canvas_size);
    }

    let scale = (canvas_size as f64 / original_width as f64)
        .min(canvas_size as f64 / original_height as f64)
        .min(1.0); // never upscale beyond original

    let new_width = ((original_width as f64 * scale).round() as u32).max(1);
    let new_height = ((original_height as f64 * scale).round() as u32).max(1);

    tracing::debug!(
        original_width,
        original_height,
        new_width,
        new_height,
        "resizing image"
    );

    img.resize_exact(new_width, new_height, ResizeFilter::Lanczos3)
        .to_rgba8()
}

/// Load an image from raw bytes and resize it to fit within a square canvas.
///
/// # Errors
///
/// Returns [`RenderError::ImageError`] if the bytes cannot be decoded as a
/// supported image format.
pub fn load_image_from_bytes(
    bytes: &[u8],
    canvas_size: u32,
) -> std::result::Result<RgbaImage, RenderError> {
    tracing::debug!(
        byte_count = bytes.len(),
        canvas_size,
        "loading image from bytes"
    );
    let img = image::load_from_memory(bytes)?;
    Ok(resize_image_to_fit(img, canvas_size))
}

/// Encode an RGBA image into the requested output format and verify that the
/// result satisfies the target platform's file-size constraint.
///
/// For PNG output on Discord (256 KB limit), maximum compression is applied.
/// For GIF output, the image is encoded as a single-frame GIF via the gif
/// module. For WebP, the `image` crate's built-in encoder is used.
///
/// # Errors
///
/// Returns [`RenderError::EncodingError`] if the encoded output exceeds the
/// platform's maximum file size after all compression attempts.
pub fn encode_output(
    img: &RgbaImage,
    format: OutputFormat,
    platform: Platform,
) -> std::result::Result<Vec<u8>, RenderError> {
    let max_size = platform.max_filesize_bytes();

    let encoded = match format {
        OutputFormat::Png => encode_png(img, &platform)?,
        OutputFormat::Gif => {
            let options = GifOptions {
                frame_delay_ms: 0,
                canvas_size: img.width().max(img.height()),
            };
            encode_animated_gif(std::slice::from_ref(img), &options)?
        }
        OutputFormat::Webp => encode_webp(img)?,
    };

    let encoded_size = encoded.len() as u64;
    if encoded_size > max_size {
        return Err(RenderError::EncodingError(format!(
            "encoded {format} output is {encoded_size} bytes, which exceeds the {platform} \
             limit of {max_size} bytes"
        )));
    }

    tracing::debug!(
        format = %format,
        platform = %platform,
        encoded_bytes = encoded_size,
        max_bytes = max_size,
        "encoded output image"
    );

    Ok(encoded)
}

/// Encode an image as PNG. For Discord, use maximum compression.
fn encode_png(img: &RgbaImage, platform: &Platform) -> std::result::Result<Vec<u8>, RenderError> {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);

    let (compression, filter) = match platform {
        Platform::Discord => (CompressionType::Best, FilterType::Adaptive),
        _ => (CompressionType::Default, FilterType::Adaptive),
    };

    let encoder = PngEncoder::new_with_quality(cursor, compression, filter);
    encoder.write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )?;

    Ok(buffer)
}

/// Encode an image as WebP using the `image` crate's built-in encoder.
fn encode_webp(img: &RgbaImage) -> std::result::Result<Vec<u8>, RenderError> {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);

    let encoder = image::codecs::webp::WebPEncoder::new_lossless(cursor);
    encoder.write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )?;

    Ok(buffer)
}
