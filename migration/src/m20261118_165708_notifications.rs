use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Notifications {
    Table,
    Id,
    UserId,
    RepositoryId,
    Event,
    Title,
    Body,
    Channel,
    Status,
    Attempts,
    LastError,
    DeliveredAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(Iden)]
enum Repositories {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // One row per recipient/channel pair. The channel name is resolved to
        // a pluggable implementation by the delivery worker, so adding Slack
        // or PagerDuty later needs no schema change.
        m.create_table(
            Table::create()
                .table(Notifications::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Notifications::Id)
                        .big_integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Notifications::UserId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Notifications::RepositoryId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(Notifications::Event).string().not_null())
                .col(ColumnDef::new(Notifications::Title).string().not_null())
                .col(ColumnDef::new(Notifications::Body).text().not_null())
                .col(ColumnDef::new(Notifications::Channel).string().not_null())
                .col(
                    ColumnDef::new(Notifications::Status)
                        .string()
                        .not_null()
                        .default("pending"),
                )
                .col(
                    ColumnDef::new(Notifications::Attempts)
                        .integer()
                        .not_null()
                        .default(0),
                )
                .col(ColumnDef::new(Notifications::LastError).text().null())
                .col(
                    ColumnDef::new(Notifications::DeliveredAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(Notifications::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Notifications::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-notifications-user")
                        .from(Notifications::Table, Notifications::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-notifications-repository")
                        .from(Notifications::Table, Notifications::RepositoryId)
                        .to(Repositories::Table, Repositories::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-notifications-user")
                .table(Notifications::Table)
                .col(Notifications::UserId)
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-notifications-status")
                .table(Notifications::Table)
                .col(Notifications::Status)
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Notifications::Table).to_owned())
            .await
    }
}
