//! Wire types for notification history and settings.

use ts_rs::TS;

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct NotificationResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub event: String,
    pub title: String,
    pub body: String,
    pub channel: String,
    /// `pending`, `sent` or `failed`.
    pub status: String,
    #[ts(type = "number")]
    pub attempts: i32,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct NotificationSettingsResponse {
    pub enabled: bool,
    /// Configured channel names.
    pub channels: Vec<String>,
    /// Channel names that have an implementation registered.
    pub available: Vec<String>,
}
