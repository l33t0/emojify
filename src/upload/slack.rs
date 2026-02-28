//! Slack emoji upload via the `emoji.add` API.
//!
//! Uploads a rendered image as a custom emoji to a Slack workspace using
//! multipart form data. Supports dry-run validation without making network
//! calls.

use crate::config::SecretString;
use crate::error::UploadError;
use crate::platform::Platform;

use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tracing::info;

/// Result of a Slack emoji upload operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SlackUploadResult {
    /// The emoji name as registered on Slack (without colons).
    pub name: String,
    /// URL to the workspace emoji customization page.
    pub url: String,
    /// Whether the upload succeeded.
    pub success: bool,
}

/// Response shape returned by the Slack `emoji.add` API.
#[derive(Debug, Deserialize)]
struct SlackApiResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

/// Upload an image as a custom emoji to a Slack workspace.
///
/// When `dry_run` is true, inputs are validated but no network request is made.
///
/// # Errors
///
/// Returns [`UploadError`] if authentication fails, the file exceeds size
/// limits, or the Slack API returns an error.
#[tracing::instrument(skip(token, image_data))]
pub async fn upload_to_slack(
    token: &SecretString,
    workspace: &str,
    emoji_name: &str,
    image_data: &[u8],
    dry_run: bool,
) -> std::result::Result<SlackUploadResult, UploadError> {
    // Validate inputs regardless of dry-run mode.
    if token.expose().is_empty() {
        return Err(UploadError::AuthenticationFailed(
            "Slack token is empty".to_string(),
        ));
    }

    if emoji_name.is_empty() {
        return Err(UploadError::ApiError {
            status: 0,
            message: "emoji name must not be empty".to_string(),
        });
    }

    let max_size = Platform::Slack.max_filesize_bytes();
    let actual_size = image_data.len() as u64;
    if actual_size > max_size {
        return Err(UploadError::FileTooLarge {
            size: actual_size,
            max: max_size,
        });
    }

    let url = format!("https://{workspace}.slack.com/customize/emoji");

    if dry_run {
        info!(
            emoji_name = %emoji_name,
            workspace = %workspace,
            file_size = actual_size,
            "dry run: skipping Slack upload"
        );
        return Ok(SlackUploadResult {
            name: emoji_name.to_string(),
            url,
            success: true,
        });
    }

    let endpoint = format!("https://{workspace}.slack.com/api/emoji.add");

    let image_part = Part::bytes(image_data.to_vec())
        .file_name("emoji.png")
        .mime_str("image/png")
        .map_err(|error| UploadError::NetworkError(error.to_string()))?;

    let form = Form::new()
        .text("token", token.expose().to_string())
        .text("name", emoji_name.to_string())
        .text("mode", "data".to_string())
        .part("image", image_part);

    let client = reqwest::Client::new();

    let response = client
        .post(&endpoint)
        .multipart(form)
        .send()
        .await
        .map_err(|error| UploadError::NetworkError(error.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| UploadError::NetworkError(error.to_string()))?;

    if !status.is_success() {
        return Err(UploadError::ApiError {
            status: status.as_u16(),
            message: body,
        });
    }

    let api_response: SlackApiResponse =
        serde_json::from_str(&body).map_err(|error| UploadError::ApiError {
            status: status.as_u16(),
            message: format!("failed to parse response: {error}; body: {body}"),
        })?;

    if !api_response.ok {
        let error_message = api_response
            .error
            .unwrap_or_else(|| "unknown error".to_string());

        // Slack returns specific error codes for auth failures.
        if error_message == "not_authed" || error_message == "invalid_auth" {
            return Err(UploadError::AuthenticationFailed(error_message));
        }

        return Err(UploadError::ApiError {
            status: status.as_u16(),
            message: error_message,
        });
    }

    info!(
        emoji_name = %emoji_name,
        workspace = %workspace,
        "uploaded emoji to Slack"
    );

    Ok(SlackUploadResult {
        name: emoji_name.to_string(),
        url,
        success: true,
    })
}
