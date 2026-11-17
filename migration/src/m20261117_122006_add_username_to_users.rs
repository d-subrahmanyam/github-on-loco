//! Adds the canonical public handle (`username`) to `users`.
//!
//! `users.name` is deliberately kept as the *display* name. Repo URLs, git
//! remotes and all ownership checks use `username`, which is URL-safe and
//! unique. The column is added nullable first so existing rows can be
//! backfilled, then promoted to `NOT NULL` and given a unique index.

use sea_orm_migration::prelude::*;

/// Identifier helper for the table and the one column this migration touches.
#[derive(Iden)]
enum Users {
    Table,
    Username,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add the column as nullable so the backfill cannot fail.
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::Username).string().null())
                    .to_owned(),
            )
            .await?;

        // 2. Backfill existing rows. `user-<id>` is guaranteed unique even when
        //    display names are not, so creating the unique index below is safe.
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE users SET username = 'user-' || id::text WHERE username IS NULL",
            )
            .await?;

        // 3. Now that every row has a value, tighten the column and index it.
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::Username).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-users-username")
                    .table(Users::Table)
                    .col(Users::Username)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx-users-username")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Username)
                    .to_owned(),
            )
            .await
    }
}
