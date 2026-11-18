use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum WebhookDeliveries {
    Table,
    Id,
    WebhookId,
    Event,
    Payload,
    Status,
    Attempts,
    LastError,
    DeliveredAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Webhooks {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // One row per attempted delivery; the queue worker updates it so the
        // SPA can show a delivery history and failed attempts can be retried.
        m.create_table(
            Table::create()
                .table(WebhookDeliveries::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(WebhookDeliveries::Id)
                        .big_integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(WebhookDeliveries::WebhookId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(WebhookDeliveries::Event).string().not_null())
                .col(ColumnDef::new(WebhookDeliveries::Payload).text().not_null())
                .col(
                    ColumnDef::new(WebhookDeliveries::Status)
                        .string()
                        .not_null()
                        .default("pending"),
                )
                .col(
                    ColumnDef::new(WebhookDeliveries::Attempts)
                        .integer()
                        .not_null()
                        .default(0),
                )
                .col(ColumnDef::new(WebhookDeliveries::LastError).text().null())
                .col(
                    ColumnDef::new(WebhookDeliveries::DeliveredAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(WebhookDeliveries::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(WebhookDeliveries::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-webhook-deliveries-webhook")
                        .from(WebhookDeliveries::Table, WebhookDeliveries::WebhookId)
                        .to(Webhooks::Table, Webhooks::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-webhook-deliveries-webhook")
                .table(WebhookDeliveries::Table)
                .col(WebhookDeliveries::WebhookId)
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(WebhookDeliveries::Table).to_owned())
            .await
    }
}
