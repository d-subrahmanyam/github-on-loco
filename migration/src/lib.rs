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
mod m20261117_173120_issues;
mod m20261117_173244_issue_comments;
mod m20261117_173330_labels;
mod m20261117_173417_issue_labels;
mod m20261117_192611_pull_requests;
mod m20261117_192708_review_comments;
mod m20261117_200241_collaborators;
mod m20261118_011005_stars;
mod m20261118_011120_watches;
mod m20261118_011245_events;
mod m20261118_013846_branch_protection_rules;
mod m20261118_014015_pr_reviews;
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
            Box::new(m20261117_173120_issues::Migration),
            Box::new(m20261117_173244_issue_comments::Migration),
            Box::new(m20261117_173330_labels::Migration),
            Box::new(m20261117_173417_issue_labels::Migration),
            Box::new(m20261117_192611_pull_requests::Migration),
            Box::new(m20261117_192708_review_comments::Migration),
            Box::new(m20261117_200241_collaborators::Migration),
            Box::new(m20261118_011005_stars::Migration),
            Box::new(m20261118_011120_watches::Migration),
            Box::new(m20261118_011245_events::Migration),
            Box::new(m20261118_013846_branch_protection_rules::Migration),
            Box::new(m20261118_014015_pr_reviews::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
