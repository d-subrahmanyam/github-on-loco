use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum PrReviews {
    Table,
    PullRequestId,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "pr_reviews",
            &[
                ("id", ColType::PkAuto),
                ("state", ColType::String),
                ("body", ColType::StringNull),
            ],
            &[("pull_request", ""), ("user", "")],
        )
        .await?;

        // A reviewer can change their verdict, but has only one active review.
        m.create_index(
            Index::create()
                .name("idx-pr-reviews-pull-request-user")
                .table(PrReviews::Table)
                .col(PrReviews::PullRequestId)
                .col(PrReviews::UserId)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "pr_reviews").await
    }
}
