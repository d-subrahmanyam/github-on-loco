//! Infrastructure helpers that are not domain models.
//!
//! Settings come from `config/<env>.yaml` via [`settings::AppSettings`]; git
//! filesystem access is isolated in [`git`] so controllers never shell out to
//! `git` themselves.

pub mod git;
pub mod settings;
