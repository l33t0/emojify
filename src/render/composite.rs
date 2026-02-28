//! Overlay compositing for combining a base image with one or more overlays.
//!
//! Each overlay is positioned according to an [`Anchor`] point and scaled as a
//! fraction of the canvas width. Overlays are alpha-blended onto the base image
//! in the order they appear in the slice.

use crate::error::RenderError;

use image::RgbaImage;
use image::imageops::{FilterType, overlay};

use std::str::FromStr;

/// Anchor point that determines where an overlay is placed on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Top-left corner of the canvas.
    TopLeft,
    /// Top-right corner of the canvas.
    TopRight,
    /// Top edge, horizontally centered.
    TopCenter,
    /// Bottom-left corner of the canvas.
    BottomLeft,
    /// Bottom-right corner of the canvas.
    BottomRight,
    /// Bottom edge, horizontally centered.
    BottomCenter,
    /// Dead center of the canvas.
    Center,
}

impl FromStr for Anchor {
    type Err = RenderError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "top-left" | "topleft" | "tl" => Ok(Anchor::TopLeft),
            "top-right" | "topright" | "tr" => Ok(Anchor::TopRight),
            "top-center" | "topcenter" | "tc" => Ok(Anchor::TopCenter),
            "bottom-left" | "bottomleft" | "bl" => Ok(Anchor::BottomLeft),
            "bottom-right" | "bottomright" | "br" => Ok(Anchor::BottomRight),
            "bottom-center" | "bottomcenter" | "bc" => Ok(Anchor::BottomCenter),
            "center" | "c" => Ok(Anchor::Center),
            _ => Err(RenderError::OverlayError(format!(
                "unknown anchor position '{value}'; expected one of: top-left, top-right, \
                 top-center, bottom-left, bottom-right, bottom-center, center"
            ))),
        }
    }
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Anchor::TopLeft => "top-left",
            Anchor::TopRight => "top-right",
            Anchor::TopCenter => "top-center",
            Anchor::BottomLeft => "bottom-left",
            Anchor::BottomRight => "bottom-right",
            Anchor::BottomCenter => "bottom-center",
            Anchor::Center => "center",
        };
        write!(formatter, "{label}")
    }
}

/// Specification for a single overlay to be composited onto the base image.
#[derive(Debug, Clone)]
pub struct OverlaySpec {
    /// The overlay image to composite.
    pub image: RgbaImage,
    /// Where to anchor the overlay on the canvas.
    pub anchor: Anchor,
    /// Scale factor relative to the canvas width. Clamped to `0.0..=1.0`.
    /// Defaults to `0.4` (40% of canvas width).
    pub scale: f32,
}

impl OverlaySpec {
    /// Default overlay scale: 40% of canvas width.
    pub const DEFAULT_SCALE: f32 = 0.4;
}

/// Composite one or more overlays onto a base image, modifying it in place.
///
/// Each overlay is resized according to its `scale` factor (relative to the
/// base image width), then positioned according to its `anchor`, and finally
/// alpha-blended onto the base.
///
/// # Errors
///
/// Returns [`RenderError::OverlayError`] if the base image has zero dimensions.
pub fn composite(
    base: &mut RgbaImage,
    overlays: &[OverlaySpec],
) -> std::result::Result<(), RenderError> {
    let canvas_width = base.width();
    let canvas_height = base.height();

    if canvas_width == 0 || canvas_height == 0 {
        return Err(RenderError::OverlayError(
            "base image has zero dimensions".to_string(),
        ));
    }

    for (index, spec) in overlays.iter().enumerate() {
        let clamped_scale = spec.scale.clamp(0.0, 1.0);
        let target_size = (canvas_width as f32 * clamped_scale).round() as u32;
        let target_size = target_size.max(1);

        let resized =
            image::imageops::resize(&spec.image, target_size, target_size, FilterType::Lanczos3);

        let overlay_width = resized.width();
        let overlay_height = resized.height();

        let (x, y) = compute_anchor_position(
            canvas_width,
            canvas_height,
            overlay_width,
            overlay_height,
            spec.anchor,
        );

        tracing::debug!(
            overlay_index = index,
            anchor = %spec.anchor,
            scale = clamped_scale,
            overlay_size = target_size,
            x,
            y,
            "compositing overlay"
        );

        overlay(base, &resized, x, y);
    }

    Ok(())
}

/// Compute the top-left (x, y) position for an overlay given the anchor point.
fn compute_anchor_position(
    canvas_width: u32,
    canvas_height: u32,
    overlay_width: u32,
    overlay_height: u32,
    anchor: Anchor,
) -> (i64, i64) {
    let canvas_w = canvas_width as i64;
    let canvas_h = canvas_height as i64;
    let overlay_w = overlay_width as i64;
    let overlay_h = overlay_height as i64;

    match anchor {
        Anchor::TopLeft => (0, 0),
        Anchor::TopRight => (canvas_w - overlay_w, 0),
        Anchor::TopCenter => ((canvas_w - overlay_w) / 2, 0),
        Anchor::BottomLeft => (0, canvas_h - overlay_h),
        Anchor::BottomRight => (canvas_w - overlay_w, canvas_h - overlay_h),
        Anchor::BottomCenter => ((canvas_w - overlay_w) / 2, canvas_h - overlay_h),
        Anchor::Center => ((canvas_w - overlay_w) / 2, (canvas_h - overlay_h) / 2),
    }
}
