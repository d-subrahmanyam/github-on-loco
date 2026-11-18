use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Forks {
    Table,
    Id,
    SourceRepositoryId,
    RepositoryId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Repositories {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Both columns point at `repositories`: one is the source, one is the
        // newly created fork. The generator cannot express two FKs to the same
        // table from differently named columns, so this migration is explicit.
        m.create_table(
            Table::create()
                .table(Forks::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Forks::Id)
                        .big_integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Forks::SourceRepositoryId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(Forks::RepositoryId).big_integer().not_null())
                .col(
                    ColumnDef::new(Forks::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Forks::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-forks-source-repository")
                        .from(Forks::Table, Forks::SourceRepositoryId)
                        .to(Repositories::Table, Repositories::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-forks-repository")
                        .from(Forks::Table, Forks::RepositoryId)
                        .to(Repositories::Table, Repositories::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        // A repository is a fork of at most one source.
        m.create_index(
            Index::create()
                .name("idx-forks-repository")
                .table(Forks::Table)
                .col(Forks::RepositoryId)
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-forks-source")
                .table(Forks::Table)
                .col(Forks::SourceRepositoryId)
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Forks::Table).to_owned())
            .await
    }
}
