use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum TeamRepositories {
    Table,
    TeamId,
    RepositoryId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "team_repositories",
            &[("id", ColType::PkAuto), ("role", ColType::String)],
            &[("team", ""), ("repository", "")],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-team-repositories-team-repository")
                .table(TeamRepositories::Table)
                .col(TeamRepositories::TeamId)
                .col(TeamRepositories::RepositoryId)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "team_repositories").await
    }
}
