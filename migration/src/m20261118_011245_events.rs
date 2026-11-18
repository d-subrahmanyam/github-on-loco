use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Events {
    Table,
    RepositoryId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "events",
            &[
                ("id", ColType::PkAuto),
                ("kind", ColType::String),
                ("payload", ColType::JsonBinaryNull),
            ],
            &[("repository", ""), ("user", "")],
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-events-repository")
                .table(Events::Table)
                .col(Events::RepositoryId)
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "events").await
    }
}
