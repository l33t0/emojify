# Split Subcommand Design

**Date:** 2026-03-01
**Status:** Approved

## Purpose

Take an input image and split it into a grid of emoji-sized tiles (128x128) that can be pasted as custom emoji in Slack/Discord to display a large image — similar to [slack-big-emoji](https://github.com/kinduff/slack-big-emoji).

## CLI Interface

```
emojify split <IMAGE> [--name <PREFIX>] [--grid 5x5] [--platform slack] [-O ./output/] [--upload] [--token ...] [--workspace ...] [--dry-run] [--json]
```

- `IMAGE` — path to source image (required positional arg)
- `--name` — optional prefix for tile filenames and emoji names. Defaults to the input filename stem (e.g. `cats.jpg` → `cats`)
- `--grid` — `COLSxROWS` format, default `5x5`
- `--platform` — defaults to `slack`, determines tile size (128x128) and file size limits
- `-O` / `--output-dir` — defaults to `./output/`
- `--upload` — optional flag; when set, uploads all tiles then prints emoji grid
- `--token`, `--workspace`, `--dry-run` — reuse existing upload semantics, only relevant with `--upload`
- `--json` — machine-readable output

## Processing Pipeline

1. Load source image with `image::open`
2. Resize to exact grid dimensions: `cols * 128` x `rows * 128` (stretches to fill, no letterboxing)
3. Crop into `cols * rows` tiles of 128x128 each, left-to-right, top-to-bottom
4. Encode each tile as PNG via existing `encode_output`
5. Write tiles to `{output_dir}/{name}{index:02}.png`
6. Print paste-able emoji grid to stdout and save to `{output_dir}/{name}_grid.txt`
7. If `--upload`, upload each tile via existing Slack/Discord upload functions

## Tile Naming

Sequential zero-padded: `name00`, `name01`, ..., `name24` for a 5x5 grid.

## Grid Text Output

```
:cats00::cats01::cats02::cats03::cats04:
:cats05::cats06::cats07::cats08::cats09:
:cats10::cats11::cats12::cats13::cats14:
:cats15::cats16::cats17::cats18::cats19:
:cats20::cats21::cats22::cats23::cats24:
```

## Code Changes

- `src/cli.rs` — add `Split(SplitArguments)` variant + `SplitArguments` struct
- `src/render/split.rs` — new module: `split_image(img, cols, rows, tile_size) -> Vec<RgbaImage>`
- `src/render.rs` — add `mod split` + re-export `split_image`
- `src/main.rs` — add `handle_split` handler, wire into `run()` dispatch
- `tests/render_tests.rs` — add split tests (correct tile count, tile dimensions, grid text format)
- `tests/cli_tests.rs` — add split CLI integration test
