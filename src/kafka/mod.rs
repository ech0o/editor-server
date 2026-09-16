pub mod consumer;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::model::RunStatus;

#[derive(Debug,Serialize,Deserialize)]
pub struct JobEvent{
    pub job_id: Uuid,
    pub status: RunStatus
}