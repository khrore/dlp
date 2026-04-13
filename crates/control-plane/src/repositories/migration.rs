#![allow(elided_lifetimes_in_paths)]

use sea_orm_migration::prelude::*;

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260410_000001_create_storage_schema::Migration)]
    }
}

mod m20260410_000001_create_storage_schema {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub(super) struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            // PostgreSQL sequence DDL is backend-specific and clearer here than forcing it
            // through a partial abstraction, so migrations keep this raw SQL narrowly
            // scoped.
            manager
                .get_connection()
                .execute_unprepared(
                    r#"
                    CREATE SEQUENCE IF NOT EXISTS deployment_id_seq START WITH 1 INCREMENT BY 1;
                    CREATE SEQUENCE IF NOT EXISTS replica_id_seq START WITH 1 INCREMENT BY 1;
                    CREATE SEQUENCE IF NOT EXISTS lease_id_seq START WITH 1 INCREMENT BY 1;
                    "#,
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Deployments::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Deployments::Id)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Deployments::Name).string().not_null())
                        .col(ColumnDef::new(Deployments::ArtifactRef).string().not_null())
                        .col(
                            ColumnDef::new(Deployments::ReplicasDesired)
                                .integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Deployments::Framework).string().not_null())
                        .col(ColumnDef::new(Deployments::Mode).string().not_null())
                        .col(ColumnDef::new(Deployments::Device).string().not_null())
                        .col(
                            ColumnDef::new(Deployments::AcceleratorRuntime)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::ArchitectureFamily)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::MemoryRequirementBytes)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::ConcurrencyRequirement)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::PendingReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::AssignedReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::PullingReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::StartingReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::ReadyReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::FailedReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::StoppedReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Deployments::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Replicas::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Replicas::Id)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Replicas::DeploymentId).string().not_null())
                        .col(ColumnDef::new(Replicas::State).string().not_null())
                        .col(ColumnDef::new(Replicas::AssignedWorkerId).string().null())
                        .col(ColumnDef::new(Replicas::LeaseId).string().null())
                        .col(ColumnDef::new(Replicas::StatusMessage).string().null())
                        .col(
                            ColumnDef::new(Replicas::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Replicas::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_replicas_deployment_id")
                                .from(Replicas::Table, Replicas::DeploymentId)
                                .to(Deployments::Table, Deployments::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Workers::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Workers::Id)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Workers::DisplayName).string().not_null())
                        .col(ColumnDef::new(Workers::State).string().not_null())
                        .col(
                            ColumnDef::new(Workers::AssignedReplicas)
                                .integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Workers::AvailableSlots).integer().not_null())
                        .col(
                            ColumnDef::new(Workers::LastHeartbeatAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Workers::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Workers::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(WorkerCapabilities::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(WorkerCapabilities::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::WorkerId)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::Framework)
                                .string()
                                .not_null(),
                        )
                        .col(ColumnDef::new(WorkerCapabilities::Mode).string().not_null())
                        .col(
                            ColumnDef::new(WorkerCapabilities::Device)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::AcceleratorRuntime)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::ArchitectureFamily)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::AvailableMemoryBytes)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::ConcurrencySlots)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerCapabilities::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_worker_capabilities_worker_id")
                                .from(WorkerCapabilities::Table, WorkerCapabilities::WorkerId)
                                .to(Workers::Table, Workers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Leases::Table)
                        .if_not_exists()
                        .col(ColumnDef::new(Leases::Id).string().not_null().primary_key())
                        .col(ColumnDef::new(Leases::WorkerId).string().not_null())
                        .col(ColumnDef::new(Leases::DeploymentId).string().not_null())
                        .col(ColumnDef::new(Leases::ReplicaId).string().not_null())
                        .col(ColumnDef::new(Leases::State).string().not_null())
                        .col(ColumnDef::new(Leases::Framework).string().not_null())
                        .col(ColumnDef::new(Leases::Mode).string().not_null())
                        .col(ColumnDef::new(Leases::Device).string().not_null())
                        .col(
                            ColumnDef::new(Leases::AcceleratorRuntime)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Leases::ArchitectureFamily)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Leases::MemoryRequirementBytes)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Leases::ConcurrencyRequirement)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Leases::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Leases::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_leases_worker_id")
                                .from(Leases::Table, Leases::WorkerId)
                                .to(Workers::Table, Workers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_leases_deployment_id")
                                .from(Leases::Table, Leases::DeploymentId)
                                .to(Deployments::Table, Deployments::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_leases_replica_id")
                                .from(Leases::Table, Leases::ReplicaId)
                                .to(Replicas::Table, Replicas::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(WorkerAssignments::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(WorkerAssignments::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(WorkerAssignments::WorkerId)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerAssignments::ReplicaId)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerAssignments::LeaseId)
                                .string()
                                .not_null(),
                        )
                        .col(ColumnDef::new(WorkerAssignments::Payload).json().not_null())
                        .col(
                            ColumnDef::new(WorkerAssignments::DeliveryState)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(WorkerAssignments::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_worker_assignments_worker_id")
                                .from(WorkerAssignments::Table, WorkerAssignments::WorkerId)
                                .to(Workers::Table, Workers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_worker_assignments_replica_id")
                                .from(WorkerAssignments::Table, WorkerAssignments::ReplicaId)
                                .to(Replicas::Table, Replicas::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_worker_assignments_lease_id")
                                .from(WorkerAssignments::Table, WorkerAssignments::LeaseId)
                                .to(Leases::Table, Leases::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            for index in [
                Index::create()
                    .name("idx_replicas_deployment_id")
                    .table(Replicas::Table)
                    .col(Replicas::DeploymentId)
                    .to_owned(),
                Index::create()
                    .name("idx_replicas_state")
                    .table(Replicas::Table)
                    .col(Replicas::State)
                    .to_owned(),
                Index::create()
                    .name("idx_leases_worker_id_state")
                    .table(Leases::Table)
                    .col(Leases::WorkerId)
                    .col(Leases::State)
                    .to_owned(),
                Index::create()
                    .name("idx_workers_state")
                    .table(Workers::Table)
                    .col(Workers::State)
                    .to_owned(),
                Index::create()
                    .name("idx_workers_last_heartbeat_at")
                    .table(Workers::Table)
                    .col(Workers::LastHeartbeatAt)
                    .to_owned(),
                Index::create()
                    .name("idx_worker_assignments_worker_order")
                    .table(WorkerAssignments::Table)
                    .col(WorkerAssignments::WorkerId)
                    .col(WorkerAssignments::CreatedAt)
                    .col(WorkerAssignments::Id)
                    .to_owned(),
            ] {
                manager.create_index(index).await?;
            }

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(
                    Table::drop()
                        .table(WorkerAssignments::Table)
                        .if_exists()
                        .to_owned(),
                )
                .await?;
            manager
                .drop_table(Table::drop().table(Leases::Table).if_exists().to_owned())
                .await?;
            manager
                .drop_table(
                    Table::drop()
                        .table(WorkerCapabilities::Table)
                        .if_exists()
                        .to_owned(),
                )
                .await?;
            manager
                .drop_table(Table::drop().table(Workers::Table).if_exists().to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(Replicas::Table).if_exists().to_owned())
                .await?;
            manager
                .drop_table(
                    Table::drop()
                        .table(Deployments::Table)
                        .if_exists()
                        .to_owned(),
                )
                .await?;

            manager
                .get_connection()
                // PostgreSQL sequence DDL is backend-specific and clearer here than forcing it
                // through a partial abstraction, so migrations keep this raw SQL narrowly scoped.
                .execute_unprepared(
                    r#"
                    DROP SEQUENCE IF EXISTS deployment_id_seq;
                    DROP SEQUENCE IF EXISTS replica_id_seq;
                    DROP SEQUENCE IF EXISTS lease_id_seq;
                    "#,
                )
                .await?;

            Ok(())
        }
    }

    #[derive(DeriveIden)]
    enum Deployments {
        Table,
        Id,
        Name,
        ArtifactRef,
        ReplicasDesired,
        Framework,
        Mode,
        Device,
        AcceleratorRuntime,
        ArchitectureFamily,
        MemoryRequirementBytes,
        ConcurrencyRequirement,
        PendingReplicas,
        AssignedReplicas,
        PullingReplicas,
        StartingReplicas,
        ReadyReplicas,
        FailedReplicas,
        StoppedReplicas,
        CreatedAt,
        UpdatedAt,
    }

    #[derive(DeriveIden)]
    enum Replicas {
        Table,
        Id,
        DeploymentId,
        State,
        AssignedWorkerId,
        LeaseId,
        StatusMessage,
        CreatedAt,
        UpdatedAt,
    }

    #[derive(DeriveIden)]
    enum Workers {
        Table,
        Id,
        DisplayName,
        State,
        AssignedReplicas,
        AvailableSlots,
        LastHeartbeatAt,
        CreatedAt,
        UpdatedAt,
    }

    #[derive(DeriveIden)]
    enum WorkerCapabilities {
        Table,
        Id,
        WorkerId,
        Framework,
        Mode,
        Device,
        AcceleratorRuntime,
        ArchitectureFamily,
        AvailableMemoryBytes,
        ConcurrencySlots,
        CreatedAt,
    }

    #[derive(DeriveIden)]
    enum Leases {
        Table,
        Id,
        WorkerId,
        DeploymentId,
        ReplicaId,
        State,
        Framework,
        Mode,
        Device,
        AcceleratorRuntime,
        ArchitectureFamily,
        MemoryRequirementBytes,
        ConcurrencyRequirement,
        CreatedAt,
        UpdatedAt,
    }

    #[derive(DeriveIden)]
    enum WorkerAssignments {
        Table,
        Id,
        WorkerId,
        ReplicaId,
        LeaseId,
        Payload,
        DeliveryState,
        CreatedAt,
    }
}
