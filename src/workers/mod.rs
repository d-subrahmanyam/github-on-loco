//! Background workers for work that must outlive an HTTP request.
//!
//! Register a new worker in `App::connect_workers` (the generator does this).
//! Never substitute `tokio::spawn`: it is not durable, retried or observable.

pub mod deliver_webhook;
pub mod downloader;

pub mod merge_pull_request;
