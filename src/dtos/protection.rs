//! Wire types for branch protection and pull request reviews.

use ts_rs::TS;

/// One branch protection rule, as shown on the settings page.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct BranchProtectionResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub branch: String,
    /// Direct pushes are rejected unless the pusher is a repo admin.
    pub require_pull_request: bool,
    #[ts(type = "number")]
    pub required_approvals: i64,
}

/// Body of `POST /api/repos/{owner}/{repo}/branch-protection` (an upsert).
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct SetBranchProtectionParams {
    pub branch: String,
    pub require_pull_request: bool,
    #[ts(type = "number")]
    pub required_approvals: i64,
}

/// One reviewer's verdict on a pull request.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PrReviewResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub author: String,
    /// `approved`, `changes_requested` or `commented`.
    pub state: String,
    pub body: Option<String>,
    pub created_at: String,
}

/// Body of `POST /api/repos/{owner}/{repo}/pulls/{number}/reviews`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct SetReviewParams {
    pub state: String,
    pub body: Option<String>,
}
