//! CLI tasks — the `rake task` equivalent. Run one with `cargo loco task <name>`.
//!
//! Recurring work is a task plus a `scheduler:` entry in `config/<env>.yaml`;
//! work triggered by a request belongs in a `workers/` background worker.

pub mod dev_reset;
pub mod user_create;
pub mod user_delete;
