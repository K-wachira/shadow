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
pub struct DbLog {
    pub id: i32,
    pub content: String,
    pub energy: i32, 
    pub mood: i32,
    pub weather: String,
}
