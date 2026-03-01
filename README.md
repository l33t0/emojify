# emojify

Split images into emoji grids for Slack and Discord big-emoji, generate custom emoji from text or images, and upload them directly.

[![CI](https://github.com/l33t0/emojify/actions/workflows/ci.yml/badge.svg)](https://github.com/l33t0/emojify/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/emojify.svg)](https://crates.io/crates/emojify)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Big Emoji

Split any image into a grid of emoji-sized tiles that combine into a large image when pasted into Slack or Discord:

```sh
emojify split photo.jpg --name cats --grid 5x5
```

This produces 25 tile images (`cats00.png` through `cats24.png`) and a grid text file you can paste directly into chat:

```
:cats00::cats01::cats02::cats03::cats04:
:cats05::cats06::cats07::cats08::cats09:
:cats10::cats11::cats12::cats13::cats14:
:cats15::cats16::cats17::cats18::cats19:
:cats20::cats21::cats22::cats23::cats24:
```

Upload all tiles in one command:

```sh
emojify split photo.jpg --name cats --grid 5x5 --upload --platform slack --token "$SLACK_TOKEN" --workspace myteam
```

## Features

- **Image splitting** — split any image into an NxM grid of emoji tiles for big-emoji compositions
- Render text to emoji-sized images with automatic font scaling and multi-line support
- Load and resize existing images to platform-compliant dimensions
- Overlay compositing with configurable anchor positions (corners, edges, center)
- Linear gradient generation and gradient-masked text rendering
- Animated GIF output with pulse effects
- Platform-aware encoding that enforces Slack and Discord file size and dimension limits
- Batch generation from a TOML spec file
- Direct upload to Slack and Discord with dry-run validation
- Interactive TUI for live emoji preview and generation
- Configuration file support for defaults and API tokens

## Installation

### Cargo

```sh
cargo install emojify
```

### Homebrew

```sh
brew tap l33t0/emojify
brew install emojify
```

### Docker

```sh
docker run --rm ghcr.io/l33t0/emojify generate "hello" > hello.png
```

### From source

```sh
git clone https://github.com/l33t0/emojify.git
cd emojify
cargo build --release
# Binary is at target/release/emojify
```

## Quick Start

Split an image into a 5x5 emoji grid:

```sh
emojify split photo.jpg --name hype --grid 5x5
```

Generate a text emoji for Slack (default):

```sh
emojify generate "LFG" -O output/lfg.png
```

Generate for Discord with custom colours:

```sh
emojify generate "GG" --platform discord --foreground "#00FF00" --background "#1a1a1a" -O output/gg.png
```

Upload to Slack (dry run):

```sh
emojify upload output/lfg.png --platform slack --name lfg --dry-run --token "$SLACK_TOKEN" --workspace myteam
```

Batch generate from a spec file:

```sh
emojify batch emojis.toml --platform slack --output-dir ./output
```

Launch the interactive TUI:

```sh
emojify tui
```

## CLI Reference

### `emojify generate`

Render an emoji image from text or an input image.

```
emojify generate [TEXT] [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--input <PATH>` | `-i` | | Path to an input image file |
| `--stdin` | | `false` | Read image data from stdin |
| `--platform <PLATFORM>` | `-p` | `slack` | Target platform: `slack` or `discord` |
| `--output <PATH>` | `-O` | `./output.png` | Output file path |
| `--format <FORMAT>` | `-f` | inferred | Output format: `png`, `webp`, or `gif` |
| `--animated` | `-a` | `false` | Generate an animated GIF |
| `--preview` | | `false` | Open the output with the system viewer |
| `--font-size <PX>` | | `64` | Font size in pixels |
| `--padding <PX>` | | `8` | Padding around text in pixels |
| `--foreground <HEX>` | | `#FFFFFF` | Text colour as hex (`#RRGGBB`) |
| `--background <HEX>` | | `transparent` | Background colour or `transparent` |
| `--gradient <SPEC>` | | | Gradient as `start_hex,end_hex[,direction]` |
| `--overlay <SPEC>` | `-o` | | Overlay as `emoji:anchor` (up to 2) |
| `--json` | | `false` | Emit machine-readable JSON output |

Exactly one of `TEXT`, `--input`, or `--stdin` must be provided.

### `emojify upload`

Upload a generated emoji image to Slack or Discord.

```
emojify upload <FILE> [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--platform <PLATFORM>` | `-p` | | Target platform: `slack` or `discord` |
| `--name <NAME>` | `-n` | | Emoji name on the platform |
| `--token <TOKEN>` | `-t` | env var | API token (`SLACK_TOKEN` or `DISCORD_TOKEN`) |
| `--workspace <ID>` | `-w` | | Workspace (Slack) or guild ID (Discord) |
| `--dry-run` | | `false` | Validate without uploading |

### `emojify split`

Split an image into a grid of emoji-sized tiles for big-emoji usage.

```
emojify split <IMAGE> [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--name <PREFIX>` | `-n` | filename stem | Base name prefix for tiles and emoji names |
| `--grid <COLSxROWS>` | `-g` | `5x5` | Grid dimensions (e.g. `5x5`, `3x2`, `7x4`) |
| `--platform <PLATFORM>` | `-p` | `slack` | Target platform: `slack` or `discord` |
| `--output-dir <DIR>` | `-O` | `./output` | Directory for tile images and grid text |
| `--upload` | | `false` | Upload all tiles after splitting |
| `--token <TOKEN>` | `-t` | env var | API token (only with `--upload`) |
| `--workspace <ID>` | `-w` | | Workspace or guild ID (only with `--upload`) |
| `--dry-run` | | `false` | Validate without uploading |
| `--json` | | `false` | Emit machine-readable JSON output |

### `emojify batch`

Generate multiple emoji images from a TOML specification file.

```
emojify batch <SPEC_FILE> [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--platform <PLATFORM>` | `-p` | `slack` | Target platform |
| `--output-dir <DIR>` | `-o` | `.` | Output directory |
| `--json` | | `false` | Emit machine-readable JSON output |

Example spec file (`emojis.toml`):

```toml
[[emoji]]
name = "ship-it"
text = "SHIP"
foreground = "#00FF00"

[[emoji]]
name = "nope"
text = "NOPE"
foreground = "#FF0000"
background = "#000000"
font_size = 48
```

### `emojify tui`

Launch the interactive terminal UI for live emoji preview and generation.

```
emojify tui
```

## Configuration

Create `~/.config/emojify/config.toml` to set defaults:

```toml
platform = "slack"
output_dir = "~/emojis"
font_size = 48
slack_token = "xoxp-..."
discord_token = "Bot ..."
```

CLI flags always take precedence over config file values. Token fields can also be set via `SLACK_TOKEN` and `DISCORD_TOKEN` environment variables.

The config file is checked for world-readable permissions on Unix systems. A warning is emitted if tokens may be exposed.

## Platform Constraints

| Platform | Max Dimensions | Max File Size | Supported Formats |
|----------|---------------|---------------|-------------------|
| Slack | 128x128 px | 1 MB | PNG, GIF, WebP |
| Discord | 128x128 px | 256 KB | PNG, GIF, WebP |

## Building from Source

Requirements:
- Rust 1.85.0 or later (MSRV)
- A C linker (provided by your system toolchain)

```sh
git clone https://github.com/l33t0/emojify.git
cd emojify
cargo build --release
```

Run the test suite:

```sh
cargo nextest run
```

Run lints:

```sh
cargo fmt --check
cargo clippy -- -D warnings
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-change`)
3. Make your changes and add tests
4. Ensure `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo nextest run` all pass
5. Commit with a descriptive message using conventional commit format
6. Open a pull request against `master`

## License

MIT -- see [LICENSE](LICENSE) for details.
