use amp_common::schema::Source;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreatePlaybookRequest {
    pub title: String,
    pub description: String,
    pub preface: Source,
}
