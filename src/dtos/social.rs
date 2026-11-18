//! Wire types for stars, watches and the activity feed.

use ts_rs::TS;

/// Counts and the viewer's own state after a star/watch toggle.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct RepoStatsResponse {
    #[ts(type = "number")]
    pub star_count: i64,
    #[ts(type = "number")]
    pub watch_count: i64,
    pub starred: bool,
    pub watching: bool,
}

/// One entry in a repository's activity feed.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct EventResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub actor: String,
    pub kind: String,
    /// Human-readable sentence built from the kind and its small payload.
    pub text: String,
    pub created_at: String,
}
