use serde::{Deserialize, Serialize};

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

#[derive(Debug)]
pub struct EntryLog {
    pub id: i32,
    pub content: String,
    pub energy: Option<i32>, 
    pub mood: Option<i32>,
    pub weather: Option<String>,
}

#[derive(Debug)]
pub struct FileIngest {
    pub id: Option<i32>,
    pub file_name: String,
    pub time_stamp: String,
    pub is_ingested: Option<bool
}
