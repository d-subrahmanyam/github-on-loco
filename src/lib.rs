//! A GitHub-like code hosting app built with [Loco](https://loco.rs).
//!
//! The architecture is described in `docs/architecture.md`; the conventions
//! every contribution follows are in `docs/design-practices.md`. In short:
//!
//! * **Git is the source of truth for repository contents**; Postgres holds
//!   metadata, ownership and (later) social features.
//! * **Fat models, slim controllers.** Handlers parse, call a model method,
//!   render a DTO.
//! * **Everything configurable lives in `config/<env>.yaml`** and is read via
//!   `ctx.config` — never `std::env::var` in application code.

pub mod app;
pub mod controllers;
pub mod data;
pub mod dtos;
pub mod initializers;
pub mod mailers;
pub mod models;
pub mod notifications;
pub mod services;
pub mod tasks;
pub mod views;
pub mod workers;
