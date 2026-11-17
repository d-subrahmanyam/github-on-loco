use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Identifier helper for the per-repository uniqueness index.
#[derive(Iden)]
enum PullRequests {
    Table,
    RepositoryId,
    Number,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "pull_requests",
            &[
                ("id", ColType::PkAuto),
                ("number", ColType::BigInteger),
                ("title", ColType::String),
                ("body", ColType::TextNull),
                ("state", ColType::String),
                ("head_ref", ColType::String),
                ("head_sha", ColType::String),
                ("base_ref", ColType::String),
                ("merged_commit_sha", ColType::StringNull),
                ("merged_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[("repository", ""), ("user", "")],
        )
        .await?;

        // PR numbers are unique within a repository, not globally.
        m.create_index(
            Index::create()
                .name("idx-pull_requests-repository-number")
                .table(PullRequests::Table)
                .col(PullRequests::RepositoryId)
                .col(PullRequests::Number)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "pull_requests").await
    }
}
