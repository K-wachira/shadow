use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Sessions {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub status: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub metadata_json: String,
}

#[derive(Serialize, Deserialize)]
pub struct SessionMessages {
    pub id: i64,
    pub session_id: i64,
    pub seq: i32,
    pub created_at_ms: i64,
    pub role: String,
    pub content: String,
    pub status: String,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub metadata_json: String,
}