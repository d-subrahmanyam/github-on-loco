//! Wire types for webhook subscriptions and their delivery history.

use ts_rs::TS;

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct WebhookResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub url: String,
    /// Comma-separated event kinds, or `*`.
    pub events: String,
    pub is_active: bool,
    pub created_by: String,
    /// Real value only on creation; list/show return a masked placeholder.
    pub secret: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateWebhookParams {
    pub url: String,
    pub events: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateWebhookParams {
    pub events: String,
    pub is_active: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct WebhookDeliveryResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub event: String,
    /// `pending`, `success` or `failed`.
    pub status: String,
    #[ts(type = "number")]
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub delivered_at: Option<String>,
}
