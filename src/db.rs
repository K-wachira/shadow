use crate::ingest::process_json_file;
use crate::models::{EntryLog, FileIngest, RawLog};

use std::path::PathBuf;
use chrono::prelude::*;
use rusqlite::{Connection, Result, params};
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

    // Fetches all logs
    pub fn get_logs(&self, limit: Option<usize>) -> Result<Vec<EntryLog>, rusqlite::Error> {
        let query = match limit {
            Some(n) => format!(
                "SELECT id, content, energy, mood, weather FROM shadow_logs ORDER BY time_stamp DESC LIMIT {}", n ),
            None => "SELECT id, content, energy, mood, weather FROM shadow_logs ORDER BY time_stamp DESC".to_string(),
        };
        let mut stmt = self.conn.prepare(&query)?;

        let logs: Vec<EntryLog> = stmt
            .query_map([], |row| {
                Ok(EntryLog {
                    id: row.get::<_, i32>(0)?,
                    content: row.get::<_, String>(1)?,
                    energy: row.get::<_, Option<i32>>(2)?,
                    mood: row.get::<_, Option<i32>>(3)?,
                    weather: row.get::<_, Option<String>>(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(logs)
    }

    // get_range

    // get_recent

    // file_logged ? returns if a file has been previously logged
    fn file_logged(&self, log_name: &String) -> Result<bool, rusqlite::Error> {
        let exists = self
            .conn
            .query_row(
                "SELECT id FROM ingested_files WHERE file_name = ?1",
                [&log_name],
                |row| row.get::<_, i64>(0),
            )
            .is_ok();
        Ok(exists)
    }

    pub fn insert_file_ingest(
        &self,
        log_name: &String,
        dir: &PathBuf,
    ) -> Result<(), rusqlite::Error> {
        if self.file_logged(&log_name)? {
            return Ok(());
        }

        let file_ing = FileIngest {
            id: None,
            file_name: log_name.to_string(),
            time_stamp: Local::now().to_string(),
            is_ingested: Some(true),
        };
        self.conn.execute(
            "INSERT INTO ingested_files (file_name, time_stamp, ingested)
                VALUES (?1, ?2, ?3)",
            params![
                file_ing.file_name,
                file_ing.time_stamp,
                file_ing.is_ingested
            ],
        )?;

        match self.insert(&process_json_file(&log_name, &dir).unwrap()) {
            Ok(_) => {}
            Err(err) => {
                println!("Some Error {err} when inserting from file")
            }
        }
        Ok(())
    }

    pub fn get_file_ingests(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<FileIngest>, rusqlite::Error> {
        let query = match limit {
            Some(n) => format!(
                "SELECT id, file_name, time_stamp, ingested FROM ingested_files ORDER BY time_stamp DESC LIMIT {}", n ),
            None => "SELECT id, file_name, time_stamp, ingested FROM ingested_files ORDER BY time_stamp DESC".to_string(),
        };
        let mut stmt = self.conn.prepare(&query)?;

        let file_ingests: Vec<FileIngest> = stmt
            .query_map([], |row| {
                Ok(FileIngest {
                    id: row.get::<_, Option<i32>>(0)?,
                    file_name: row.get::<_, String>(1)?,
                    time_stamp: row.get::<_, String>(2)?,
                    is_ingested: None,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(file_ingests)
    }
}
