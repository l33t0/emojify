//! Integration tests for the emojify rendering pipeline.
//!
//! These tests exercise text rendering, image encoding, overlay compositing,
//! gradient generation, and animated GIF output to verify correctness of the
//! core render module.

use emojify::platform::{OutputFormat, Platform};
use emojify::render::{
    Anchor, GifOptions, GradientSpec, OverlaySpec, TextRenderOptions, composite,
    encode_animated_gif, encode_output, generate_gradient, generate_pulse_animation, render_text,
};
use image::Rgba;

/// Helper to build a basic [`TextRenderOptions`] with sensible defaults for a
/// 128x128 canvas targeting Slack.
fn default_text_options(text: &str) -> TextRenderOptions {
    TextRenderOptions {
        text: text.to_string(),
        font_size: 64.0,
        padding: 8,
        foreground: Rgba([255, 255, 255, 255]),
        background: Some(Rgba([0, 0, 0, 255])),
        canvas_size: 128,
    }
}

#[test]
fn test_text_render_produces_valid_png_slack() {
    let options = default_text_options("Hi");
    let img = render_text(&options).unwrap();

    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);

    let encoded = encode_output(&img, OutputFormat::Png, Platform::Slack).unwrap();

    // PNG magic bytes: 0x89 P N G
    assert!(encoded.len() >= 8, "encoded output too short");
    assert_eq!(&encoded[0..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn test_text_render_produces_valid_png_discord() {
    let options = TextRenderOptions {
        text: "ok".to_string(),
        font_size: 48.0,
        padding: 4,
        foreground: Rgba([255, 0, 0, 255]),
        background: Some(Rgba([0, 0, 0, 255])),
        canvas_size: 128,
    };

    let img = render_text(&options).unwrap();

    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);

    let encoded = encode_output(&img, OutputFormat::Png, Platform::Discord).unwrap();
    assert_eq!(&encoded[0..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn test_discord_output_under_256kb() {
    let options = default_text_options("XL");
    let img = render_text(&options).unwrap();
    let encoded = encode_output(&img, OutputFormat::Png, Platform::Discord).unwrap();

    assert!(
        encoded.len() < 262_144,
        "Discord output is {} bytes, exceeds 256 KiB limit",
        encoded.len()
    );
}

#[test]
fn test_overlay_compositing_dimensions() {
    let mut base = image::RgbaImage::new(128, 128);
    // Fill base with solid blue.
    for pixel in base.pixels_mut() {
        *pixel = Rgba([0, 0, 255, 255]);
    }

    // Create a small red overlay image.
    let mut overlay_img = image::RgbaImage::new(64, 64);
    for pixel in overlay_img.pixels_mut() {
        *pixel = Rgba([255, 0, 0, 255]);
    }

    let overlays = vec![OverlaySpec {
        image: overlay_img,
        anchor: Anchor::Center,
        scale: 0.4,
    }];

    composite(&mut base, &overlays).unwrap();

    // Base dimensions must be unchanged after compositing.
    assert_eq!(base.width(), 128);
    assert_eq!(base.height(), 128);
}

#[test]
fn test_gradient_generation() {
    let spec = GradientSpec {
        start_color: Rgba([255, 0, 0, 255]),
        end_color: Rgba([0, 0, 255, 255]),
    };

    let img = generate_gradient(&spec, 128, 128);

    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);

    // Top-left pixel should be the start colour (red).
    let top_left = img.get_pixel(0, 0);
    assert_eq!(top_left.0[0], 255, "top-left red channel should be 255");
    assert_eq!(top_left.0[2], 0, "top-left blue channel should be 0");

    // Bottom-left pixel should be the end colour (blue).
    let bottom_left = img.get_pixel(0, 127);
    assert_eq!(bottom_left.0[0], 0, "bottom-left red channel should be 0");
    assert_eq!(
        bottom_left.0[2], 255,
        "bottom-left blue channel should be 255"
    );
}

#[test]
fn test_multiline_text_render() {
    let options = TextRenderOptions {
        text: "AB\nCD".to_string(),
        font_size: 32.0,
        padding: 4,
        foreground: Rgba([255, 255, 255, 255]),
        background: Some(Rgba([0, 0, 0, 255])),
        canvas_size: 128,
    };

    let img = render_text(&options).unwrap();

    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);

    // Verify it can be encoded as valid PNG.
    let encoded = encode_output(&img, OutputFormat::Png, Platform::Slack).unwrap();
    assert_eq!(&encoded[0..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn test_transparent_background() {
    let options = TextRenderOptions {
        text: "T".to_string(),
        font_size: 64.0,
        padding: 8,
        foreground: Rgba([255, 255, 255, 255]),
        background: None,
        canvas_size: 128,
    };

    let img = render_text(&options).unwrap();

    // With no background set, pixels where no glyph is drawn should have alpha == 0.
    let has_transparent = img.pixels().any(|pixel| pixel.0[3] == 0);
    assert!(
        has_transparent,
        "expected at least one fully transparent pixel with no background"
    );
}

#[test]
fn test_pulse_animation_gif() {
    let mut base = image::RgbaImage::new(128, 128);
    for pixel in base.pixels_mut() {
        *pixel = Rgba([0, 200, 100, 255]);
    }

    let options = GifOptions {
        frame_delay_ms: 100,
        canvas_size: 128,
    };

    let gif_data = generate_pulse_animation(&base, &options).unwrap();

    // GIF magic bytes: "GIF89a" or "GIF87a"
    assert!(gif_data.len() >= 6, "GIF output too short");
    assert_eq!(&gif_data[0..3], b"GIF");
}

// ---------------------------------------------------------------------------
// Negative / error-path tests
// ---------------------------------------------------------------------------

#[test]
fn test_empty_text_returns_error() {
    let options = TextRenderOptions {
        text: String::new(),
        font_size: 64.0,
        padding: 8,
        foreground: Rgba([255, 255, 255, 255]),
        background: Some(Rgba([0, 0, 0, 255])),
        canvas_size: 128,
    };

    let result = render_text(&options);
    assert!(result.is_err(), "empty text should return an error");
}

#[test]
fn test_excessive_padding_returns_error() {
    let options = TextRenderOptions {
        text: "X".to_string(),
        font_size: 64.0,
        padding: 128, // padding == canvas size, leaves 0 available
        foreground: Rgba([255, 255, 255, 255]),
        background: Some(Rgba([0, 0, 0, 255])),
        canvas_size: 128,
    };

    let result = render_text(&options);
    assert!(
        result.is_err(),
        "padding >= canvas size should return an error"
    );
}

#[test]
fn test_empty_frames_gif_returns_error() {
    let options = GifOptions {
        frame_delay_ms: 100,
        canvas_size: 128,
    };

    let result = encode_animated_gif(&[], &options);
    assert!(result.is_err(), "empty frames should return an error");
}

#[test]
fn test_invalid_gradient_spec_returns_error() {
    let result = GradientSpec::parse("not-a-valid-spec");
    assert!(
        result.is_err(),
        "invalid gradient spec should return an error"
    );
}

#[test]
fn test_composite_zero_dimension_returns_error() {
    let mut base = image::RgbaImage::new(0, 0);
    let overlay_img = image::RgbaImage::new(10, 10);
    let overlays = vec![OverlaySpec {
        image: overlay_img,
        anchor: Anchor::Center,
        scale: 0.4,
    }];

    let result = composite(&mut base, &overlays);
    assert!(
        result.is_err(),
        "zero-dimension base should return an error"
    );
}
