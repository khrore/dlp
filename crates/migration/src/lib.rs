mod m20260320_000001_create_jobs_and_workers;

pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20260320_000001_create_jobs_and_workers::Migration,
        )]
    }
}
