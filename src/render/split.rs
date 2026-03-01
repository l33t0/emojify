//! Split a source image into a grid of equally sized tiles for multi-emoji
//! compositions, and generate paste-able emoji grid text.

use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};

use std::fmt::Write;

/// Resize `img` to exactly `cols * tile_size` by `rows * tile_size` using
/// Lanczos3, then crop it into individual tiles ordered left-to-right,
/// top-to-bottom.
///
/// Returns a `Vec` of `cols * rows` tiles, each `tile_size x tile_size` pixels.
pub fn split_image(img: DynamicImage, cols: u32, rows: u32, tile_size: u32) -> Vec<RgbaImage> {
    let total_width = cols * tile_size;
    let total_height = rows * tile_size;

    let resized = img
        .resize_exact(total_width, total_height, FilterType::Lanczos3)
        .to_rgba8();

    let mut tiles = Vec::with_capacity((cols * rows) as usize);

    for row in 0..rows {
        for col in 0..cols {
            let x = col * tile_size;
            let y = row * tile_size;
            let tile = image::imageops::crop_imm(&resized, x, y, tile_size, tile_size).to_image();
            tiles.push(tile);
        }
    }

    tiles
}

/// Generate paste-able emoji grid text where each tile is `:name{index}:`
/// arranged in rows separated by newlines.
///
/// Indices are zero-based and zero-padded to 2 digits when the total tile count
/// is under 100, or 3 digits when 100 or more.
pub fn format_emoji_grid(name: &str, cols: u32, rows: u32) -> String {
    let total = cols * rows;
    let pad = if total >= 100 { 3 } else { 2 };

    let mut output = String::new();

    for row in 0..rows {
        if row > 0 {
            output.push('\n');
        }
        for col in 0..cols {
            let index = row * cols + col;
            match pad {
                3 => write!(output, ":{name}{index:03}:").unwrap(),
                _ => write!(output, ":{name}{index:02}:").unwrap(),
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_produces_correct_tile_count() {
        let img = DynamicImage::new_rgba8(200, 100);
        let tiles = split_image(img, 4, 2, 32);
        assert_eq!(tiles.len(), 8);
    }

    #[test]
    fn split_tiles_have_correct_dimensions() {
        let img = DynamicImage::new_rgba8(300, 300);
        let tiles = split_image(img, 3, 3, 64);
        for tile in &tiles {
            assert_eq!(tile.width(), 64);
            assert_eq!(tile.height(), 64);
        }
    }

    #[test]
    fn format_grid_2x2() {
        let grid = format_emoji_grid("test", 2, 2);
        assert_eq!(grid, ":test00::test01:\n:test02::test03:");
    }

    #[test]
    fn format_grid_single_tile() {
        let grid = format_emoji_grid("x", 1, 1);
        assert_eq!(grid, ":x00:");
    }

    #[test]
    fn format_grid_3_digit_padding() {
        // 10 * 10 = 100, should use 3-digit padding
        let grid = format_emoji_grid("big", 10, 10);
        assert!(grid.contains(":big000:"));
        assert!(grid.contains(":big099:"));
    }

    #[test]
    fn format_grid_under_100_uses_2_digits() {
        // 3 * 3 = 9, should use 2-digit padding
        let grid = format_emoji_grid("sm", 3, 3);
        assert!(grid.contains(":sm00:"));
        assert!(grid.contains(":sm08:"));
    }

    #[test]
    fn format_grid_row_layout() {
        let grid = format_emoji_grid("e", 3, 2);
        let lines: Vec<&str> = grid.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], ":e00::e01::e02:");
        assert_eq!(lines[1], ":e03::e04::e05:");
    }
}
