//! Command-line interface definitions using `clap` derive macros.
//!
//! All CLI arguments, subcommands, and value types are defined here and parsed
//! in `main.rs` before being dispatched to the appropriate handler.

use crate::platform::{OutputFormat, Platform};

use clap::{Parser, Subcommand, ValueEnum};

use std::path::PathBuf;

/// Generate platform-compatible custom emoji images from text, images, or stdin.
#[derive(Debug, Clone, Parser)]
#[command(name = "emojify", version, about, long_about = None)]
pub struct Arguments {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Generate an emoji image from text or an input image.
    Generate(GenerateArguments),

    /// Upload a generated emoji image to a platform.
    Upload(UploadArguments),

    /// Batch-generate emoji images from a spec file.
    Batch(BatchArguments),

    /// Split an image into an emoji grid for big-emoji usage.
    Split(SplitArguments),

    /// Launch the interactive terminal UI.
    Tui,
}

/// Arguments for the `generate` subcommand.
#[derive(Debug, Clone, Parser)]
pub struct GenerateArguments {
    /// Text to render as an emoji. Mutually exclusive with --input and --stdin.
    #[arg()]
    pub text: Option<String>,

    /// Path to an input image file to use as the emoji base.
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Read input from stdin (e.g. piped image data).
    #[arg(long, default_value_t = false)]
    pub stdin: bool,

    /// Target platform controlling output dimensions and file size limits.
    #[arg(short, long, default_value = "slack")]
    pub platform: Platform,

    /// Overlay specification in the format `emoji:anchor`. The anchor controls
    /// placement and must be one of: top-left, top-right, top-center,
    /// bottom-left, bottom-right, bottom-center, center. May be specified
    /// up to two times.
    #[arg(short, long, value_name = "EMOJI:ANCHOR", num_args = 1..=2)]
    pub overlay: Vec<OverlayArg>,

    /// Output file path for the generated image.
    #[arg(short = 'O', long, default_value = "./output.png")]
    pub output: PathBuf,

    /// Output image format. Inferred from the output extension if omitted.
    #[arg(short, long)]
    pub format: Option<OutputFormat>,

    /// Generate an animated GIF instead of a static image.
    #[arg(short, long, default_value_t = false)]
    pub animated: bool,

    /// Open a preview of the generated image after rendering.
    #[arg(long, default_value_t = false)]
    pub preview: bool,

    /// Font size in pixels for text rendering.
    #[arg(long, default_value_t = 64)]
    pub font_size: u32,

    /// Padding in pixels around the rendered content.
    #[arg(long, default_value_t = 8)]
    pub padding: u32,

    /// Foreground (text) color as a hex string, e.g. "#FF0000".
    #[arg(long, default_value = "#FFFFFF")]
    pub foreground: String,

    /// Background color as a hex string or "transparent".
    #[arg(long, default_value = "transparent")]
    pub background: String,

    /// Apply a gradient to the text. Format: "color1,color2[,direction]"
    /// where direction is one of: horizontal, vertical, diagonal.
    #[arg(long)]
    pub gradient: Option<String>,

    /// Emit machine-readable JSON output instead of human-friendly text.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// Arguments for the `upload` subcommand.
#[derive(Debug, Clone, Parser)]
pub struct UploadArguments {
    /// Path to the image file to upload.
    #[arg()]
    pub file: PathBuf,

    /// Target platform to upload the emoji to.
    #[arg(short, long)]
    pub platform: Platform,

    /// Name for the custom emoji on the target platform.
    #[arg(short, long)]
    pub name: String,

    /// API token for authentication. Can also be set via environment variables
    /// SLACK_TOKEN or DISCORD_TOKEN depending on the target platform.
    #[arg(short, long)]
    pub token: Option<String>,

    /// Workspace or server identifier (platform-specific).
    #[arg(short, long)]
    pub workspace: Option<String>,

    /// Validate the upload without actually sending the file.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for the `batch` subcommand.
#[derive(Debug, Clone, Parser)]
pub struct BatchArguments {
    /// Path to a TOML spec file describing the batch of emoji to generate.
    #[arg()]
    pub spec_file: PathBuf,

    /// Target platform controlling output dimensions and file size limits.
    #[arg(short, long, default_value = "slack")]
    pub platform: Platform,

    /// Directory to write generated images into.
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,

    /// Emit machine-readable JSON output instead of human-friendly text.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// Arguments for the `split` subcommand.
#[derive(Debug, Clone, Parser)]
pub struct SplitArguments {
    /// Path to the source image to split into tiles.
    #[arg()]
    pub image: PathBuf,

    /// Base name prefix for tile filenames and emoji names.
    /// Defaults to the input filename stem.
    #[arg(short, long)]
    pub name: Option<String>,

    /// Grid dimensions as COLSxROWS (e.g. 5x5, 3x2, 7x4).
    #[arg(short, long, default_value = "5x5")]
    pub grid: GridSpec,

    /// Target platform controlling tile size and file size limits.
    #[arg(short, long, default_value = "slack")]
    pub platform: Platform,

    /// Directory to write tile images and grid text into.
    #[arg(short = 'O', long, default_value = "./output")]
    pub output_dir: PathBuf,

    /// Upload all tiles to the target platform after splitting.
    #[arg(long, default_value_t = false)]
    pub upload: bool,

    /// API token for authentication (only used with --upload).
    #[arg(short, long)]
    pub token: Option<String>,

    /// Workspace or server identifier (only used with --upload).
    #[arg(short, long)]
    pub workspace: Option<String>,

    /// Validate without uploading (only used with --upload).
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Emit machine-readable JSON output.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// Grid dimensions for the `split` subcommand, parsed from a `COLSxROWS` string.
#[derive(Debug, Clone, Copy)]
pub struct GridSpec {
    /// Number of columns in the grid.
    pub cols: u32,
    /// Number of rows in the grid.
    pub rows: u32,
}

impl std::str::FromStr for GridSpec {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (cols_str, rows_str) = value.split_once('x').ok_or_else(|| {
            format!("invalid grid format '{value}': expected COLSxROWS (e.g. 5x5)")
        })?;

        let cols: u32 = cols_str
            .parse()
            .map_err(|_| format!("invalid column count '{cols_str}'"))?;
        let rows: u32 = rows_str
            .parse()
            .map_err(|_| format!("invalid row count '{rows_str}'"))?;

        if cols == 0 || rows == 0 {
            return Err("grid dimensions must be at least 1x1".to_string());
        }

        Ok(GridSpec { cols, rows })
    }
}

/// Anchor position for an overlay on the emoji canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OverlayAnchor {
    /// Top-left corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Top-center edge.
    TopCenter,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom-right corner.
    BottomRight,
    /// Bottom-center edge.
    BottomCenter,
    /// Dead center of the canvas.
    Center,
}

/// A parsed overlay argument consisting of an emoji identifier and its
/// anchor position on the canvas.
#[derive(Debug, Clone)]
pub struct OverlayArg {
    /// The emoji identifier or file path for the overlay image.
    pub emoji: String,
    /// Where to place the overlay on the base image.
    pub anchor: OverlayAnchor,
}

impl std::str::FromStr for OverlayArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (emoji, anchor_str) = value
            .rsplit_once(':')
            .ok_or_else(|| format!("invalid overlay format '{value}': expected 'emoji:anchor'"))?;

        if emoji.is_empty() {
            return Err(format!(
                "invalid overlay format '{value}': emoji part is empty"
            ));
        }

        let anchor = match anchor_str {
            "top-left" => OverlayAnchor::TopLeft,
            "top-right" => OverlayAnchor::TopRight,
            "top-center" => OverlayAnchor::TopCenter,
            "bottom-left" => OverlayAnchor::BottomLeft,
            "bottom-right" => OverlayAnchor::BottomRight,
            "bottom-center" => OverlayAnchor::BottomCenter,
            "center" => OverlayAnchor::Center,
            other => {
                return Err(format!(
                    "invalid overlay anchor '{other}': expected one of top-left, top-right, \
                     top-center, bottom-left, bottom-right, bottom-center, center"
                ));
            }
        };

        Ok(OverlayArg {
            emoji: emoji.to_owned(),
            anchor,
        })
    }
}
