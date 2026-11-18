//! Feature-flagged fan-out of activity events to notification channels.
//!
//! When `settings.notifications.enabled` is false this does nothing at all —
//! no rows, no queue jobs, no SMTP. When enabled, one pending row is created per
//! (watcher, channel) pair and the delivery worker resolves the channel by name.

use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use serde_json::Value;

use crate::data::settings::AppSettings;
use crate::models::{issues, organizations, pull_requests, repositories, users, watches};
use crate::notifications::channel;
use crate::workers::deliver_notification::{Worker as DeliverNotificationWorker, WorkerArgs};

fn event_label(event: &str) -> &'static str {
    match event {
        "push" => "New commits",
        "issue.opened" => "New issue",
        "issue.closed" => "Issue closed",
        "issue.reopened" => "Issue reopened",
        "pr.opened" => "New pull request",
        "pr.merged" => "Pull request merged",
        "pr.approved" => "Pull request approved",
        "release.published" => "New release",
        "fork" => "New fork",
        _ => "Repository activity",
    }
}

/// Resolves a human-readable title and body for an event.
async fn describe(
    ctx: &AppContext,
    repository_id: i64,
    owner: &str,
    repo: &str,
    actor: &str,
    event: &str,
    payload: Option<&Value>,
) -> Result<(String, String)> {
    let number = payload
        .and_then(|value| value.get("number"))
        .and_then(Value::as_i64);
    let branch = payload
        .and_then(|value| value.get("branch"))
        .and_then(Value::as_str);
    let tag = payload
        .and_then(|value| value.get("tag"))
        .and_then(Value::as_str);

    let subject = if let Some(number) = number {
        let title = if event.starts_with("issue.") {
            issues::Entity::find()
                .filter(issues::Column::RepositoryId.eq(repository_id))
                .filter(issues::Column::Number.eq(number))
                .one(&ctx.db)
                .await?
                .map(|row| row.title)
        } else if event.starts_with("pr.") {
            pull_requests::Entity::find()
                .filter(pull_requests::Column::RepositoryId.eq(repository_id))
                .filter(pull_requests::Column::Number.eq(number))
                .one(&ctx.db)
                .await?
                .map(|row| row.title)
        } else {
            None
        };
        title.map(|title| format!("{title} (#{number})"))
    } else {
        None
    };

    let title = format!("{} in {}/{}", event_label(event), owner, repo);
    let mut body = format!("{actor} triggered `{event}`.");
    if let Some(subject) = subject {
        body.push_str(&format!("\n\n{subject}"));
    }
    if let Some(branch) = branch {
        body.push_str(&format!("\n\nBranch: `{branch}`"));
    }
    if let Some(tag) = tag {
        body.push_str(&format!("\n\nTag: `{tag}`"));
    }
    Ok((title, body))
}

/// Resolves the owner handle for a repository (user or organization).
async fn owner_of(db: &sea_orm::DatabaseConnection, repo: &repositories::Model) -> Result<String> {
    if let Some(organization_id) = repo.organization_id {
        Ok(organizations::Entity::find_by_id(organization_id)
            .one(db)
            .await?
            .map(|org| org.handle)
            .unwrap_or_else(|| "unknown".to_owned()))
    } else {
        Ok(users::Entity::find_by_id(repo.user_id)
            .one(db)
            .await?
            .map(|user| user.username)
            .unwrap_or_else(|| "unknown".to_owned()))
    }
}

/// Enqueues notifications for an activity event. No-op when the flag is off.
///
/// # Errors
/// Returns a database or queue error; callers log and swallow it so a broken
/// notification cannot fail the action that produced the event.
pub async fn dispatch(
    ctx: &AppContext,
    repository_id: i64,
    event: &str,
    actor_id: i64,
    payload: Option<&Value>,
) -> Result<()> {
    let settings = AppSettings::from_context(ctx)?;
    if !settings.notifications.enabled || settings.notifications.channels.is_empty() {
        return Ok(());
    }

    let mut watchers: Vec<i64> = watches::Entity::watcher_ids(&ctx.db, repository_id)
        .await?
        .into_iter()
        .filter(|id| *id != actor_id) // never notify the actor about their own action
        .collect();
    if watchers.is_empty() {
        return Ok(());
    }
    watchers.sort_unstable();
    watchers.dedup();

    let Some(repo) = repositories::Entity::find_by_id(repository_id)
        .one(&ctx.db)
        .await?
    else {
        return Ok(());
    };
    let owner = owner_of(&ctx.db, &repo).await?;
    let actor = users::Entity::find_by_id(actor_id)
        .one(&ctx.db)
        .await?
        .map(|user| user.username)
        .unwrap_or_else(|| "unknown".to_owned());
    let (title, body) = describe(
        ctx,
        repository_id,
        &owner,
        repo.display_name()?,
        &actor,
        event,
        payload,
    )
    .await?;

    let mut channels = Vec::new();
    for name in &settings.notifications.channels {
        if channel(name).is_some() {
            channels.push(name.clone());
        } else {
            tracing::warn!(channel = %name, "notification channel is not registered");
        }
    }

    for user_id in watchers {
        for name in &channels {
            let notification = crate::models::notifications::ActiveModel::create(
                &ctx.db,
                user_id,
                repository_id,
                event,
                &title,
                &body,
                name,
            )
            .await?;
            DeliverNotificationWorker::perform_later(
                ctx,
                WorkerArgs {
                    notification_id: notification.id,
                },
            )
            .await?;
        }
    }
    Ok(())
}
