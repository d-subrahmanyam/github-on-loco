//! Sea-ORM migrations, ordered oldest first.
//!
//! Add a migration with `cargo loco generate migration <name>`, then run
//! `cargo loco db migrate && cargo loco db entities`. The second command
//! regenerates `src/models/_entities/` from the live schema.

#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20261117_121915_repositories;
mod m20261117_122006_add_username_to_users;
mod m20261117_151206_personal_access_tokens;
mod m20261117_154653_branches;
mod m20261117_154742_commits;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20261117_121915_repositories::Migration),
            Box::new(m20261117_122006_add_username_to_users::Migration),
            Box::new(m20261117_151206_personal_access_tokens::Migration),
            Box::new(m20261117_154653_branches::Migration),
            Box::new(m20261117_154742_commits::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
