use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Identifier helper for the per-repository uniqueness index.
#[derive(Iden)]
enum Issues {
    Table,
    RepositoryId,
    Number,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "issues",
            &[
                ("id", ColType::PkAuto),
                ("number", ColType::BigInteger),
                ("title", ColType::String),
                ("body", ColType::TextNull),
                ("state", ColType::String),
            ],
            &[
                ("repository", ""),
                // `author` is the display name of the column pair; the real
                // table is `users` and the FK column is `user_id`.
                ("users", "user_id"),
            ],
        )
        .await?;

        // Issue numbers are unique within a repository, not globally.
        m.create_index(
            Index::create()
                .name("idx-issues-repository-number")
                .table(Issues::Table)
                .col(Issues::RepositoryId)
                .col(Issues::Number)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "issues").await
    }
}
