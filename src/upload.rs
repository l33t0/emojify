//! Upload pipeline for Slack and Discord emoji APIs.

mod discord;
mod slack;

pub use discord::{DiscordUploadResult, upload_to_discord};
pub use slack::{SlackUploadResult, upload_to_slack};
