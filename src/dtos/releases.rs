//! Wire types for releases and git tags.

use ts_rs::TS;

/// A git tag as reported by `git for-each-ref`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TagResponse {
    pub name: String,
    /// Commit SHA the tag points at.
    pub sha: String,
    /// ISO-8601 date, or an empty string when git has none for the tag.
    pub created_at: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ReleaseResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub tag_name: String,
    pub title: String,
    pub body: Option<String>,
    pub author: String,
    pub is_prerelease: bool,
    pub published_at: String,
    /// Commit the tag resolved to when the release was created.
    pub commit_sha: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateReleaseParams {
    pub tag_name: String,
    /// Branch name or commit SHA the tag should point at.
    pub target: String,
    pub title: String,
    pub body: Option<String>,
    pub is_prerelease: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateReleaseParams {
    pub title: String,
    pub body: Option<String>,
    pub is_prerelease: bool,
}
