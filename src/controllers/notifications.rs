//! Notification settings and the signed-in user's delivery history.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;

use crate::data::settings::AppSettings;
use crate::dtos::notifications::{NotificationResponse, NotificationSettingsResponse};
use crate::models::{notifications, users};
use crate::notifications::available_channels;

/// `GET /api/notifications/settings`.
#[debug_handler]
pub async fn settings(State(ctx): State<AppContext>, _auth: auth::JWT) -> Result<Response> {
    let settings = AppSettings::from_context(&ctx)?.notifications;
    format::json(NotificationSettingsResponse {
        enabled: settings.enabled,
        channels: settings.channels,
        available: available_channels(),
    })
}

/// `GET /api/notifications` — the current user's most recent notifications.
#[debug_handler]
pub async fn list(State(ctx): State<AppContext>, auth: auth::JWT) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let records = notifications::Model::find_by_user(&ctx.db, user.id, 50).await?;
    let response = records
        .into_iter()
        .map(|row| NotificationResponse {
            id: row.id,
            event: row.event,
            title: row.title,
            body: row.body,
            channel: row.channel,
            status: row.status,
            attempts: row.attempts,
            created_at: row.created_at.to_rfc3339(),
            delivered_at: row.delivered_at.map(|date| date.to_rfc3339()),
        })
        .collect::<Vec<_>>();
    format::json(response)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/notifications")
        .add("/", get(list))
        .add("/settings", get(settings))
}
