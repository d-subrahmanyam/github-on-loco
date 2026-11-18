//! Notification channel trait, built-in channels, and the runtime registry.

use async_trait::async_trait;
use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use crate::{
    mailers::notification::NotificationMailer,
    models::{notifications, users},
};

/// A delivery mechanism for a stored notification.
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// Configuration name, e.g. `email`, `slack`, `pagerduty`.
    fn name(&self) -> &'static str;

    /// Delivers one notification row.
    ///
    /// # Errors
    /// Returns an error on transport failure so the queue can retry.
    async fn send(&self, ctx: &AppContext, notification: &notifications::Model) -> Result<()>;
}

/// Development/default channel: writes the notification to the application log.
pub struct LogChannel;

#[async_trait]
impl NotificationChannel for LogChannel {
    fn name(&self) -> &'static str {
        "log"
    }

    async fn send(&self, _ctx: &AppContext, notification: &notifications::Model) -> Result<()> {
        tracing::info!(
            notification_id = notification.id,
            user_id = notification.user_id,
            event = %notification.event,
            title = %notification.title,
            body = %notification.body,
            "notification (log channel)"
        );
        Ok(())
    }
}

/// Email channel backed by Loco's mailer and the embedded template.
pub struct EmailChannel;

#[async_trait]
impl NotificationChannel for EmailChannel {
    fn name(&self) -> &'static str {
        "email"
    }

    async fn send(&self, ctx: &AppContext, notification: &notifications::Model) -> Result<()> {
        let Some(user) = users::Entity::find_by_id(notification.user_id)
            .one(&ctx.db)
            .await?
        else {
            return Err(Error::string("notification recipient no longer exists"));
        };
        NotificationMailer::send(ctx, &user.email, &notification.title, &notification.body).await
    }
}

struct Registry {
    channels: RwLock<HashMap<String, Arc<dyn NotificationChannel>>>,
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let mut channels: HashMap<String, Arc<dyn NotificationChannel>> = HashMap::new();
        channels.insert("log".to_owned(), Arc::new(LogChannel));
        channels.insert("email".to_owned(), Arc::new(EmailChannel));
        Registry {
            channels: RwLock::new(channels),
        }
    })
}

/// Adds or replaces a channel. New providers (Slack, Teams, PagerDuty, …) call
/// this during application startup.
///
/// # Errors
/// Returns an error if another thread holds the registry lock.
pub fn register_channel(channel: Arc<dyn NotificationChannel>) -> Result<()> {
    let mut channels = registry()
        .channels
        .write()
        .map_err(|_| Error::string("notification registry lock poisoned"))?;
    channels.insert(channel.name().to_owned(), channel);
    Ok(())
}

/// Looks up a channel by its configured name.
#[must_use]
pub fn channel(name: &str) -> Option<Arc<dyn NotificationChannel>> {
    registry()
        .channels
        .read()
        .ok()
        .and_then(|channels| channels.get(name).cloned())
}

/// Sorted names of all registered channels, for the settings endpoint.
#[must_use]
pub fn available_channels() -> Vec<String> {
    let Ok(channels) = registry().channels.read() else {
        return Vec::new();
    };
    let mut names: Vec<String> = channels.keys().cloned().collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeChannel;

    #[async_trait]
    impl NotificationChannel for FakeChannel {
        fn name(&self) -> &'static str {
            "fake-test-channel"
        }

        async fn send(&self, _ctx: &AppContext, _n: &notifications::Model) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn new_channels_can_be_registered_and_looked_up() {
        register_channel(Arc::new(FakeChannel)).unwrap();
        let found = channel("fake-test-channel").expect("channel should be registered");
        assert_eq!(found.name(), "fake-test-channel");
        assert!(channel("does-not-exist").is_none());
        assert!(available_channels().contains(&"fake-test-channel".to_owned()));
        assert!(available_channels().contains(&"email".to_owned()));
    }
}
