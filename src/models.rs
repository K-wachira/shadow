//From disk (Icloud)
pub struct EntryLog {
    pub content: String,
    pub energy: i32,
    pub mood: i32,
    pub weather: String,
    pub location: String,
    pub time_stamp: String,
    pub device: String,
    pub log_type: String,
}

#[derive(Debug)]
pub struct DbLog {
    pub id: i32,
    pub content: String,
    pub energy: i32, 
    pub mood: i32,
    pub weather: String,
}
