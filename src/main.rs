//! Entry point for the emojify CLI application.
//!
//! Parses command-line arguments, loads configuration, initialises tracing,
//! and dispatches to the appropriate subcommand handler.

use l33t0_emojify::cli::{Arguments, Command, GenerateArguments, SplitArguments, UploadArguments};
use l33t0_emojify::config::{Config, SecretString};
use l33t0_emojify::error::RenderError;
use l33t0_emojify::parse_color;
use l33t0_emojify::platform::OutputFormat;
use l33t0_emojify::render::{
    TextRenderOptions, encode_output, load_and_resize_image, load_image_from_bytes, render_text,
};

use anyhow::{Context, bail};
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use std::io::Read;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let arguments = Arguments::parse();

    setup_tracing();

    let config = match Config::load() {
        Ok(config) => config,
        Err(config_error) => {
            error!(%config_error, "failed to load configuration");
            eprintln!("Error: {config_error}");
            return ExitCode::FAILURE;
        }
    };

    match run(arguments, config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(application_error) => {
            error!(%application_error, "command failed");
            eprintln!("Error: {application_error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Initialise the `tracing` subscriber with an env-filter driven by the
/// `EMOJIFY_LOG` or `RUST_LOG` environment variable.
fn setup_tracing() {
    let filter = EnvFilter::try_from_env("EMOJIFY_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

/// Dispatch the parsed CLI arguments to the correct subcommand handler.
async fn run(arguments: Arguments, config: Config) -> anyhow::Result<()> {
    match arguments.command {
        Command::Generate(generate_arguments) => handle_generate(generate_arguments, &config).await,
        Command::Upload(upload_arguments) => handle_upload(upload_arguments, &config).await,
        Command::Batch(batch_arguments) => handle_batch(batch_arguments, &config).await,
        Command::Split(split_arguments) => handle_split(split_arguments, &config).await,
        Command::Tui => handle_tui(&config).await,
    }
}

/// Determine the output format from an explicit flag or the output file extension.
fn resolve_format(arguments: &GenerateArguments) -> anyhow::Result<OutputFormat> {
    if let Some(format) = arguments.format {
        return Ok(format);
    }

    match arguments
        .output
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("png") => Ok(OutputFormat::Png),
        Some("webp") => Ok(OutputFormat::Webp),
        Some("gif") => Ok(OutputFormat::Gif),
        Some(other) => bail!("unrecognised output extension '.{other}': use --format to specify"),
        None => Ok(OutputFormat::Png),
    }
}

/// Execute the rendering pipeline for a single set of generate arguments and
/// return the rendered RGBA image.
fn render_image(
    arguments: &GenerateArguments,
    config: &Config,
) -> anyhow::Result<image::RgbaImage> {
    let platform = arguments.platform;
    let canvas_size = platform.max_dimension();
    let font_size = config.font_size.unwrap_or(arguments.font_size);

    if let Some(ref text) = arguments.text {
        let foreground = parse_color(&arguments.foreground)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
        let background = if arguments.background == "transparent" {
            None
        } else {
            Some(
                parse_color(&arguments.background)
                    .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?,
            )
        };

        let options = TextRenderOptions {
            text: text.clone(),
            font_size: font_size as f32,
            padding: arguments.padding,
            foreground,
            background,
            canvas_size,
        };

        let image = render_text(&options)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
        Ok(image)
    } else if let Some(ref input_path) = arguments.input {
        let image = load_and_resize_image(input_path, canvas_size)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
        Ok(image)
    } else {
        // --stdin mode
        let mut buffer = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buffer)
            .context("failed to read from stdin")?;
        let image = load_image_from_bytes(&buffer, canvas_size)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
        Ok(image)
    }
}

/// Load overlay images from CLI overlay arguments and convert them to render specs.
fn load_overlay_specs(
    overlays: &[l33t0_emojify::cli::OverlayArg],
    canvas_width: u32,
) -> anyhow::Result<Vec<l33t0_emojify::render::OverlaySpec>> {
    let _ = canvas_width; // Reserved for future scaling logic.
    let mut specs = Vec::with_capacity(overlays.len());

    for overlay_arg in overlays {
        // Try to load as a file path first, then as a solid-color placeholder.
        let overlay_image = if std::path::Path::new(&overlay_arg.emoji).exists() {
            let img = image::open(&overlay_arg.emoji)
                .with_context(|| format!("failed to load overlay image '{}'", overlay_arg.emoji))?;
            img.to_rgba8()
        } else {
            // For non-file overlays (e.g. emoji characters), create a small colored placeholder.
            // A full implementation would fetch Twemoji assets here.
            tracing::warn!(
                emoji = %overlay_arg.emoji,
                "overlay is not a file path; using placeholder"
            );
            let mut placeholder = image::RgbaImage::new(64, 64);
            for pixel in placeholder.pixels_mut() {
                *pixel = image::Rgba([255, 200, 0, 200]);
            }
            placeholder
        };

        let anchor = match overlay_arg.anchor {
            l33t0_emojify::cli::OverlayAnchor::TopLeft => l33t0_emojify::render::Anchor::TopLeft,
            l33t0_emojify::cli::OverlayAnchor::TopRight => l33t0_emojify::render::Anchor::TopRight,
            l33t0_emojify::cli::OverlayAnchor::TopCenter => {
                l33t0_emojify::render::Anchor::TopCenter
            }
            l33t0_emojify::cli::OverlayAnchor::BottomLeft => {
                l33t0_emojify::render::Anchor::BottomLeft
            }
            l33t0_emojify::cli::OverlayAnchor::BottomRight => {
                l33t0_emojify::render::Anchor::BottomRight
            }
            l33t0_emojify::cli::OverlayAnchor::BottomCenter => {
                l33t0_emojify::render::Anchor::BottomCenter
            }
            l33t0_emojify::cli::OverlayAnchor::Center => l33t0_emojify::render::Anchor::Center,
        };

        specs.push(l33t0_emojify::render::OverlaySpec {
            image: overlay_image,
            anchor,
            scale: l33t0_emojify::render::OverlaySpec::DEFAULT_SCALE,
        });
    }

    Ok(specs)
}

/// Handle the `generate` subcommand: render an emoji image from text or an
/// input image, applying overlays, gradients, and platform constraints.
async fn handle_generate(arguments: GenerateArguments, config: &Config) -> anyhow::Result<()> {
    let platform = arguments.platform;

    // Validate that exactly one input source is provided.
    let input_count =
        arguments.text.is_some() as u8 + arguments.input.is_some() as u8 + arguments.stdin as u8;

    if input_count == 0 {
        bail!("no input provided: supply text, --input, or --stdin");
    }
    if input_count > 1 {
        bail!("multiple inputs provided: use only one of text, --input, or --stdin");
    }

    let output_format = resolve_format(&arguments)?;

    info!(
        %platform,
        output = %arguments.output.display(),
        format = %output_format,
        "generating emoji"
    );

    let mut image = render_image(&arguments, config)?;

    // Apply gradient if specified.
    if let Some(ref gradient_spec) = arguments.gradient {
        // Only applies to text rendering (gradient masks onto text alpha).
        let spec = l33t0_emojify::render::GradientSpec::parse(gradient_spec)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
        let gradient_img =
            l33t0_emojify::render::generate_gradient(&spec, image.width(), image.height());
        image = l33t0_emojify::render::apply_gradient_to_text(&image, &gradient_img);
    }

    // Apply overlays if specified.
    if !arguments.overlay.is_empty() {
        let overlay_specs = load_overlay_specs(&arguments.overlay, image.width())?;
        l33t0_emojify::render::composite(&mut image, &overlay_specs)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
    }

    // Handle animated output.
    let encoded = if arguments.animated {
        let gif_options = l33t0_emojify::render::GifOptions {
            frame_delay_ms: 200,
            canvas_size: image.width().max(image.height()),
        };
        l33t0_emojify::render::generate_pulse_animation(&image, &gif_options)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?
    } else {
        encode_output(&image, output_format, platform)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?
    };

    if let Some(parent) = arguments.output.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).context("failed to create output directory")?;
    }

    std::fs::write(&arguments.output, &encoded).context("failed to write output file")?;

    if arguments.json {
        let result = serde_json::json!({
            "status": "ok",
            "output": arguments.output.display().to_string(),
            "platform": platform.to_string(),
            "format": output_format.to_string(),
            "bytes": encoded.len(),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "Wrote {} ({} bytes)",
            arguments.output.display(),
            encoded.len()
        );
    }

    if arguments.preview {
        open_preview(&arguments.output)?;
    }

    Ok(())
}

/// Handle the `upload` subcommand: validate the file and upload it to the
/// target platform.
async fn handle_upload(arguments: UploadArguments, config: &Config) -> anyhow::Result<()> {
    if !arguments.file.exists() {
        bail!("file not found: {}", arguments.file.display());
    }

    let file_size = std::fs::metadata(&arguments.file)?.len();
    let max_size = arguments.platform.max_filesize_bytes();

    if file_size > max_size {
        bail!(
            "file size {} bytes exceeds {} maximum of {} bytes",
            file_size,
            arguments.platform,
            max_size
        );
    }

    // Resolve the token: CLI flag > config file > environment variable.
    let token = resolve_token(&arguments, config)?;

    if arguments.dry_run {
        println!(
            "Dry run: would upload {} as :{}: to {}",
            arguments.file.display(),
            arguments.name,
            arguments.platform,
        );
        return Ok(());
    }

    info!(
        platform = %arguments.platform,
        name = %arguments.name,
        file = %arguments.file.display(),
        "uploading emoji"
    );

    let image_data = std::fs::read(&arguments.file).context("failed to read image file")?;

    let workspace = arguments.workspace.clone().unwrap_or_default();

    match arguments.platform {
        l33t0_emojify::Platform::Slack => {
            let secret = SecretString::new(token);
            l33t0_emojify::upload::upload_to_slack(
                &secret,
                &workspace,
                &arguments.name,
                &image_data,
                false,
            )
            .await
            .map_err(|upload_error| anyhow::anyhow!(upload_error))?;
        }
        l33t0_emojify::Platform::Discord => {
            let secret = SecretString::new(token);
            l33t0_emojify::upload::upload_to_discord(
                &secret,
                &workspace,
                &arguments.name,
                &image_data,
                false,
            )
            .await
            .map_err(|upload_error| anyhow::anyhow!(upload_error))?;
        }
        _ => bail!("unsupported platform for upload"),
    }

    println!("Uploaded :{}: to {}", arguments.name, arguments.platform);

    Ok(())
}

/// Resolve the API token from CLI args, config file, or environment.
fn resolve_token(arguments: &UploadArguments, config: &Config) -> anyhow::Result<String> {
    if let Some(ref token) = arguments.token {
        return Ok(token.clone());
    }

    match arguments.platform {
        l33t0_emojify::Platform::Slack => {
            if let Some(ref secret) = config.slack_token {
                return Ok(secret.expose().to_owned());
            }
            std::env::var("SLACK_TOKEN").context(
                "no Slack token: pass --token, set SLACK_TOKEN, or add slack_token to config",
            )
        }
        l33t0_emojify::Platform::Discord => {
            if let Some(ref secret) = config.discord_token {
                return Ok(secret.expose().to_owned());
            }
            std::env::var("DISCORD_TOKEN").context(
                "no Discord token: pass --token, set DISCORD_TOKEN, or add discord_token to config",
            )
        }
        _ => bail!("unsupported platform for upload"),
    }
}

/// Handle the `split` subcommand: split a source image into a grid of tiles
/// and optionally upload each tile to a platform.
async fn handle_split(arguments: SplitArguments, config: &Config) -> anyhow::Result<()> {
    if !arguments.image.exists() {
        bail!("image not found: {}", arguments.image.display());
    }

    let name = arguments.name.clone().unwrap_or_else(|| {
        arguments
            .image
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("emoji")
            .to_owned()
    });

    let cols = arguments.grid.cols;
    let rows = arguments.grid.rows;
    let platform = arguments.platform;
    let tile_size = platform.max_dimension();

    info!(
        %platform,
        %name,
        cols,
        rows,
        tile_size,
        "splitting image into emoji grid"
    );

    let img = image::open(&arguments.image)
        .with_context(|| format!("failed to open image '{}'", arguments.image.display()))?;

    let tiles = l33t0_emojify::render::split_image(img, cols, rows, tile_size);

    if !arguments.output_dir.exists() {
        std::fs::create_dir_all(&arguments.output_dir)
            .context("failed to create output directory")?;
    }

    let total = (cols * rows) as usize;
    let pad = if total >= 100 { 3 } else { 2 };

    let mut tile_paths = Vec::with_capacity(total);

    for (index, tile) in tiles.iter().enumerate() {
        let encoded = encode_output(tile, OutputFormat::Png, platform)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;

        let filename = match pad {
            3 => format!("{name}{index:03}.png"),
            _ => format!("{name}{index:02}.png"),
        };

        let tile_path = arguments.output_dir.join(&filename);
        std::fs::write(&tile_path, &encoded)
            .with_context(|| format!("failed to write tile '{}'", tile_path.display()))?;

        tile_paths.push(tile_path);
    }

    let grid_text = l33t0_emojify::render::format_emoji_grid(&name, cols, rows);
    let grid_path = arguments.output_dir.join(format!("{name}_grid.txt"));
    std::fs::write(&grid_path, &grid_text)
        .with_context(|| format!("failed to write grid text '{}'", grid_path.display()))?;

    // Upload tiles if requested.
    if arguments.upload {
        let token = resolve_split_token(&arguments, config)?;
        let workspace = arguments.workspace.clone().unwrap_or_default();

        for (index, tile_path) in tile_paths.iter().enumerate() {
            let emoji_name = match pad {
                3 => format!("{name}{index:03}"),
                _ => format!("{name}{index:02}"),
            };

            if arguments.dry_run {
                println!(
                    "Dry run: would upload {} as :{emoji_name}: to {platform}",
                    tile_path.display(),
                );
                continue;
            }

            let image_data = std::fs::read(tile_path).context("failed to read tile for upload")?;

            match platform {
                l33t0_emojify::Platform::Slack => {
                    let secret = SecretString::new(token.clone());
                    l33t0_emojify::upload::upload_to_slack(
                        &secret,
                        &workspace,
                        &emoji_name,
                        &image_data,
                        false,
                    )
                    .await
                    .map_err(|upload_error| anyhow::anyhow!(upload_error))?;
                }
                l33t0_emojify::Platform::Discord => {
                    let secret = SecretString::new(token.clone());
                    l33t0_emojify::upload::upload_to_discord(
                        &secret,
                        &workspace,
                        &emoji_name,
                        &image_data,
                        false,
                    )
                    .await
                    .map_err(|upload_error| anyhow::anyhow!(upload_error))?;
                }
                _ => bail!("unsupported platform for upload"),
            }

            info!(emoji_name, "uploaded tile");
        }
    }

    if arguments.json {
        let result = serde_json::json!({
            "status": "ok",
            "name": name,
            "platform": platform.to_string(),
            "cols": cols,
            "rows": rows,
            "tiles": total,
            "output_dir": arguments.output_dir.display().to_string(),
            "grid_text": grid_text,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "Split into {total} tiles in {}",
            arguments.output_dir.display()
        );
        println!("Grid text written to {}", grid_path.display());
        println!("\n{grid_text}");
    }

    Ok(())
}

/// Resolve the API token for the split subcommand from CLI args, config file,
/// or environment.
fn resolve_split_token(arguments: &SplitArguments, config: &Config) -> anyhow::Result<String> {
    if let Some(ref token) = arguments.token {
        return Ok(token.clone());
    }

    match arguments.platform {
        l33t0_emojify::Platform::Slack => {
            if let Some(ref secret) = config.slack_token {
                return Ok(secret.expose().to_owned());
            }
            std::env::var("SLACK_TOKEN").context(
                "no Slack token: pass --token, set SLACK_TOKEN, or add slack_token to config",
            )
        }
        l33t0_emojify::Platform::Discord => {
            if let Some(ref secret) = config.discord_token {
                return Ok(secret.expose().to_owned());
            }
            std::env::var("DISCORD_TOKEN").context(
                "no Discord token: pass --token, set DISCORD_TOKEN, or add discord_token to config",
            )
        }
        _ => bail!("unsupported platform for upload"),
    }
}

/// Handle the `batch` subcommand: read a spec file and generate each emoji.
async fn handle_batch(
    arguments: l33t0_emojify::cli::BatchArguments,
    config: &Config,
) -> anyhow::Result<()> {
    if !arguments.spec_file.exists() {
        bail!("spec file not found: {}", arguments.spec_file.display());
    }

    info!(
        spec_file = %arguments.spec_file.display(),
        output_dir = %arguments.output_dir.display(),
        "running batch generation"
    );

    let spec_contents =
        std::fs::read_to_string(&arguments.spec_file).context("failed to read spec file")?;

    let entries: Vec<BatchEntry> = toml::from_str::<BatchSpec>(&spec_contents)
        .map_err(|parse_error| anyhow::anyhow!("failed to parse spec file: {parse_error}"))?
        .emoji;

    if !arguments.output_dir.exists() {
        std::fs::create_dir_all(&arguments.output_dir)
            .context("failed to create output directory")?;
    }

    let mut results = Vec::new();
    let platform = arguments.platform;

    for entry in &entries {
        let output_format = entry.format.unwrap_or(OutputFormat::Png);
        let output_path = arguments
            .output_dir
            .join(format!("{}.{output_format}", entry.name));

        let generate_arguments = GenerateArguments {
            text: Some(entry.text.clone()),
            input: None,
            stdin: false,
            platform,
            overlay: Vec::new(),
            output: output_path.clone(),
            format: Some(output_format),
            animated: false,
            preview: false,
            font_size: entry.font_size.unwrap_or(64),
            padding: entry.padding.unwrap_or(8),
            foreground: entry
                .foreground
                .clone()
                .unwrap_or_else(|| "#FFFFFF".to_owned()),
            background: entry
                .background
                .clone()
                .unwrap_or_else(|| "transparent".to_owned()),
            gradient: entry.gradient.clone(),
            json: arguments.json,
        };

        match render_and_write(&generate_arguments, config) {
            Ok(byte_count) => {
                info!(name = %entry.name, output = %output_path.display(), "generated");
                results.push(serde_json::json!({
                    "name": entry.name,
                    "status": "ok",
                    "output": output_path.display().to_string(),
                    "bytes": byte_count,
                }));
            }
            Err(render_error) => {
                error!(name = %entry.name, %render_error, "generation failed");
                results.push(serde_json::json!({
                    "name": entry.name,
                    "status": "error",
                    "error": render_error.to_string(),
                }));
            }
        }
    }

    if arguments.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for result in &results {
            let name = result["name"].as_str().unwrap_or("?");
            let status = result["status"].as_str().unwrap_or("?");
            if status == "ok" {
                let output = result["output"].as_str().unwrap_or("?");
                let bytes = result["bytes"].as_u64().unwrap_or(0);
                println!("{name}: wrote {output} ({bytes} bytes)");
            } else {
                let error_message = result["error"].as_str().unwrap_or("unknown error");
                eprintln!("{name}: {error_message}");
            }
        }
    }

    Ok(())
}

/// Render an image and write the encoded output to disk, returning the byte count.
fn render_and_write(arguments: &GenerateArguments, config: &Config) -> anyhow::Result<usize> {
    let output_format = resolve_format(arguments)?;
    let mut image = render_image(arguments, config)?;

    if let Some(ref gradient_spec) = arguments.gradient {
        let spec = l33t0_emojify::render::GradientSpec::parse(gradient_spec)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;
        let gradient_img =
            l33t0_emojify::render::generate_gradient(&spec, image.width(), image.height());
        image = l33t0_emojify::render::apply_gradient_to_text(&image, &gradient_img);
    }

    let encoded = encode_output(&image, output_format, arguments.platform)
        .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;

    if let Some(parent) = arguments.output.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).context("failed to create output directory")?;
    }

    let byte_count = encoded.len();
    std::fs::write(&arguments.output, &encoded).context("failed to write output file")?;
    Ok(byte_count)
}

/// Handle the `tui` subcommand: launch the interactive terminal user interface.
async fn handle_tui(config: &Config) -> anyhow::Result<()> {
    info!("launching TUI");

    l33t0_emojify::tui::run_tui(config)
        .await
        .context("TUI exited with error")?;

    Ok(())
}

/// Open a file with the system's default viewer for preview.
fn open_preview(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .context("failed to open preview")?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .context("failed to open preview")?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn()
            .context("failed to open preview")?;
    }

    Ok(())
}

/// A single entry in a batch spec file.
#[derive(Debug, Clone, serde::Deserialize)]
struct BatchEntry {
    /// The name used for the output file (without extension).
    name: String,
    /// The text to render.
    text: String,
    /// Optional output format override.
    format: Option<OutputFormat>,
    /// Optional font size override.
    font_size: Option<u32>,
    /// Optional padding override.
    padding: Option<u32>,
    /// Optional foreground colour override.
    foreground: Option<String>,
    /// Optional background colour override.
    background: Option<String>,
    /// Optional gradient specification.
    gradient: Option<String>,
}

/// Top-level batch spec file structure.
#[derive(Debug, Clone, serde::Deserialize)]
struct BatchSpec {
    /// List of emoji entries to generate.
    emoji: Vec<BatchEntry>,
}
