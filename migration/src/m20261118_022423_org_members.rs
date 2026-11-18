use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum OrgMembers {
    Table,
    OrganizationId,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "org_members",
            &[("id", ColType::PkAuto), ("role", ColType::String)],
            &[("organization", ""), ("user", "")],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-org-members-organization-user")
                .table(OrgMembers::Table)
                .col(OrgMembers::OrganizationId)
                .col(OrgMembers::UserId)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "org_members").await
    }
}
