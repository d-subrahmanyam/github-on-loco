//! Delivers one queued notification through its configured channel.

use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::models::notifications;
use crate::notifications::channel;

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {
    pub notification_id: i64,
}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "DeliverNotification".to_string()
    }

    fn max_retries() -> u32 {
        3
    }

    async fn perform(&self, args: WorkerArgs) -> Result<()> {
        let Some(notification) = notifications::Entity::find_by_id(args.notification_id)
            .one(&self.ctx.db)
            .await?
        else {
            return Ok(());
        };
        let Some(provider) = channel(&notification.channel) else {
            let message = format!("unknown notification channel: {}", notification.channel);
            notifications::ActiveModel::mark_failed(&self.ctx.db, notification, &message).await?;
            return Err(Error::string(&message));
        };

        match provider.send(&self.ctx, &notification).await {
            Ok(()) => {
                notifications::ActiveModel::mark_sent(&self.ctx.db, notification).await?;
                Ok(())
            }
            Err(err) => {
                let message = err.to_string();
                notifications::ActiveModel::mark_failed(&self.ctx.db, notification, &message)
                    .await?;
                Err(err)
            }
        }
    }
}
