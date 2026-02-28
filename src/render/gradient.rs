//! Linear gradient generation and text masking.
//!
//! Parses a gradient spec from `"start_hex:end_hex"` format, generates a
//! top-to-bottom linear gradient image, and can apply the gradient as a
//! foreground colour to rendered text (using the text image's alpha channel
//! as a mask).

use crate::error::RenderError;

use image::{Rgba, RgbaImage};

/// A linear gradient defined by its start and end colours.
#[derive(Debug, Clone)]
pub struct GradientSpec {
    /// Colour at the top of the gradient.
    pub start_color: Rgba<u8>,
    /// Colour at the bottom of the gradient.
    pub end_color: Rgba<u8>,
}

impl GradientSpec {
    /// Parse a gradient specification from the format `"start_hex:end_hex"`.
    ///
    /// Each hex component may optionally start with `#`. Both 6-digit (`RRGGBB`)
    /// and 8-digit (`RRGGBBAA`) hex strings are accepted. When no alpha is
    /// provided, the colour is fully opaque.
    ///
    /// # Examples
    ///
    /// ```text
    /// "FF0000:0000FF"     -> red to blue
    /// "#FF0000:#0000FF"   -> same
    /// "FF000080:0000FF80" -> semi-transparent red to semi-transparent blue
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::GradientError`] if the format is invalid or the
    /// hex values cannot be parsed.
    pub fn parse(spec: &str) -> std::result::Result<Self, RenderError> {
        let parts: Vec<&str> = spec.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(RenderError::GradientError(format!(
                "gradient spec must be 'start_hex:end_hex', got '{spec}'"
            )));
        }

        let start_color = crate::parse_color(parts[0].trim()).map_err(|_| {
            RenderError::GradientError(format!("invalid start colour in gradient spec '{spec}'"))
        })?;
        let end_color = crate::parse_color(parts[1].trim()).map_err(|_| {
            RenderError::GradientError(format!("invalid end colour in gradient spec '{spec}'"))
        })?;

        Ok(GradientSpec {
            start_color,
            end_color,
        })
    }
}

/// Generate a linear gradient image (top-to-bottom) of the given dimensions.
///
/// The top row matches `spec.start_color` and the bottom row matches
/// `spec.end_color`, with linear interpolation in between.
pub fn generate_gradient(spec: &GradientSpec, width: u32, height: u32) -> RgbaImage {
    let mut image = RgbaImage::new(width, height);

    if height == 0 || width == 0 {
        return image;
    }

    let max_row = if height > 1 { height - 1 } else { 1 };

    for y in 0..height {
        let fraction = y as f32 / max_row as f32;
        let color = interpolate_color(spec.start_color, spec.end_color, fraction);
        for x in 0..width {
            image.put_pixel(x, y, color);
        }
    }

    image
}

/// Apply a gradient as the foreground colour of text.
///
/// The `text_image` is used as an alpha mask: for each pixel the gradient
/// colour is adopted and the alpha channel is taken from the text image.
/// This produces colourful text on a transparent background.
///
/// Both images should have the same dimensions; if they differ the smaller
/// extent is used.
pub fn apply_gradient_to_text(text_image: &RgbaImage, gradient: &RgbaImage) -> RgbaImage {
    let width = text_image.width().min(gradient.width());
    let height = text_image.height().min(gradient.height());
    let mut output = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let text_pixel = text_image.get_pixel(x, y);
            let gradient_pixel = gradient.get_pixel(x, y);

            // Use the text pixel's alpha as the mask. If the text pixel is
            // transparent the output pixel is also transparent.
            let text_alpha = text_pixel.0[3];
            if text_alpha == 0 {
                continue; // output pixel stays [0,0,0,0]
            }

            // Combine gradient colour with text alpha.
            let combined_alpha = ((gradient_pixel.0[3] as u16 * text_alpha as u16) / 255) as u8;

            output.put_pixel(
                x,
                y,
                Rgba([
                    gradient_pixel.0[0],
                    gradient_pixel.0[1],
                    gradient_pixel.0[2],
                    combined_alpha,
                ]),
            );
        }
    }

    output
}

/// Linearly interpolate between two RGBA colours.
fn interpolate_color(start: Rgba<u8>, end: Rgba<u8>, fraction: f32) -> Rgba<u8> {
    let fraction = fraction.clamp(0.0, 1.0);
    Rgba([
        lerp_u8(start.0[0], end.0[0], fraction),
        lerp_u8(start.0[1], end.0[1], fraction),
        lerp_u8(start.0[2], end.0[2], fraction),
        lerp_u8(start.0[3], end.0[3], fraction),
    ])
}

/// Linearly interpolate between two `u8` values.
fn lerp_u8(start: u8, end: u8, fraction: f32) -> u8 {
    let result = start as f32 + (end as f32 - start as f32) * fraction;
    result.round().clamp(0.0, 255.0) as u8
}
