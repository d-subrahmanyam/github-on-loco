use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Webhooks {
    Table,
    Id,
    RepositoryId,
    Url,
    Events,
    Secret,
    IsActive,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Repositories {
    Table,
    Id,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // `events` is a comma-separated list of event kinds (or `*`), and
        // `secret` is the HMAC key used to sign delivery payloads.
        m.create_table(
            Table::create()
                .table(Webhooks::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Webhooks::Id)
                        .big_integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Webhooks::RepositoryId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(Webhooks::Url).text().not_null())
                .col(ColumnDef::new(Webhooks::Events).text().not_null())
                .col(ColumnDef::new(Webhooks::Secret).string().not_null())
                .col(
                    ColumnDef::new(Webhooks::IsActive)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(ColumnDef::new(Webhooks::CreatedBy).big_integer().not_null())
                .col(
                    ColumnDef::new(Webhooks::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Webhooks::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-webhooks-repository")
                        .from(Webhooks::Table, Webhooks::RepositoryId)
                        .to(Repositories::Table, Repositories::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-webhooks-created-by")
                        .from(Webhooks::Table, Webhooks::CreatedBy)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-webhooks-repository-url")
                .table(Webhooks::Table)
                .col(Webhooks::RepositoryId)
                .col(Webhooks::Url)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Webhooks::Table).to_owned())
            .await
    }
}
