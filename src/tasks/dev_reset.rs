//! `dev:reset` — reset the development database and git storage together.
//!
//! Loco's `db reset` only recreates Postgres. Git data lives on disk under
//! `repo_base_dir`, so a plain reset leaves orphaned bare repositories behind.
//! Recreating a metadata row with the same `owner/name` would then silently
//! reattach to that old history (`git init --bare` succeeds on an existing
//! repo). This task clears both stores and re-seeds the fixture users so the
//! two can never disagree.

use loco_rs::{db, environment::Environment, prelude::*};
use migration::Migrator;
use std::path::Path;

use crate::data::{git, settings::AppSettings};
use crate::models::_entities::users;

pub struct DevReset;

#[async_trait]
impl Task for DevReset {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "dev:reset".to_string(),
            detail: "Development only: recreate the database schema, delete all bare \
                     repositories on disk, and re-seed the fixture users."
                .to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        if ctx.environment == Environment::Production {
            return Err(Error::string("dev:reset is not allowed in production"));
        }

        // Same schema reset as `cargo loco db reset`, followed by the on-disk
        // half that Loco knows nothing about.
        db::reset::<Migrator>(&ctx.db).await?;
        tracing::warn!("database recreated");

        let settings = AppSettings::from_context(ctx)?;
        git::clear_repo_storage(&settings.repo_base_dir).await?;
        tracing::warn!(path = %settings.repo_base_dir, "git storage cleared");

        // Restore the accounts so a reset leaves the app immediately usable.
        let fixture = Path::new("src/fixtures").join("users.yaml");
        db::seed::<users::ActiveModel>(&ctx.db, &fixture.display().to_string()).await?;
        tracing::warn!("fixture users re-seeded");

        Ok(())
    }
}
