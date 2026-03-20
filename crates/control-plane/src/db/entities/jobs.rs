use sea_orm::{Set, entity::prelude::*};

use crate::ApiError;
use dlp_shared::JobRecord;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub job_id: String,
    pub job_kind: String,
    pub status: String,
    pub required_capabilities: Vec<String>,
    pub payload: Json,
    pub assigned_worker: Option<String>,
    pub result: Option<Json>,
    pub error: Option<String>,
    pub created_at: DateTimeUtc,
    pub started_at: Option<DateTimeUtc>,
    pub finished_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl TryFrom<Model> for JobRecord {
    type Error = ApiError;

    fn try_from(value: Model) -> Result<Self, Self::Error> {
        Ok(JobRecord {
            job_id: value.job_id,
            job_kind: super::super::parse_job_kind(&value.job_kind)?,
            status: super::super::parse_job_status(&value.status)?,
            required_capabilities: value.required_capabilities,
            payload: value.payload,
            assigned_worker: value.assigned_worker,
            result: value.result,
            error: value.error,
            created_at: value.created_at,
            started_at: value.started_at,
            finished_at: value.finished_at,
        })
    }
}

impl ActiveModel {
    pub fn from_record(value: JobRecord) -> Self {
        Self {
            job_id: Set(value.job_id),
            job_kind: Set(super::super::job_kind_to_db(&value.job_kind).to_string()),
            status: Set(super::super::job_status_to_db(&value.status).to_string()),
            required_capabilities: Set(value.required_capabilities),
            payload: Set(value.payload),
            assigned_worker: Set(value.assigned_worker),
            result: Set(value.result),
            error: Set(value.error),
            created_at: Set(value.created_at),
            started_at: Set(value.started_at),
            finished_at: Set(value.finished_at),
        }
    }
}
