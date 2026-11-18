use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Releases {
    Table,
    Id,
    RepositoryId,
    TagName,
    Title,
    Body,
    AuthorId,
    IsPrerelease,
    PublishedAt,
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
        // Git tags are authoritative for the ref; this row only holds release
        // metadata. `tag_name` is unique per repository so a release maps to
        // exactly one tag.
        m.create_table(
            Table::create()
                .table(Releases::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Releases::Id)
                        .big_integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Releases::RepositoryId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(Releases::TagName).string().not_null())
                .col(ColumnDef::new(Releases::Title).string().not_null())
                .col(ColumnDef::new(Releases::Body).text().null())
                .col(ColumnDef::new(Releases::AuthorId).big_integer().not_null())
                .col(
                    ColumnDef::new(Releases::IsPrerelease)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(Releases::PublishedAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(Releases::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Releases::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-releases-repository")
                        .from(Releases::Table, Releases::RepositoryId)
                        .to(Repositories::Table, Repositories::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-releases-author")
                        .from(Releases::Table, Releases::AuthorId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-releases-repository-tag")
                .table(Releases::Table)
                .col(Releases::RepositoryId)
                .col(Releases::TagName)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Releases::Table).to_owned())
            .await
    }
}
