use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Teams {
    Table,
    OrganizationId,
    Name,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "teams",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("description", ColType::StringNull),
            ],
            &[("organization", "")],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-teams-organization-name")
                .table(Teams::Table)
                .col(Teams::OrganizationId)
                .col(Teams::Name)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "teams").await
    }
}
