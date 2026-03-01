# Split Subcommand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `split` subcommand that takes an image and splits it into an NxM grid of 128x128 emoji tiles for Slack/Discord big-emoji usage.

**Architecture:** New `split` subcommand in CLI dispatches to a `render::split_image` function that resizes and crops. A `format_emoji_grid` helper generates the paste-able text. Upload integration reuses existing Slack/Discord upload functions behind an `--upload` flag.

**Tech Stack:** image crate (resize/crop), existing encode_output, existing upload module

---

### Task 1: Add split_image to render pipeline

**Files:**
- Create: `src/render/split.rs`
- Modify: `src/render.rs`

**Step 1: Create `src/render/split.rs`**

```rust
//! Image splitting for emoji grid generation.

use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};

/// Split an image into a grid of equally-sized tiles.
///
/// The source image is resized to exactly `cols * tile_size` by `rows * tile_size`
/// then cropped into individual tiles, left-to-right, top-to-bottom.
///
/// Returns a `Vec` of `cols * rows` tiles, each `tile_size x tile_size` pixels.
pub fn split_image(img: DynamicImage, cols: u32, rows: u32, tile_size: u32) -> Vec<RgbaImage> {
    let target_width = cols * tile_size;
    let target_height = rows * tile_size;

    let resized = img
        .resize_exact(target_width, target_height, FilterType::Lanczos3)
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

/// Format the emoji grid text for pasting into Slack/Discord.
///
/// Each tile is referenced as `:name{index:02}:` and tiles are arranged
/// in rows of `cols` width.
pub fn format_emoji_grid(name: &str, cols: u32, rows: u32) -> String {
    let mut grid = String::new();
    let total = cols * rows;
    let pad_width = if total > 100 { 3 } else { 2 };

    for row in 0..rows {
        for col in 0..cols {
            let index = row * cols + col;
            grid.push_str(&format!(":{name}{index:0>pad_width$}:"));
        }
        grid.push('\n');
    }

    grid
}
```

**Step 2: Add module and re-exports to `src/render.rs`**

Add `mod split;` and `pub use split::{split_image, format_emoji_grid};` to the existing re-exports.

**Step 3: Run `cargo check`**

Run: `cargo check`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/render/split.rs src/render.rs
git commit -m "feat(render): add split_image and format_emoji_grid"
```

---

### Task 2: Add render split tests

**Files:**
- Modify: `tests/render_tests.rs`

**Step 1: Add split tests**

Add these tests to `tests/render_tests.rs`:

```rust
use emojify::render::{split_image, format_emoji_grid};

#[test]
fn test_split_image_produces_correct_tile_count() {
    let img = image::DynamicImage::ImageRgba8(image::RgbaImage::new(500, 500));
    let tiles = split_image(img, 5, 5, 128);
    assert_eq!(tiles.len(), 25);
}

#[test]
fn test_split_image_tile_dimensions() {
    let img = image::DynamicImage::ImageRgba8(image::RgbaImage::new(500, 500));
    let tiles = split_image(img, 3, 2, 128);
    assert_eq!(tiles.len(), 6);
    for tile in &tiles {
        assert_eq!(tile.width(), 128);
        assert_eq!(tile.height(), 128);
    }
}

#[test]
fn test_split_image_non_square_grid() {
    let img = image::DynamicImage::ImageRgba8(image::RgbaImage::new(800, 200));
    let tiles = split_image(img, 7, 2, 128);
    assert_eq!(tiles.len(), 14);
    for tile in &tiles {
        assert_eq!(tile.width(), 128);
        assert_eq!(tile.height(), 128);
    }
}

#[test]
fn test_format_emoji_grid_5x5() {
    let grid = format_emoji_grid("cats", 5, 5);
    let lines: Vec<&str> = grid.trim().lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], ":cats00::cats01::cats02::cats03::cats04:");
    assert_eq!(lines[4], ":cats20::cats21::cats22::cats23::cats24:");
}

#[test]
fn test_format_emoji_grid_3x2() {
    let grid = format_emoji_grid("dog", 3, 2);
    let lines: Vec<&str> = grid.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], ":dog00::dog01::dog02:");
    assert_eq!(lines[1], ":dog03::dog04::dog05:");
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: all new + existing tests pass

**Step 3: Commit**

```bash
git add tests/render_tests.rs
git commit -m "test(render): add split_image and format_emoji_grid tests"
```

---

### Task 3: Add SplitArguments to CLI

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add Split variant and SplitArguments struct**

Add to the `Command` enum:

```rust
/// Split an image into an emoji grid for big-emoji usage.
Split(SplitArguments),
```

Add the struct:

```rust
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

/// Grid dimensions parsed from a `COLSxROWS` string.
#[derive(Debug, Clone, Copy)]
pub struct GridSpec {
    pub cols: u32,
    pub rows: u32,
}

impl std::str::FromStr for GridSpec {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (cols_str, rows_str) = value
            .split_once('x')
            .ok_or_else(|| format!("invalid grid format '{value}': expected COLSxROWS (e.g. 5x5)"))?;

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
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: compiles (warning about unused Split variant is fine — wired in next task)

**Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add Split subcommand and GridSpec parser"
```

---

### Task 4: Wire handle_split into main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Add Split dispatch in `run()`**

In the `run()` match, add:

```rust
Command::Split(split_arguments) => handle_split(split_arguments, &config).await,
```

**Step 2: Add `handle_split` function**

```rust
/// Handle the `split` subcommand: split an image into a grid of emoji tiles.
async fn handle_split(
    arguments: emojify::cli::SplitArguments,
    config: &Config,
) -> anyhow::Result<()> {
    if !arguments.image.exists() {
        bail!("image not found: {}", arguments.image.display());
    }

    let name = arguments
        .name
        .clone()
        .unwrap_or_else(|| {
            arguments
                .image
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("emoji")
                .to_owned()
        });

    let cols = arguments.grid.cols;
    let rows = arguments.grid.rows;
    let tile_size = arguments.platform.max_dimension();

    info!(
        image = %arguments.image.display(),
        %name,
        grid = format!("{cols}x{rows}"),
        %tile_size,
        "splitting image into emoji grid"
    );

    let img = image::open(&arguments.image)
        .with_context(|| format!("failed to open image '{}'", arguments.image.display()))?;

    let tiles = emojify::render::split_image(img, cols, rows, tile_size);

    // Create output directory.
    if !arguments.output_dir.exists() {
        std::fs::create_dir_all(&arguments.output_dir)
            .context("failed to create output directory")?;
    }

    // Determine zero-padding width.
    let total = tiles.len();
    let pad_width = if total > 100 { 3 } else { 2 };

    // Encode and write each tile.
    let format = OutputFormat::Png;
    let mut tile_paths = Vec::with_capacity(total);

    for (index, tile) in tiles.iter().enumerate() {
        let tile_name = format!("{name}{index:0>pad_width$}");
        let file_name = format!("{tile_name}.png");
        let tile_path = arguments.output_dir.join(&file_name);

        let encoded = encode_output(tile, format, arguments.platform)
            .map_err(|render_error: RenderError| anyhow::anyhow!(render_error))?;

        std::fs::write(&tile_path, &encoded)
            .with_context(|| format!("failed to write tile '{}'", tile_path.display()))?;

        tile_paths.push((tile_name, tile_path, encoded.len()));
    }

    // Generate and write grid text.
    let grid_text = emojify::render::format_emoji_grid(&name, cols, rows);
    let grid_path = arguments.output_dir.join(format!("{name}_grid.txt"));
    std::fs::write(&grid_path, &grid_text)
        .context("failed to write grid text file")?;

    // Upload if requested.
    if arguments.upload {
        let token = resolve_split_token(&arguments, config)?;

        for (tile_name, tile_path, _byte_count) in &tile_paths {
            if arguments.dry_run {
                println!("Dry run: would upload {} as :{tile_name}: to {}", tile_path.display(), arguments.platform);
                continue;
            }

            let image_data = std::fs::read(tile_path)
                .with_context(|| format!("failed to read tile '{}'", tile_path.display()))?;

            let workspace = arguments.workspace.clone().unwrap_or_default();

            match arguments.platform {
                emojify::Platform::Slack => {
                    let secret = SecretString::new(token.clone());
                    emojify::upload::upload_to_slack(&secret, &workspace, tile_name, &image_data, false)
                        .await
                        .map_err(|upload_error| anyhow::anyhow!(upload_error))?;
                }
                emojify::Platform::Discord => {
                    let secret = SecretString::new(token.clone());
                    emojify::upload::upload_to_discord(&secret, &workspace, tile_name, &image_data, false)
                        .await
                        .map_err(|upload_error| anyhow::anyhow!(upload_error))?;
                }
                _ => bail!("unsupported platform for upload"),
            }

            info!(tile = %tile_name, "uploaded");
        }
    }

    // Output results.
    if arguments.json {
        let result = serde_json::json!({
            "status": "ok",
            "name": name,
            "grid": format!("{cols}x{rows}"),
            "tiles": total,
            "output_dir": arguments.output_dir.display().to_string(),
            "grid_text": grid_text.trim(),
            "tile_files": tile_paths.iter().map(|(n, p, b)| serde_json::json!({
                "name": n,
                "path": p.display().to_string(),
                "bytes": b,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Split {} into {} tiles ({cols}x{rows})", arguments.image.display(), total);
        for (tile_name, tile_path, byte_count) in &tile_paths {
            println!("  {tile_name}: {} ({byte_count} bytes)", tile_path.display());
        }
        println!();
        println!("Grid text (saved to {}):", grid_path.display());
        println!("{grid_text}");
    }

    Ok(())
}

/// Resolve upload token for split subcommand.
fn resolve_split_token(
    arguments: &emojify::cli::SplitArguments,
    config: &Config,
) -> anyhow::Result<String> {
    if let Some(ref token) = arguments.token {
        return Ok(token.clone());
    }

    match arguments.platform {
        emojify::Platform::Slack => {
            if let Some(ref secret) = config.slack_token {
                return Ok(secret.expose().to_owned());
            }
            std::env::var("SLACK_TOKEN").context(
                "no Slack token: pass --token, set SLACK_TOKEN, or add slack_token to config",
            )
        }
        emojify::Platform::Discord => {
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
```

Add `emojify::cli::SplitArguments` to the import at the top of main.rs (alongside `GenerateArguments` and `UploadArguments`). Also add `emojify::cli::Command::Split` recognition in the `run()` match arm.

**Step 3: Run `cargo check` then `cargo clippy -- -D warnings`**

Expected: compiles, no warnings

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire handle_split into CLI dispatch"
```

---

### Task 5: Add CLI integration tests for split

**Files:**
- Modify: `tests/cli_tests.rs`

**Step 1: Add split CLI tests**

```rust
#[test]
fn test_split_basic() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("source.png");
    let output_dir = tmp.path().join("tiles");

    // Generate a source image first.
    emojify()
        .args(["generate", "BIG", "--background", "#FF0000", "-O", input.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args([
            "split",
            input.to_str().unwrap(),
            "--name", "test",
            "--grid", "3x2",
            "-O", output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":test00:"))
        .stdout(predicate::str::contains("6 tiles"));

    // Verify tile files exist.
    assert!(output_dir.join("test00.png").exists());
    assert!(output_dir.join("test05.png").exists());
    assert!(output_dir.join("test_grid.txt").exists());
}

#[test]
fn test_split_defaults_name_from_filename() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("cats.png");
    let output_dir = tmp.path().join("tiles");

    emojify()
        .args(["generate", "meow", "-O", input.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args([
            "split",
            input.to_str().unwrap(),
            "--grid", "2x2",
            "-O", output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":cats00:"));

    assert!(output_dir.join("cats00.png").exists());
}

#[test]
fn test_split_missing_image_fails() {
    emojify()
        .args(["split", "/nonexistent/image.png"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
}

#[test]
fn test_split_invalid_grid_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("img.png");

    emojify()
        .args(["generate", "x", "-O", input.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args(["split", input.to_str().unwrap(), "--grid", "bad"])
        .assert()
        .failure();
}
```

**Step 2: Run full test suite**

Run: `cargo test`
Expected: all tests pass

**Step 3: Run `cargo fmt` and `cargo clippy -- -D warnings`**

Expected: clean

**Step 4: Commit**

```bash
git add tests/cli_tests.rs
git commit -m "test(cli): add split subcommand integration tests"
```

---

### Task 6: Update help text and ensure split shows in --help

**Step 1: Run `cargo run -- --help` and `cargo run -- split --help`**

Verify split subcommand appears in help and has correct descriptions.

**Step 2: Run full verification**

Run: `cargo fmt && cargo clippy -- -D warnings && cargo test`
Expected: all clean, all tests pass

**Step 3: Final commit with .gitignore update**

```bash
git add .gitignore docs/
git commit -m "docs: add split subcommand design and implementation plan"
```
