use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Watches {
    Table,
    RepositoryId,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "watches",
            &[("id", ColType::PkAuto)],
            &[("repository", ""), ("user", "")],
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-watches-repository-user")
                .table(Watches::Table)
                .col(Watches::RepositoryId)
                .col(Watches::UserId)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "watches").await
    }
}
