use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Jobs::JobId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Jobs::JobKind).string().not_null())
                    .col(ColumnDef::new(Jobs::Status).string().not_null())
                    .col(
                        ColumnDef::new(Jobs::RequiredCapabilities)
                            .array(ColumnType::String(StringLen::None))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Jobs::Payload).json_binary().not_null())
                    .col(ColumnDef::new(Jobs::AssignedWorker).string())
                    .col(ColumnDef::new(Jobs::Result).json_binary())
                    .col(ColumnDef::new(Jobs::Error).string())
                    .col(
                        ColumnDef::new(Jobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Jobs::StartedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Jobs::FinishedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Workers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Workers::WorkerId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Workers::Name).string().not_null())
                    .col(
                        ColumnDef::new(Workers::Capabilities)
                            .array(ColumnType::String(StringLen::None))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Workers::Status).string().not_null())
                    .col(
                        ColumnDef::new(Workers::LastSeenAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Workers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    JobId,
    JobKind,
    Status,
    RequiredCapabilities,
    Payload,
    AssignedWorker,
    Result,
    Error,
    CreatedAt,
    StartedAt,
    FinishedAt,
}

#[derive(DeriveIden)]
enum Workers {
    Table,
    WorkerId,
    Name,
    Capabilities,
    Status,
    LastSeenAt,
}
