use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SynchronizationRequest {
    pub kind: String,
    pub paths: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub payload: Vec<u8>,
}
