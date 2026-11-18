//! Thin HTTP handlers. Domain decisions belong in `models/`.
//!
//! `auth` is the Loco starter's JWT endpoints, `repositories` is the
//! create/list API, `git_http` implements git's smart-HTTP transport, and
//! `git_browse` exposes read-only code browsing.

pub mod auth;
pub mod collaborators;
pub mod forks;
pub mod git_browse;
pub mod git_http;
pub mod repositories;

pub mod issues;
pub mod organizations;
pub mod protection;
pub mod pulls;
pub mod releases;
pub mod social;

pub mod tokens;
pub mod webhooks;
