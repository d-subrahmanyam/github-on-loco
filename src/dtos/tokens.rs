//! Wire types for personal access token management. The token hash is never
//! serialized; the plaintext is returned exactly once on creation.

use ts_rs::TS;

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateTokenParams {
    pub name: String,
    #[serde(default)]
    pub scopes: Option<String>,
    /// Optional lifetime in days; absent means the token never expires.
    #[serde(default)]
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TokenResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub scopes: Option<String>,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct NewTokenResponse {
    pub token: String,
    pub info: TokenResponse,
}
