use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "personal_access_tokens",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("token_hash", ColType::StringUniq),
                ("scopes", ColType::StringNull),
                ("expires_at", ColType::TimestampWithTimeZoneNull),
                ("last_used_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[("user", "")],
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "personal_access_tokens").await
    }
}
