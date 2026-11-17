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
}

impl AppSettings {
    pub fn from_context(ctx: &AppContext) -> Result<Self> {
        ctx.config.settings::<Self>()
    }
}
