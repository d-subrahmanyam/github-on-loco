//! Typed JSON contracts shared with the SPA.
//!
//! Every type here derives `ts-rs::TS` and exports a binding into
//! `frontend/src/bindings/`, so a change to the API contract is a compile-time
//! event on both sides.

pub mod collaborators;
pub mod common;
pub mod forks;
pub mod issues;
pub mod organizations;
pub mod protection;
pub mod pulls;
pub mod releases;
pub mod repositories;
pub mod search;
pub mod social;
pub mod tokens;
pub mod webhooks;
