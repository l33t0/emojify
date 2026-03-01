//! CLI integration tests for the emojify binary.
//!
//! Uses `assert_cmd` to invoke the compiled binary and verify that each
//! subcommand, flag, and error path behaves as documented.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Build a [`Command`] targeting the `emojify` binary.
fn emojify() -> Command {
    assert_cmd::cargo_bin_cmd!("emojify")
}

#[test]
fn test_generate_text_default() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("hello.png");

    emojify()
        .args(["generate", "hello", "-O", output.to_str().unwrap()])
        .assert()
        .success();

    assert!(output.exists(), "output file should be created");
    assert!(
        std::fs::metadata(&output).unwrap().len() > 0,
        "output file should not be empty"
    );
}

#[test]
fn test_generate_with_platform_discord() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test_discord.png");

    emojify()
        .args([
            "generate",
            "test",
            "--platform",
            "discord",
            "-O",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output.exists(), "output file should be created");

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(
        size < 256_000,
        "Discord output should be under 256 KB, got {size} bytes"
    );
}

#[test]
fn test_generate_with_custom_colors() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("colored.png");

    emojify()
        .args([
            "generate",
            "X",
            "--foreground",
            "#FF0000",
            "--background",
            "#0000FF",
            "-O",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output.exists(), "output file should be created");
}

#[test]
fn test_upload_dry_run_slack() {
    let tmp = TempDir::new().unwrap();
    let image_path = tmp.path().join("emoji.png");

    // First generate an image to upload.
    emojify()
        .args(["generate", "up", "-O", image_path.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args([
            "upload",
            image_path.to_str().unwrap(),
            "--platform",
            "slack",
            "--name",
            "test",
            "--dry-run",
            "--token",
            "fake",
            "--workspace",
            "test-workspace",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_upload_dry_run_discord() {
    let tmp = TempDir::new().unwrap();
    let image_path = tmp.path().join("emoji.png");

    // Generate an image to upload.
    emojify()
        .args(["generate", "up", "-O", image_path.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args([
            "upload",
            image_path.to_str().unwrap(),
            "--platform",
            "discord",
            "--name",
            "test",
            "--dry-run",
            "--token",
            "fake",
            "--workspace",
            "123456789",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_generate_invalid_format() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("bad.png");

    emojify()
        .args([
            "generate",
            "x",
            "--format",
            "invalid",
            "-O",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_version_flag() {
    emojify()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("emojify"));
}

#[test]
fn test_help_flag() {
    emojify()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"))
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("upload"));
}

// ---------------------------------------------------------------------------
// Negative / error-path tests
// ---------------------------------------------------------------------------

#[test]
fn test_generate_no_input_fails() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("empty.png");

    emojify()
        .args(["generate", "-O", output.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn test_upload_missing_file_fails() {
    emojify()
        .args([
            "upload",
            "/nonexistent/file.png",
            "--platform",
            "slack",
            "--name",
            "test",
            "--token",
            "fake",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
}

#[test]
fn test_batch_missing_spec_fails() {
    emojify()
        .args(["batch", "/nonexistent/spec.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
}

// ---------------------------------------------------------------------------
// Split subcommand tests
// ---------------------------------------------------------------------------

#[test]
fn test_split_basic() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("source.png");
    let output_dir = tmp.path().join("tiles");

    // Generate a source image first.
    emojify()
        .args([
            "generate",
            "BIG",
            "--background",
            "#FF0000",
            "-O",
            input.to_str().unwrap(),
        ])
        .assert()
        .success();

    emojify()
        .args([
            "split",
            input.to_str().unwrap(),
            "--name",
            "test",
            "--grid",
            "3x2",
            "-O",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":test00:"))
        .stdout(predicate::str::contains("6 tiles"));

    assert!(output_dir.join("test00.png").exists());
    assert!(output_dir.join("test05.png").exists());
    assert!(output_dir.join("test_grid.txt").exists());
}

#[test]
fn test_split_defaults_name_from_filename() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("cats.png");
    let output_dir = tmp.path().join("out");

    // Generate an image named "cats.png".
    emojify()
        .args(["generate", "meow", "-O", input.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args([
            "split",
            input.to_str().unwrap(),
            "--grid",
            "2x2",
            "-O",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":cats00:"));

    assert!(output_dir.join("cats00.png").exists());
}

#[test]
fn test_split_missing_image_fails() {
    emojify()
        .args(["split", "/nonexistent/image.png", "--grid", "2x2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
}

#[test]
fn test_split_invalid_grid_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("source.png");

    // Generate a source image.
    emojify()
        .args(["generate", "X", "-O", input.to_str().unwrap()])
        .assert()
        .success();

    emojify()
        .args(["split", input.to_str().unwrap(), "--grid", "bad"])
        .assert()
        .failure();
}
