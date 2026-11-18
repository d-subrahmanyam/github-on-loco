//! Active Record-style domain models.
//!
//! `models/<name>.rs` is where finders, state transitions and validation go.
//! `models/_entities/` is generated from the live database schema by
//! `cargo loco db entities` and must never be edited by hand.

pub mod _entities;
pub mod branch_protection_rules;
pub mod branches;
pub mod collaborators;
pub mod commits;
pub mod events;
pub mod forks;
pub mod issue_comments;
pub mod issue_labels;
pub mod issues;
pub mod labels;
pub mod personal_access_tokens;
pub mod pr_reviews;
pub mod pull_requests;
pub mod repositories;
pub mod review_comments;
pub mod stars;
pub mod users;
pub mod watches;
