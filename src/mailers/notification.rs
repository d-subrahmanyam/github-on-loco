//! Transactional email for activity notifications. Template is embedded at
//! compile time, like the auth mailer.

#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

static generic: include_dir::Dir<'_> = include_dir!("src/mailers/notification/generic");

#[allow(clippy::module_name_repetitions)]
pub struct NotificationMailer {}
impl Mailer for NotificationMailer {}

impl NotificationMailer {
    /// Sends one notification email.
    ///
    /// # Errors
    /// Returns an error if the template cannot be rendered or mail delivery fails.
    pub async fn send(ctx: &AppContext, to: &str, title: &str, body: &str) -> Result<()> {
        Self::mail_template(
            ctx,
            &generic,
            mailer::Args {
                to: to.to_owned(),
                locals: json!({
                    "title": title,
                    "body": body,
                    "host": ctx.config.server.full_url(),
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
