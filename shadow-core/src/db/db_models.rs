use serde::{Deserialize, Serialize};

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

//From disk (Icloud)
#[derive(Serialize, Deserialize)]
pub struct RawLog {
    pub content: String,
    pub energy: Option<i32>,
    pub mood: Option<i32>,
    pub weather: Option<String>,
    pub location: Option<String>,
    pub time_stamp: String,
    pub device: Option<String>,
    #[serde(rename = "type")]
    pub log_type: Option<String>,
}

#[derive(serde::Serialize)]
#[derive(Debug)]
pub struct EntryLog {
    pub id: i32,
    pub content: String,
    pub energy: Option<i32>,
    pub mood: Option<i32>,
    pub weather: Option<String>,
    pub location: Option<String>,
    pub time_stamp: String,
    pub device: String,
    pub log_type: Option<String>,
}

#[derive(Debug)]
pub struct FileIngest {
    pub id: Option<i32>,
    pub file_name: String,
    pub time_stamp: String,
    pub is_ingested: Option<bool>,
}
