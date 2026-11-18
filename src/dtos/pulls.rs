//! Wire types for the pull request API. Entities never go on the wire.

use ts_rs::TS;

use crate::dtos::repositories::Commit;

/// Body of `POST /api/repos/{owner}/{repo}/pulls`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreatePullRequestParams {
    pub title: String,
    pub body: Option<String>,
    /// Branch containing the changes.
    pub head_ref: String,
    /// Branch the changes are proposed against.
    pub base_ref: String,
}

/// Body of `POST /api/repos/{owner}/{repo}/pulls/{number}/comments`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AddReviewCommentParams {
    pub body: String,
    /// Optional file the comment is anchored to.
    pub path: Option<String>,
    /// Optional 1-based line number within that file.
    #[ts(type = "number | null")]
    pub line: Option<i64>,
}

/// One PR in a list or detail response.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PullRequestResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author: String,
    pub head_ref: String,
    pub head_sha: String,
    pub base_ref: String,
    #[ts(type = "number")]
    pub comment_count: i64,
    /// Approvals required by the base branch's protection rule (0 when none).
    #[ts(type = "number")]
    pub required_approvals: i64,
    /// Approving reviews from someone other than the PR author.
    #[ts(type = "number")]
    pub approval_count: i64,
    /// Why this PR cannot be merged right now; `null` when it can.
    #[ts(type = "string | null")]
    pub merge_blocked_reason: Option<String>,
    pub merged_at: Option<String>,
    pub merged_commit_sha: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A review comment on a PR.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ReviewCommentResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub body: String,
    pub author: String,
    pub path: Option<String>,
    #[ts(type = "number | null")]
    pub line: Option<i64>,
    pub created_at: String,
}

/// One changed file in the compare view.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DiffFile {
    pub path: String,
    /// `added`, `modified`, `deleted` or `renamed`.
    pub status: String,
    #[ts(type = "number")]
    pub additions: i64,
    #[ts(type = "number")]
    pub deletions: i64,
    /// Unified diff for this file, capped by the server.
    pub patch: String,
}

/// Result of `GET /api/repos/{owner}/{repo}/pulls/compare`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CompareResult {
    /// Merge base SHA the three-dot diff is measured from.
    pub base: String,
    /// Head SHA of the comparison.
    pub head: String,
    #[ts(type = "number")]
    pub ahead_by: i64,
    #[ts(type = "number")]
    pub behind_by: i64,
    /// Commits in head but not base.
    pub commits: Vec<Commit>,
    pub files: Vec<DiffFile>,
}
