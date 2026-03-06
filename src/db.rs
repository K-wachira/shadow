use crate::models::{DbLog, EntryLog};

use rusqlite::{Connection, Result};
use std::process;

pub struct Database {
    conn: Connection,
}

impl Database {
    // Initialize a new DB
    pub fn new(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).unwrap();
        let db = Database { conn: conn };

        db.initialize_logs()?;
        db.initialize_ingested_files()?;
        Ok(db)
    }

    // Create logs table if it does not exist
    fn initialize_logs(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS shadow_logs (
            id    INTEGER PRIMARY KEY,
            content  TEXT NOT NULL,
            energy INTEGER,
            mood INTEGER,
            weather TEXT,
            location TEXT,
            time_stamp TEXT NOT NULL,
            device TEXT NOT NULL,
            log_type TEXT
        )",
            )
            .unwrap_or_else(|_| {
                process::exit(1);
            });
        Ok(())
    }

    fn initialize_ingested_files(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS ingested_files (
            id    INTEGER PRIMARY KEY,
            file_name  TEXT NOT NULL,
            time_stamp TEXT NOT NULL,
            ingested TEXT
        )",
            )
            .unwrap_or_else(|_| {
                process::exit(1);
            });
        Ok(())
    }

    // Inserts a single log
    pub fn insert(&self, log: &RawLog) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO shadow_logs (content, energy, mood, weather, location, time_stamp, device, log_type)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![&log.content, &log.energy, &log.mood, &log.weather, &log.location, &log.time_stamp, &log.device, &log.log_type],
        )?;

        Ok(())
    }

    //Fetches all logs
    pub fn get_logs(&self, limit: Option<usize>) -> Result<Vec<DbLog>, rusqlite::Error> {
        let query = match limit {
            Some(n) => format!(
                "SELECT id, content, energy, mood, weather FROM shadow_logs LIMIT {}", n ),
            None => "SELECT id, content, energy, mood, weather FROM shadow_logs".to_string(),
        };

        let mut stmt = self.conn.prepare(&query)?;

        let logs: Vec<DbLog> = stmt
            .query_map([], |row| {
                Ok(DbLog {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    energy: row.get(2)?,
                    mood: row.get(3)?,
                    weather: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(logs);
    }
}
