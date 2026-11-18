use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Repositories {
    Table,
    OrganizationId,
}

#[derive(Iden)]
enum Organizations {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // `user_id` stays as the creator/audit field. A non-null
        // `organization_id` means the organisation, not the creator, owns the
        // repository — that is the single switch the authorization rule reads.
        m.alter_table(
            Table::alter()
                .table(Repositories::Table)
                .add_column(
                    ColumnDef::new(Repositories::OrganizationId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;
        m.create_foreign_key(
            ForeignKey::create()
                .name("fk-repositories-organization")
                .from(Repositories::Table, Repositories::OrganizationId)
                .to(Organizations::Table, Organizations::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-repositories-organization")
                .table(Repositories::Table)
                .col(Repositories::OrganizationId)
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Repositories::Table)
                .drop_column(Repositories::OrganizationId)
                .to_owned(),
        )
        .await
    }
}
