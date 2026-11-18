use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum BranchProtectionRules {
    Table,
    RepositoryId,
    Branch,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "branch_protection_rules",
            &[
                ("id", ColType::PkAuto),
                ("branch", ColType::String),
                ("require_pull_request", ColType::Boolean),
                ("required_approvals", ColType::BigInteger),
            ],
            &[("repository", "")],
        )
        .await?;

        // One rule per branch: the settings page upserts rather than duplicating.
        m.create_index(
            Index::create()
                .name("idx-branch-protection-repository-branch")
                .table(BranchProtectionRules::Table)
                .col(BranchProtectionRules::RepositoryId)
                .col(BranchProtectionRules::Branch)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "branch_protection_rules").await
    }
}
