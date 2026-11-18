//! Typed view over the `settings:` block of `config/<env>.yaml`.
//!
//! Application code reads configuration through `ctx.config`; it must never
//! call `std::env::var` directly. The YAML file does the environment lookup
//! with `get_env(...)` interpolation.

use loco_rs::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub repo_base_dir: String,
    /// Feature-flagged notification fan-out. Disabled by default so no mail
    /// server or third-party credentials are needed to run the app.
    #[serde(default)]
    pub notifications: NotificationSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationSettings {
    #[serde(default)]
    pub enabled: bool,
    /// Names resolved against the notification channel registry.
    #[serde(default = "default_channels")]
    pub channels: Vec<String>,
}

fn default_channels() -> Vec<String> {
    vec!["email".to_owned()]
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            channels: default_channels(),
        }
    }
}

impl AppSettings {
    pub fn from_context(ctx: &AppContext) -> Result<Self> {
        ctx.config.settings::<Self>()
    }
}
