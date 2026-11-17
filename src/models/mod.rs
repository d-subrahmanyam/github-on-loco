//! Active Record-style domain models.
//!
//! `models/<name>.rs` is where finders, state transitions and validation go.
//! `models/_entities/` is generated from the live database schema by
//! `cargo loco db entities` and must never be edited by hand.

pub mod _entities;
pub mod branches;
pub mod commits;
pub mod personal_access_tokens;
pub mod repositories;
pub mod users;
