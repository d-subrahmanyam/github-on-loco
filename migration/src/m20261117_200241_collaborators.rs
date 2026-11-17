use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Identifier helper for the per-repository uniqueness index.
#[derive(Iden)]
enum Collaborators {
    Table,
    RepositoryId,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "collaborators",
            &[("id", ColType::PkAuto), ("role", ColType::String)],
            &[("repository", ""), ("user", "")],
        )
        .await?;

        // A user has at most one role in a given repository.
        m.create_index(
            Index::create()
                .name("idx-collaborators-repository-user")
                .table(Collaborators::Table)
                .col(Collaborators::RepositoryId)
                .col(Collaborators::UserId)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "collaborators").await
    }
}
