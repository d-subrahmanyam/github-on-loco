//! Thin HTTP handlers. Domain decisions belong in `models/`.
//!
//! `auth` is the Loco starter's JWT endpoints, `repositories` is the
//! create/list API, `git_http` implements git's smart-HTTP transport, and
//! `git_browse` exposes read-only code browsing.

pub mod auth;
pub mod git_browse;
pub mod git_http;
pub mod repositories;
