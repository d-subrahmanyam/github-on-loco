use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Organizations {
    Table,
    Handle,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "organizations",
            &[
                ("id", ColType::PkAuto),
                ("handle", ColType::String),
                ("display_name", ColType::String),
                ("description", ColType::StringNull),
            ],
            &[],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-organizations-handle")
                .table(Organizations::Table)
                .col(Organizations::Handle)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "organizations").await
    }
}
