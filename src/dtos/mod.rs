//! Typed JSON contracts shared with the SPA.
//!
//! Every type here derives `ts-rs::TS` and exports a binding into
//! `frontend/src/bindings/`, so a change to the API contract is a compile-time
//! event on both sides.

pub mod common;
pub mod issues;
pub mod pulls;
pub mod repositories;
pub mod tokens;
