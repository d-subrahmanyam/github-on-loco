//! Pluggable notification delivery.
//!
//! A channel is anything that can deliver a stored notification: email today,
//! Slack/Teams/PagerDuty later. Channels register by name; configuration picks
//! which names receive fan-out. Adding a provider requires only a new
//! implementation of [`NotificationChannel`] plus one `register_channel` call.

pub mod channels;

pub use channels::{available_channels, channel, register_channel, NotificationChannel};
