use axum::http::{HeaderName, HeaderValue};
use github_loco::{models::users, views::auth::LoginResponse};
use loco_rs::{app::AppContext, TestServer};

const USER_EMAIL: &str = "test@loco.com";
const USER_PASSWORD: &str = "1234";

pub struct LoggedInUser {
    pub user: users::Model,
    pub token: String,
}

/// Registers, verifies and logs in a user with explicit details. Request tests
/// are serial and the test DB is recreated per boot, so emails never collide.
pub async fn register_and_login(
    request: &TestServer,
    ctx: &AppContext,
    email: &str,
    name: &str,
    username: Option<&str>,
    password: &str,
) -> LoggedInUser {
    let mut payload = serde_json::json!({
        "name": name,
        "email": email,
        "password": password
    });
    if let Some(username) = username {
        payload["username"] = serde_json::json!(username);
    }

    request.post("/api/auth/register").json(&payload).await;
    let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();

    request
        .post("/api/auth/verify")
        .json(&serde_json::json!({ "token": user.email_verification_token }))
        .await;

    let response = request
        .post("/api/auth/login")
        .json(&serde_json::json!({ "email": email, "password": password }))
        .await;
    let login_response: LoginResponse = serde_json::from_str(&response.text()).unwrap();

    LoggedInUser {
        user: users::Model::find_by_email(&ctx.db, email).await.unwrap(),
        token: login_response.token,
    }
}

pub async fn init_user_login(request: &TestServer, ctx: &AppContext) -> LoggedInUser {
    register_and_login(request, ctx, USER_EMAIL, "loco", None, USER_PASSWORD).await
}

pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
