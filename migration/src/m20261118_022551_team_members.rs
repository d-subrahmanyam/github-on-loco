use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum TeamMembers {
    Table,
    TeamId,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "team_members",
            &[("id", ColType::PkAuto)],
            &[("team", ""), ("user", "")],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-team-members-team-user")
                .table(TeamMembers::Table)
                .col(TeamMembers::TeamId)
                .col(TeamMembers::UserId)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "team_members").await
    }
}
