//! Full ratatui TUI application for interactive emoji generation.
//!
//! Provides a split-pane interface with a live unicode block-art preview on the
//! left and an editable form on the right. Supports generating emoji images,
//! writing them to disk, and uploading to Slack or Discord.

use crate::config::Config;
use crate::platform::Platform;
use crate::render::{TextRenderOptions, render_text};
use crate::upload::{upload_to_discord, upload_to_slack};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use image::{Rgba, RgbaImage};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tracing::{error, info};

use std::io::Stdout;
use std::time::Duration;

/// Which form field is currently focused for editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveField {
    /// The emoji text input.
    Text,
    /// The target platform selector.
    Platform,
    /// Foreground color hex input.
    Foreground,
    /// Background color hex input.
    Background,
    /// Font size numeric input.
    FontSize,
    /// Output file path input.
    OutputPath,
    /// First overlay specification.
    Overlay1,
    /// Second overlay specification.
    Overlay2,
}

impl ActiveField {
    /// Advance to the next field in tab order.
    fn next(self) -> Self {
        match self {
            ActiveField::Text => ActiveField::Platform,
            ActiveField::Platform => ActiveField::Foreground,
            ActiveField::Foreground => ActiveField::Background,
            ActiveField::Background => ActiveField::FontSize,
            ActiveField::FontSize => ActiveField::OutputPath,
            ActiveField::OutputPath => ActiveField::Overlay1,
            ActiveField::Overlay1 => ActiveField::Overlay2,
            ActiveField::Overlay2 => ActiveField::Text,
        }
    }

    /// Move to the previous field in tab order.
    fn previous(self) -> Self {
        match self {
            ActiveField::Text => ActiveField::Overlay2,
            ActiveField::Platform => ActiveField::Text,
            ActiveField::Foreground => ActiveField::Platform,
            ActiveField::Background => ActiveField::Foreground,
            ActiveField::FontSize => ActiveField::Background,
            ActiveField::OutputPath => ActiveField::FontSize,
            ActiveField::Overlay1 => ActiveField::OutputPath,
            ActiveField::Overlay2 => ActiveField::Overlay1,
        }
    }

    /// Human-readable label for the field.
    fn label(self) -> &'static str {
        match self {
            ActiveField::Text => "Text",
            ActiveField::Platform => "Platform",
            ActiveField::Foreground => "Foreground",
            ActiveField::Background => "Background",
            ActiveField::FontSize => "Font Size",
            ActiveField::OutputPath => "Output Path",
            ActiveField::Overlay1 => "Overlay 1",
            ActiveField::Overlay2 => "Overlay 2",
        }
    }
}

/// All ordered fields for iteration during rendering.
const ALL_FIELDS: [ActiveField; 8] = [
    ActiveField::Text,
    ActiveField::Platform,
    ActiveField::Foreground,
    ActiveField::Background,
    ActiveField::FontSize,
    ActiveField::OutputPath,
    ActiveField::Overlay1,
    ActiveField::Overlay2,
];

/// Application state for the interactive TUI.
#[derive(Debug)]
pub struct App {
    /// Currently focused form field.
    pub(crate) active_field: ActiveField,
    /// Emoji text to render.
    pub(crate) text: String,
    /// Target platform.
    pub(crate) platform: Platform,
    /// Foreground color as a hex string (e.g. "#FFFFFF").
    pub(crate) foreground: String,
    /// Background color as a hex string or "transparent".
    pub(crate) background: String,
    /// Font size in pixels as a string for editing.
    pub(crate) font_size: String,
    /// Output file path.
    pub(crate) output_path: String,
    /// First overlay specification (format: emoji:anchor).
    pub(crate) overlay1: String,
    /// Second overlay specification (format: emoji:anchor).
    pub(crate) overlay2: String,
    /// Status bar message shown at the bottom of the form panel.
    pub(crate) status_message: String,
    /// Set to true when the user requests to quit.
    pub(crate) should_quit: bool,
    /// Cached preview image from the last successful render.
    pub(crate) preview_image: Option<RgbaImage>,
    /// Loaded application configuration.
    pub(crate) config: Config,
}

impl App {
    /// Create a new `App` seeded with values from the provided configuration.
    fn new(config: Config) -> Self {
        let platform = config.platform.unwrap_or_default();
        Self {
            active_field: ActiveField::Text,
            text: String::new(),
            platform,
            foreground: "#FFFFFF".to_string(),
            background: "transparent".to_string(),
            font_size: config
                .font_size
                .map_or("64".to_string(), |s| s.to_string()),
            output_path: config.output_dir.as_ref().map_or(
                "./output.png".to_string(),
                |d| d.join("output.png").display().to_string(),
            ),
            overlay1: String::new(),
            overlay2: String::new(),
            status_message:
                "Ready. Press Enter to generate, 'u' to upload, Space to cycle platform, 'q' to quit."
                    .to_string(),
            should_quit: false,
            preview_image: None,
            config,
        }
    }

    /// Get a mutable reference to the string backing the currently active field.
    ///
    /// Returns `None` for the `Platform` field, which is cycled rather than
    /// edited as text.
    fn active_field_value_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            ActiveField::Text => Some(&mut self.text),
            ActiveField::Platform => None,
            ActiveField::Foreground => Some(&mut self.foreground),
            ActiveField::Background => Some(&mut self.background),
            ActiveField::FontSize => Some(&mut self.font_size),
            ActiveField::OutputPath => Some(&mut self.output_path),
            ActiveField::Overlay1 => Some(&mut self.overlay1),
            ActiveField::Overlay2 => Some(&mut self.overlay2),
        }
    }

    /// Get the display value for a field.
    fn field_display_value(&self, field: ActiveField) -> String {
        match field {
            ActiveField::Text => self.text.clone(),
            ActiveField::Platform => format!("{}", self.platform),
            ActiveField::Foreground => self.foreground.clone(),
            ActiveField::Background => self.background.clone(),
            ActiveField::FontSize => self.font_size.clone(),
            ActiveField::OutputPath => self.output_path.clone(),
            ActiveField::Overlay1 => self.overlay1.clone(),
            ActiveField::Overlay2 => self.overlay2.clone(),
        }
    }

    /// Cycle the platform between Slack and Discord.
    fn cycle_platform(&mut self) {
        self.platform = match self.platform {
            Platform::Slack => Platform::Discord,
            Platform::Discord => Platform::Slack,
        };
    }

    /// Parse the foreground hex color string into an RGBA value.
    fn parse_foreground(&self) -> Rgba<u8> {
        parse_hex_color(&self.foreground).unwrap_or(Rgba([255, 255, 255, 255]))
    }

    /// Parse the background color string into an optional RGBA value.
    fn parse_background(&self) -> Option<Rgba<u8>> {
        if self.background.trim().eq_ignore_ascii_case("transparent") {
            None
        } else {
            Some(parse_hex_color(&self.background).unwrap_or(Rgba([0, 0, 0, 255])))
        }
    }

    /// Parse the font size string into an f32 value.
    fn parse_font_size(&self) -> f32 {
        self.font_size.parse::<f32>().unwrap_or(64.0)
    }

    /// Attempt to render the current form state into a preview image.
    fn try_render_preview(&mut self) {
        if self.text.is_empty() {
            self.preview_image = None;
            return;
        }

        let options = TextRenderOptions {
            text: self.text.clone(),
            font_size: self.parse_font_size(),
            padding: 8,
            foreground: self.parse_foreground(),
            background: self.parse_background(),
            canvas_size: self.platform.max_dimension(),
        };

        match render_text(&options) {
            Ok(image) => {
                self.preview_image = Some(image);
            }
            Err(render_error) => {
                self.status_message = format!("Preview error: {render_error}");
                self.preview_image = None;
            }
        }
    }

    /// Generate the emoji image and write it to the output path.
    fn generate(&mut self) {
        if self.text.is_empty() {
            self.status_message = "Cannot generate: text is empty.".to_string();
            return;
        }

        let options = TextRenderOptions {
            text: self.text.clone(),
            font_size: self.parse_font_size(),
            padding: 8,
            foreground: self.parse_foreground(),
            background: self.parse_background(),
            canvas_size: self.platform.max_dimension(),
        };

        let image = match render_text(&options) {
            Ok(image) => image,
            Err(render_error) => {
                self.status_message = format!("Render failed: {render_error}");
                return;
            }
        };

        self.preview_image = Some(image.clone());

        if let Err(save_error) = image.save(&self.output_path) {
            self.status_message = format!("Save failed: {save_error}");
            return;
        }

        self.status_message = format!("Saved to {}", self.output_path);
        info!(output = %self.output_path, "generated emoji image");
    }
}

/// Parse a hex color string like "#RRGGBB" or "#RRGGBBAA" into an RGBA value.
fn parse_hex_color(hex: &str) -> Option<Rgba<u8>> {
    crate::parse_color(hex).ok()
}

/// Convert an `RgbaImage` to a vector of lines using unicode half-block
/// characters for a best-effort terminal preview.
///
/// Each output line represents two rows of pixels using the upper-half-block
/// character. The preview is scaled to fit within the given terminal
/// dimensions.
fn image_to_block_art(image: &RgbaImage, max_columns: u16, max_rows: u16) -> Vec<Line<'static>> {
    if image.width() == 0 || image.height() == 0 || max_columns == 0 || max_rows == 0 {
        return vec![Line::from("(empty)")];
    }

    // Each terminal cell covers 1 pixel wide and 2 pixels tall (using half-blocks).
    let available_pixel_width = max_columns as u32;
    let available_pixel_height = max_rows as u32 * 2;

    let scale_x = available_pixel_width as f64 / image.width() as f64;
    let scale_y = available_pixel_height as f64 / image.height() as f64;
    let scale = scale_x.min(scale_y).min(1.0);

    let display_width = ((image.width() as f64 * scale).round() as u32).max(1);
    let display_height = ((image.height() as f64 * scale).round() as u32).max(1);

    // Ensure the display height is even for half-block pairing.
    let display_height = if display_height % 2 != 0 {
        display_height + 1
    } else {
        display_height
    };

    let mut lines = Vec::with_capacity((display_height / 2) as usize);

    for row_pair in (0..display_height).step_by(2) {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(display_width as usize);

        for column in 0..display_width {
            // Map display coordinates back to source image coordinates.
            let source_x =
                ((column as f64 / scale).round() as u32).min(image.width().saturating_sub(1));
            let source_y_top =
                ((row_pair as f64 / scale).round() as u32).min(image.height().saturating_sub(1));
            let source_y_bottom = (((row_pair + 1) as f64 / scale).round() as u32)
                .min(image.height().saturating_sub(1));

            let top_pixel = image.get_pixel(source_x, source_y_top);
            let bottom_pixel = image.get_pixel(source_x, source_y_bottom);

            let top_color = rgba_to_terminal_color(top_pixel);
            let bottom_color = rgba_to_terminal_color(bottom_pixel);

            // Use upper-half-block: foreground is top, background is bottom.
            spans.push(Span::styled(
                "\u{2580}",
                Style::default().fg(top_color).bg(bottom_color),
            ));
        }

        lines.push(Line::from(spans));
    }

    lines
}

/// Map an RGBA pixel to a ratatui terminal color.
///
/// Fully transparent pixels map to the terminal default (black). Opaque or
/// semi-transparent pixels map to an RGB color.
fn rgba_to_terminal_color(pixel: &Rgba<u8>) -> Color {
    if pixel.0[3] < 32 {
        Color::Black
    } else {
        Color::Rgb(pixel.0[0], pixel.0[1], pixel.0[2])
    }
}

/// Draw the complete UI frame.
fn draw_frame(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &App) -> anyhow::Result<()> {
    terminal.draw(|frame| {
        let size = frame.area();

        // Top-level vertical layout: title bar, main body, bottom bar.
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // title bar
                Constraint::Min(10),   // main body
                Constraint::Length(3), // bottom bar
            ])
            .split(size);

        // Title bar.
        let max_dim = app.platform.max_dimension();
        let max_size = app.platform.max_filesize_bytes();
        let title_text = format!(
            " emojify | platform: {} | max: {}x{}px, {} bytes",
            app.platform, max_dim, max_dim, max_size
        );
        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title(" emojify "));
        frame.render_widget(title, vertical_chunks[0]);

        // Main body: left preview panel + right form panel.
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(vertical_chunks[1]);

        // Left panel: preview.
        let preview_block = Block::default().borders(Borders::ALL).title(" Preview ");

        let preview_inner = preview_block.inner(horizontal_chunks[0]);
        frame.render_widget(preview_block, horizontal_chunks[0]);

        if let Some(ref image) = app.preview_image {
            let preview_width = preview_inner.width;
            let preview_height = preview_inner.height;
            let art_lines = image_to_block_art(image, preview_width, preview_height);
            let preview_paragraph = Paragraph::new(art_lines);
            frame.render_widget(preview_paragraph, preview_inner);
        } else {
            let placeholder =
                Paragraph::new("No preview available.\nEnter text and press Enter to generate.")
                    .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(placeholder, preview_inner);
        }

        // Right panel: form fields + status.
        let form_block = Block::default().borders(Borders::ALL).title(" Settings ");

        let form_inner = form_block.inner(horizontal_chunks[1]);
        frame.render_widget(form_block, horizontal_chunks[1]);

        // Layout for form fields: one row per field + status area.
        let field_count = ALL_FIELDS.len() as u16;
        let mut form_constraints: Vec<Constraint> =
            ALL_FIELDS.iter().map(|_| Constraint::Length(2)).collect();
        // Status area gets remaining space.
        form_constraints.push(Constraint::Min(2));

        let form_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(form_constraints)
            .split(form_inner);

        for (index, &field) in ALL_FIELDS.iter().enumerate() {
            let is_active = field == app.active_field;
            let label = field.label();
            let value = app.field_display_value(field);

            let display = if is_active {
                format!(" > {label}: {value}_")
            } else {
                format!("   {label}: {value}")
            };

            let style = if is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let field_paragraph = Paragraph::new(display).style(style);

            if index < form_chunks.len() {
                frame.render_widget(field_paragraph, form_chunks[index]);
            }
        }

        // Status message in the remaining form space.
        let status_index = field_count as usize;
        if status_index < form_chunks.len() {
            let status = Paragraph::new(format!(" {}", app.status_message))
                .style(Style::default().fg(Color::Green))
                .wrap(Wrap { trim: false });
            frame.render_widget(status, form_chunks[status_index]);
        }

        // Bottom help bar.
        let help_text =
            " q: quit | Tab/Shift+Tab: navigate | Space: cycle platform | Enter: generate | u: upload | Esc: quit ";
        let help_bar = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Keys "));
        frame.render_widget(help_bar, vertical_chunks[2]);
    })?;

    Ok(())
}

/// Handle a keyboard event and update app state accordingly.
///
/// Returns an [`InputAction`] describing what side-effect, if any, should
/// occur as a result of the key press.
fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    match code {
        KeyCode::Char('q') if app.active_field != ActiveField::Text => {
            app.should_quit = true;
            InputAction::None
        }
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            InputAction::None
        }
        KeyCode::Esc => {
            app.should_quit = true;
            InputAction::None
        }
        KeyCode::Tab if modifiers.contains(KeyModifiers::SHIFT) => {
            app.active_field = app.active_field.previous();
            InputAction::None
        }
        KeyCode::Tab => {
            app.active_field = app.active_field.next();
            InputAction::None
        }
        KeyCode::Char(' ') if app.active_field == ActiveField::Platform => {
            app.cycle_platform();
            InputAction::PreviewUpdate
        }
        KeyCode::Enter => InputAction::Generate,
        KeyCode::Char('u') if app.active_field != ActiveField::Text => InputAction::Upload,
        KeyCode::Backspace => {
            if let Some(value) = app.active_field_value_mut() {
                value.pop();
            }
            InputAction::PreviewUpdate
        }
        KeyCode::Char(character) => {
            if let Some(value) = app.active_field_value_mut() {
                value.push(character);
            }
            InputAction::PreviewUpdate
        }
        _ => InputAction::None,
    }
}

/// Actions that can result from processing user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputAction {
    /// No side effects needed.
    None,
    /// The preview should be re-rendered.
    PreviewUpdate,
    /// The user wants to generate and save the image.
    Generate,
    /// The user wants to upload the current image.
    Upload,
}

/// Attempt an upload of the last generated image to the current platform.
async fn attempt_upload(app: &mut App) {
    let image_data = match &app.preview_image {
        Some(image) => {
            let mut buffer = std::io::Cursor::new(Vec::new());
            if let Err(encode_error) = image.write_to(&mut buffer, image::ImageFormat::Png) {
                app.status_message = format!("Encode failed: {encode_error}");
                return;
            }
            buffer.into_inner()
        }
        None => {
            app.status_message = "Nothing to upload. Generate an image first.".to_string();
            return;
        }
    };

    let config = app.config.clone();

    let emoji_name = if app.text.is_empty() {
        "emoji".to_string()
    } else {
        app.text
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_lowercase()
    };

    if emoji_name.is_empty() {
        app.status_message =
            "Cannot upload: emoji name would be empty after sanitization.".to_string();
        return;
    }

    match app.platform {
        Platform::Slack => {
            let token = match config.slack_token {
                Some(token) => token,
                None => {
                    app.status_message =
                        "No Slack token configured. Set slack_token in config.toml.".to_string();
                    return;
                }
            };
            app.status_message = "Uploading to Slack...".to_string();
            match upload_to_slack(&token, "workspace", &emoji_name, &image_data, false).await {
                Ok(result) => {
                    app.status_message =
                        format!("Uploaded '{}' to Slack: {}", result.name, result.url);
                }
                Err(upload_error) => {
                    app.status_message = format!("Slack upload failed: {upload_error}");
                    error!(%upload_error, "Slack upload failed");
                }
            }
        }
        Platform::Discord => {
            let token = match config.discord_token {
                Some(token) => token,
                None => {
                    app.status_message =
                        "No Discord token configured. Set discord_token in config.toml."
                            .to_string();
                    return;
                }
            };
            app.status_message = "Uploading to Discord...".to_string();
            match upload_to_discord(&token, "guild_id", &emoji_name, &image_data, false).await {
                Ok(result) => {
                    app.status_message =
                        format!("Uploaded '{}' to Discord: {}", result.name, result.url);
                }
                Err(upload_error) => {
                    app.status_message = format!("Discord upload failed: {upload_error}");
                    error!(%upload_error, "Discord upload failed");
                }
            }
        }
    }
}

/// Run the interactive TUI application.
///
/// Sets up the terminal, runs the event loop, and ensures cleanup on exit.
///
/// # Errors
///
/// Returns an error if terminal setup, event reading, or rendering fails.
pub async fn run_tui(config: &Config) -> anyhow::Result<()> {
    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(config.clone());

    let result = run_event_loop(&mut terminal, &mut app).await;

    // Terminal teardown -- always runs even if the event loop errored.
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Core event loop that reads input, updates state, and renders frames.
async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    // Initial draw.
    draw_frame(terminal, app)?;

    loop {
        // Poll for events with a short timeout to stay responsive.
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    let action = handle_key_event(app, key_event.code, key_event.modifiers);

                    match action {
                        InputAction::None => {}
                        InputAction::PreviewUpdate => {
                            app.try_render_preview();
                        }
                        InputAction::Generate => {
                            app.generate();
                        }
                        InputAction::Upload => {
                            // We need to draw "Uploading..." before the async call.
                            app.status_message = format!("Uploading to {}...", app.platform);
                            draw_frame(terminal, app)?;
                            attempt_upload(app).await;
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal will be redrawn on next iteration.
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }

        draw_frame(terminal, app)?;
    }

    Ok(())
}
