use sea_orm::{Set, entity::prelude::*};

use crate::ApiError;
use dlp_shared::WorkerRecord;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub worker_id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: String,
    pub last_seen_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl TryFrom<Model> for WorkerRecord {
    type Error = ApiError;

    fn try_from(value: Model) -> Result<Self, Self::Error> {
        Ok(WorkerRecord {
            worker_id: value.worker_id,
            name: value.name,
            capabilities: value.capabilities,
            status: super::super::parse_worker_status(&value.status)?,
            last_seen_at: value.last_seen_at,
        })
    }
}

impl ActiveModel {
    pub fn from_record(value: WorkerRecord) -> Self {
        Self {
            worker_id: Set(value.worker_id),
            name: Set(value.name),
            capabilities: Set(value.capabilities),
            status: Set(super::super::worker_status_to_db(&value.status).to_string()),
            last_seen_at: Set(value.last_seen_at),
        }
    }
}
