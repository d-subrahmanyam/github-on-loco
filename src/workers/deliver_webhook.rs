//! Delivers one queued webhook payload over HTTP.
//!
//! Only the delivery id is stored in the job; the worker reloads the current
//! rows so a deleted webhook or a retried delivery cannot act on stale state.
//! A non-2xx response or transport error returns `Err`, letting Loco's queue
//! retry up to `max_retries`; each attempt is recorded on the delivery row.

use loco_rs::prelude::*;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::models::{webhook_deliveries, webhooks};
use crate::services::webhooks::sign;

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {
    pub delivery_id: i64,
}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "DeliverWebhook".to_string()
    }

    fn max_retries() -> u32 {
        3
    }

    async fn perform(&self, args: WorkerArgs) -> Result<()> {
        let Some(delivery) = webhook_deliveries::Entity::find_by_id(args.delivery_id)
            .one(&self.ctx.db)
            .await?
        else {
            return Ok(()); // deleted before delivery — nothing to do
        };
        let Some(hook) = webhooks::Entity::find_by_id(delivery.webhook_id)
            .one(&self.ctx.db)
            .await?
        else {
            return Ok(());
        };

        let signature = sign(&hook.secret, &delivery.payload)?;
        let delivery_id = delivery.id.to_string();
        let event = delivery.event.clone();
        let body = delivery.payload.clone();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|err| Error::string(&format!("failed to build webhook client: {err}")))?;

        let response = client
            .post(&hook.url)
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, "github-on-loco-webhook/1.0")
            .header("X-GitHub-Event", &event)
            .header("X-GitHub-Delivery", &delivery_id)
            .header("X-Hub-Signature-256", &signature)
            .body(body)
            .send()
            .await;

        match response {
            Ok(response) if response.status().is_success() => {
                webhook_deliveries::ActiveModel::mark_success(&self.ctx.db, delivery).await?;
                Ok(())
            }
            Ok(response) => {
                let status = response.status();
                let message = format!("HTTP {status}");
                webhook_deliveries::ActiveModel::mark_failed(&self.ctx.db, delivery, &message)
                    .await?;
                Err(Error::string(&message))
            }
            Err(err) => {
                let message = format!("transport error: {err}");
                webhook_deliveries::ActiveModel::mark_failed(&self.ctx.db, delivery, &message)
                    .await?;
                Err(Error::string(&message))
            }
        }
    }
}
