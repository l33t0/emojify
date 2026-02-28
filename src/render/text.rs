//! Text-to-image rendering with automatic scaling and multi-line support.
//!
//! Renders arbitrary UTF-8 text onto an RGBA canvas using an embedded font.
//! Text is automatically scaled down to fit within the canvas minus padding,
//! and centered both horizontally and vertically.

use crate::error::RenderError;

use ab_glyph::{Font, FontRef, ScaleFont};
use image::{Rgba, RgbaImage};

/// Raw bytes of the embedded TrueType font.
const FONT_DATA: &[u8] = include_bytes!("../../assets/font.ttf");

/// Configuration for rendering text onto a canvas.
#[derive(Debug, Clone)]
pub struct TextRenderOptions {
    /// The text to render. May contain `\n` for multi-line output.
    pub text: String,
    /// Desired font size in pixels. Will be reduced automatically if the text
    /// does not fit within the canvas.
    pub font_size: f32,
    /// Padding in pixels between the text bounding box and the canvas edges.
    pub padding: u32,
    /// Foreground (text) color.
    pub foreground: Rgba<u8>,
    /// Background color. `None` produces a fully transparent background.
    pub background: Option<Rgba<u8>>,
    /// Width and height of the square output canvas in pixels.
    pub canvas_size: u32,
}

/// Render text to an RGBA image according to the provided options.
///
/// The text is centered on a square canvas. If the rendered text would exceed
/// the available area (canvas minus padding on each side), the font size is
/// iteratively reduced until it fits.
///
/// # Errors
///
/// Returns [`RenderError::InvalidInput`] if the text is empty, or
/// [`RenderError::FontError`] if the embedded font cannot be parsed.
pub fn render_text(options: &TextRenderOptions) -> std::result::Result<RgbaImage, RenderError> {
    if options.text.is_empty() {
        return Err(RenderError::InvalidInput(
            "text must not be empty".to_string(),
        ));
    }

    let font = FontRef::try_from_slice(FONT_DATA).map_err(|error| {
        RenderError::FontError(format!("failed to parse embedded font: {error}"))
    })?;

    let canvas_size = options.canvas_size;
    let available = canvas_size.saturating_sub(options.padding * 2);
    if available == 0 {
        return Err(RenderError::InvalidInput(
            "padding is too large for the canvas size".to_string(),
        ));
    }

    let lines: Vec<&str> = options.text.split('\n').collect();

    // Find the largest font size that fits within the available area.
    let fitted_scale = find_fitting_scale(&font, &lines, options.font_size, available);

    let scaled_font = font.as_scaled(fitted_scale);
    let line_height = scaled_font.height();
    let line_gap = scaled_font.line_gap();
    let total_line_advance = line_height + line_gap;

    // Measure each line's width and compute total text block height.
    let line_widths: Vec<f32> = lines
        .iter()
        .map(|line| measure_line_width(&scaled_font, line))
        .collect();
    let total_text_height =
        line_height * lines.len() as f32 + line_gap * (lines.len().saturating_sub(1)) as f32;

    // Create canvas and fill background.
    let mut canvas = RgbaImage::new(canvas_size, canvas_size);
    if let Some(background_color) = options.background {
        for pixel in canvas.pixels_mut() {
            *pixel = background_color;
        }
    }

    // Vertical start position to center the text block.
    let block_y_start = (canvas_size as f32 - total_text_height) / 2.0 + scaled_font.ascent();

    for (line_index, line) in lines.iter().enumerate() {
        let line_width = line_widths[line_index];
        let x_start = (canvas_size as f32 - line_width) / 2.0;
        let y_baseline = block_y_start + total_line_advance * line_index as f32;

        draw_line_glyphs(
            &mut canvas,
            &font,
            fitted_scale,
            line,
            x_start,
            y_baseline,
            options.foreground,
        );
    }

    tracing::debug!(
        text = %options.text,
        fitted_font_size = %fitted_scale,
        canvas_size = %canvas_size,
        "rendered text to image"
    );

    Ok(canvas)
}

/// Measure the horizontal advance width of a single line of text.
fn measure_line_width<F: Font>(scaled_font: &ab_glyph::PxScaleFont<&F>, line: &str) -> f32 {
    let mut width = 0.0f32;
    let mut previous_glyph_id: Option<ab_glyph::GlyphId> = None;

    for character in line.chars() {
        let glyph_id = scaled_font.glyph_id(character);
        if let Some(previous) = previous_glyph_id {
            width += scaled_font.kern(previous, glyph_id);
        }
        width += scaled_font.h_advance(glyph_id);
        previous_glyph_id = Some(glyph_id);
    }

    width
}

/// Find the largest font scale (<= `max_size`) at which all lines fit within
/// `available` pixels in both width and height.
fn find_fitting_scale<F: Font>(font: &F, lines: &[&str], max_size: f32, available: u32) -> f32 {
    let available_f = available as f32;
    let minimum_size: f32 = 1.0;

    let mut low = minimum_size;
    let mut high = max_size;
    let mut best = minimum_size;

    // Binary search for the largest font size that fits.
    while (high - low) > 0.5 {
        let mid = (low + high) / 2.0;
        if text_fits(font, lines, mid, available_f) {
            best = mid;
            low = mid;
        } else {
            high = mid;
        }
    }

    best
}

/// Check whether text at the given scale fits within the available pixel budget.
fn text_fits<F: Font>(font: &F, lines: &[&str], scale: f32, available: f32) -> bool {
    let scaled = font.as_scaled(scale);
    let line_height = scaled.height();
    let line_gap = scaled.line_gap();
    let total_height =
        line_height * lines.len() as f32 + line_gap * lines.len().saturating_sub(1) as f32;

    if total_height > available {
        return false;
    }

    let max_width = lines
        .iter()
        .map(|line| measure_line_width(&scaled, line))
        .fold(0.0f32, f32::max);

    max_width <= available
}

/// Draw all glyphs for a single line onto the canvas at the given baseline position.
fn draw_line_glyphs(
    canvas: &mut RgbaImage,
    font: &FontRef<'_>,
    scale: f32,
    line: &str,
    x_start: f32,
    y_baseline: f32,
    foreground: Rgba<u8>,
) {
    let scaled_font = font.as_scaled(scale);
    let mut cursor_x = x_start;
    let mut previous_glyph_id: Option<ab_glyph::GlyphId> = None;

    for character in line.chars() {
        let glyph_id = scaled_font.glyph_id(character);

        if let Some(previous) = previous_glyph_id {
            cursor_x += scaled_font.kern(previous, glyph_id);
        }

        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(cursor_x, y_baseline));

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|offset_x, offset_y, coverage| {
                let pixel_x = bounds.min.x as i32 + offset_x as i32;
                let pixel_y = bounds.min.y as i32 + offset_y as i32;

                if pixel_x >= 0
                    && pixel_y >= 0
                    && (pixel_x as u32) < canvas.width()
                    && (pixel_y as u32) < canvas.height()
                {
                    let alpha = (coverage * foreground.0[3] as f32).round() as u8;
                    if alpha > 0 {
                        blend_pixel(canvas, pixel_x as u32, pixel_y as u32, foreground, alpha);
                    }
                }
            });
        }

        cursor_x += scaled_font.h_advance(glyph_id);
        previous_glyph_id = Some(glyph_id);
    }
}

/// Alpha-blend a single foreground pixel onto the canvas.
fn blend_pixel(canvas: &mut RgbaImage, x: u32, y: u32, foreground: Rgba<u8>, alpha: u8) {
    let destination = canvas.get_pixel_mut(x, y);
    let source_alpha = alpha as f32 / 255.0;
    let destination_alpha = destination.0[3] as f32 / 255.0;
    let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    if out_alpha > 0.0 {
        for channel in 0..3 {
            let blended = (foreground.0[channel] as f32 * source_alpha
                + destination.0[channel] as f32 * destination_alpha * (1.0 - source_alpha))
                / out_alpha;
            destination.0[channel] = blended.round() as u8;
        }
        destination.0[3] = (out_alpha * 255.0).round() as u8;
    }
}
