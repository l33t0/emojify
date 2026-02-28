//! Discord emoji upload via the REST API.
//!
//! Uploads a rendered image as a custom emoji to a Discord guild (server) using
//! the bot token and base64-encoded image data. Supports dry-run validation
//! without making network calls.

use crate::config::SecretString;
use crate::error::UploadError;
use crate::platform::Platform;

use base64::Engine;
use serde::Deserialize;
use tracing::info;

/// Result of a Discord emoji upload operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscordUploadResult {
    /// The emoji name as registered on Discord.
    pub name: String,
    /// The unique identifier assigned by Discord.
    pub id: String,
    /// CDN URL for the uploaded emoji image.
    pub url: String,
    /// Whether the upload succeeded.
    pub success: bool,
}

/// Response shape returned by the Discord create emoji endpoint.
#[derive(Debug, Deserialize)]
struct DiscordEmojiResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// Upload an image as a custom emoji to a Discord guild.
///
/// When `dry_run` is true, inputs are validated but no network request is made.
///
/// # Errors
///
/// Returns [`UploadError`] if authentication fails, the file exceeds size
/// limits, or the Discord API returns an error.
#[tracing::instrument(skip(token, image_data))]
pub async fn upload_to_discord(
    token: &SecretString,
    guild_id: &str,
    emoji_name: &str,
    image_data: &[u8],
    dry_run: bool,
) -> std::result::Result<DiscordUploadResult, UploadError> {
    // Validate inputs regardless of dry-run mode.
    if token.expose().is_empty() {
        return Err(UploadError::AuthenticationFailed(
            "Discord token is empty".to_string(),
        ));
    }

    if emoji_name.is_empty() {
        return Err(UploadError::ApiError {
            status: 0,
            message: "emoji name must not be empty".to_string(),
        });
    }

    if guild_id.is_empty() {
        return Err(UploadError::ApiError {
            status: 0,
            message: "guild ID must not be empty".to_string(),
        });
    }

    let max_size = Platform::Discord.max_filesize_bytes();
    let actual_size = image_data.len() as u64;
    if actual_size > max_size {
        return Err(UploadError::FileTooLarge {
            size: actual_size,
            max: max_size,
        });
    }

    if dry_run {
        info!(
            emoji_name = %emoji_name,
            guild_id = %guild_id,
            file_size = actual_size,
            "dry run: skipping Discord upload"
        );
        return Ok(DiscordUploadResult {
            name: emoji_name.to_string(),
            id: "dry-run".to_string(),
            url: "https://cdn.discordapp.com/emojis/dry-run.png".to_string(),
            success: true,
        });
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(image_data);
    let data_uri = format!("data:image/png;base64,{encoded}");

    let body = serde_json::json!({
        "name": emoji_name,
        "image": data_uri,
    });

    let endpoint = format!("https://discord.com/api/v10/guilds/{guild_id}/emojis");

    let client = reqwest::Client::new();

    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bot {}", token.expose()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| UploadError::NetworkError(error.to_string()))?;

    let status = response.status();
    let response_body = response
        .text()
        .await
        .map_err(|error| UploadError::NetworkError(error.to_string()))?;

    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(UploadError::AuthenticationFailed(format!(
            "HTTP {}: {response_body}",
            status.as_u16()
        )));
    }

    if !status.is_success() {
        return Err(UploadError::ApiError {
            status: status.as_u16(),
            message: response_body,
        });
    }

    let api_response: DiscordEmojiResponse =
        serde_json::from_str(&response_body).map_err(|error| UploadError::ApiError {
            status: status.as_u16(),
            message: format!("failed to parse response: {error}; body: {response_body}"),
        })?;

    let emoji_id = api_response.id.ok_or_else(|| UploadError::ApiError {
        status: status.as_u16(),
        message: api_response
            .message
            .unwrap_or_else(|| "response missing emoji id".to_string()),
    })?;

    let cdn_url = format!("https://cdn.discordapp.com/emojis/{emoji_id}.png");

    info!(
        emoji_name = %emoji_name,
        guild_id = %guild_id,
        emoji_id = %emoji_id,
        "uploaded emoji to Discord"
    );

    Ok(DiscordUploadResult {
        name: emoji_name.to_string(),
        id: emoji_id,
        url: cdn_url,
        success: true,
    })
}
