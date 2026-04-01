use crate::db::{EntryLog, FileIngest, RawLog};
use crate::ingest::process_json_file;

use crate::db::SessionMessages;
use crate::db::Sessions;
use chrono::prelude::*;
use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use tracing::error;

pub struct Database {
    conn: Connection,
}

impl Database {
    // Initialize a new DB
    pub fn new(path: &str) -> color_eyre::Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        db.initialize_logs()?;
        db.initialize_ingested_files()?;
        db.initialize_sessions()?;
        db.initialize_session_messages()?;
        Ok(db)
    }

    // Create logs table if it does not exist
    fn initialize_logs(&self) -> color_eyre::Result<()> {
        self.conn.execute_batch(
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
        );
        ",
        )?;
        Ok(())
    }

    fn initialize_ingested_files(&self) -> color_eyre::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS ingested_files (
            id    INTEGER PRIMARY KEY,
            file_name  TEXT NOT NULL,
            time_stamp TEXT NOT NULL,
            ingested TEXT
        );
        ",
        )?;
        Ok(())
    }

    fn initialize_sessions(&self) -> color_eyre::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
            id    INTEGER PRIMARY KEY,
            user_id  INTEGER NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at_ms INTEGER NOT NULL, 
            updated_at_ms INTEGER NOT NULL, 
            started_at_ms INTEGER, 
            ended_at_ms INTEGER, 
            provider TEXT, 
            model TEXT, 
            system_prompt TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_user_created_at
          ON sessions(user_id, created_at_ms);
        ",
        )?;
        Ok(())
    }

    fn initialize_session_messages(&self) -> color_eyre::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS session_messages (
              id INTEGER PRIMARY KEY,
              session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
              seq INTEGER NOT NULL,
              created_at_ms INTEGER NOT NULL,
              role TEXT NOT NULL,
              content TEXT NOT NULL,
              model TEXT,
              status TEXT NOT NULL DEFAULT 'active',
              system_prompt TEXT,
              metadata_json TEXT NOT NULL DEFAULT '{}',
              UNIQUE(session_id, seq)
            );

            CREATE INDEX IF NOT EXISTS idx_session_messages_session_seq
              ON session_messages(session_id, seq);
            ",
        )?;
        Ok(())
    }

    // Inserts a single log
    pub fn insert_log(&self, log: &RawLog) -> color_eyre::Result<EntryLog> {
        self.conn.execute(
            "INSERT INTO shadow_logs (content, energy, mood, weather, location, time_stamp, device, log_type)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &log.content,
                &log.energy,
                &log.mood,
                &log.weather,
                &log.location,
                &log.time_stamp,
                &log.device,
                &log.log_type
            ],
        )?;
        let id = self.conn.last_insert_rowid() as i32;
        Ok(EntryLog {
            id,
            content: log.content.clone(),
            energy: log.energy,
            mood: log.mood,
            weather: log.weather.clone(),
            location: log.location.clone(),
            time_stamp: log.time_stamp.clone(),
            device: log.device.clone(),
            log_type: log.log_type.clone(),
        })
    }

    // Fetches all logs
    pub fn get_logs(&self, limit: Option<i32>) -> color_eyre::Result<Vec<EntryLog>> {
        let query = match limit {
            Some(n) => format!(
                "SELECT id, content, energy, mood, weather, location, time_stamp, device, log_type FROM shadow_logs ORDER BY time_stamp DESC LIMIT {}", n ),
            None => "SELECT id, content, energy, mood, weather, location, time_stamp, device, log_type FROM shadow_logs ORDER BY time_stamp DESC".to_string(),
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
                    location: row.get::<_, Option<String>>(5)?,
                    time_stamp: row.get::<_, String>(6)?,
                    device: row.get::<_, Option<String>>(7)?,
                    log_type: row.get::<_, Option<String>>(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(logs)
    }

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
        &self, log_name: &String, dir: &PathBuf,
    ) -> color_eyre::Result<Option<EntryLog>> {
        if self.file_logged(&log_name)? {
            return Ok(None);
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

        match process_json_file(log_name, dir) {
            Ok(raw_log) => Ok(Some(self.insert_log(&raw_log)?)),
            Err(e) => {
                error!("Failed to process {log_name}: {e}");
                Ok(None)
            }
        }
    }

    pub fn get_file_ingests(
        &self, limit: Option<usize>,
    ) -> Result<Vec<FileIngest>, rusqlite::Error> {
        let query = match limit {
            Some(n) => {
                format!(
                    "SELECT id, file_name, time_stamp, ingested FROM ingested_files ORDER BY time_stamp DESC LIMIT {}",
                    n
                )
            }
            None => "SELECT id, file_name, time_stamp, ingested FROM ingested_files ORDER BY time_stamp DESC"
                .to_string(),
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

    pub fn create_session(&self, title: &str, model: &str) -> color_eyre::Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "INSERT INTO sessions (user_id, title, status, created_at_ms, updated_at_ms, model, metadata_json)
             VALUES (?1, ?2, 'active', ?3, ?4, ?5, '{}')",
            rusqlite::params![1, title, now, now, model],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_session_title(&self, session_id: i64, title: &str) -> color_eyre::Result<()> {
        self.conn.execute(
            "UPDATE sessions SET title = ?1 WHERE id = ?2",
            rusqlite::params![title, session_id],
        )?;
        Ok(())
    }

    pub fn end_session(&self, session_id: i64) -> color_eyre::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE sessions SET status = 'ended', ended_at_ms = ?1, updated_at_ms = ?2 WHERE id = ?3",
            rusqlite::params![now, now, session_id],
        )?;
        Ok(())
    }

    pub fn get_session(&self, session_id: i64) -> color_eyre::Result<Sessions> {
        let session = self.conn.query_row(
            "SELECT id, user_id, title, status, created_at_ms, updated_at_ms, 
                    started_at_ms, ended_at_ms, provider, model, system_prompt, metadata_json
             FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
            |row| {
                Ok(Sessions {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    created_at_ms: row.get(4)?,
                    updated_at_ms: row.get(5)?,
                    started_at_ms: row.get(6)?,
                    ended_at_ms: row.get(7)?,
                    provider: row.get(8)?,
                    model: row.get(9)?,
                    system_prompt: row.get(10)?,
                    metadata_json: row.get(11)?,
                })
            },
        )?;
        Ok(session)
    }

    pub fn get_recent_sessions(&self, limit: usize) -> color_eyre::Result<Vec<Sessions>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, title, status, created_at_ms, updated_at_ms,
                    started_at_ms, ended_at_ms, provider, model, system_prompt, metadata_json
             FROM sessions ORDER BY created_at_ms DESC LIMIT ?1",
        )?;
        let sessions = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(Sessions {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    created_at_ms: row.get(4)?,
                    updated_at_ms: row.get(5)?,
                    started_at_ms: row.get(6)?,
                    ended_at_ms: row.get(7)?,
                    provider: row.get(8)?,
                    model: row.get(9)?,
                    system_prompt: row.get(10)?,
                    metadata_json: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    pub fn insert_message(
        &self, session_id: i64, role: &str, content: &str, model: Option<&str>,
    ) -> color_eyre::Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();

        // get next seq number for this session
        let seq: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM session_messages WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO session_messages (session_id, seq, created_at_ms, role, content, model, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}')",
            rusqlite::params![session_id, seq, now, role, content, model],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_session_messages(
        &self, session_id: i64,
    ) -> color_eyre::Result<Vec<SessionMessages>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, seq, created_at_ms, role, content, status, model, system_prompt, metadata_json
             FROM session_messages WHERE session_id = ?1 ORDER BY seq ASC",
        )?;
        let messages = stmt
            .query_map(rusqlite::params![session_id], |row| {
                Ok(SessionMessages {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    seq: row.get(2)?,
                    created_at_ms: row.get(3)?,
                    role: row.get(4)?,
                    content: row.get(5)?,
                    status: row.get(6)?,
                    model: row.get(7)?,
                    system_prompt: row.get(8)?,
                    metadata_json: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    pub fn delete_session(&self, session_id: i64) -> color_eyre::Result<()> {
        self.conn.execute(
            "DELETE FROM session_messages WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawLog;

    fn test_db() -> Database {
        Database::new(":memory:").expect("in-memory db should open")
    }

    fn sample_log(content: &str) -> RawLog {
        RawLog {
            content: content.to_string(),
            energy: Some(7),
            mood: Some(8),
            weather: Some("Sunny".to_string()),
            location: Some("Home".to_string()),
            time_stamp: "2024-01-01T10:00:00Z".to_string(),
            device: Some("iPhone".to_string()),
            log_type: Some("journal".to_string()),
        }
    }

    // ── Initialization ────────────────────────────────────────────────────────

    #[test]
    fn new_opens_in_memory_db_and_creates_tables() {
        let db = test_db();
        assert!(db.get_logs(None).is_ok());
        assert!(db.get_recent_sessions(10).is_ok());
        assert!(db.get_file_ingests(None).is_ok());
    }

    // ── Logs ─────────────────────────────────────────────────────────────────

    #[test]
    fn insert_and_get_log_round_trip() {
        let db = test_db();
        db.insert_log(&sample_log("Felt great today")).unwrap();
        let logs = db.get_logs(None).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].content, "Felt great today");
        assert_eq!(logs[0].energy, Some(7));
        assert_eq!(logs[0].mood, Some(8));
        assert_eq!(logs[0].weather, Some("Sunny".to_string()));
    }

    #[test]
    fn get_logs_empty_db_returns_empty_vec() {
        let db = test_db();
        assert!(db.get_logs(None).unwrap().is_empty());
    }

    #[test]
    fn get_logs_respects_limit() {
        let db = test_db();
        for i in 0..5 {
            let mut log = sample_log(&format!("Log {}", i));
            log.time_stamp = format!("2024-01-0{}T10:00:00Z", i + 1);
            db.insert_log(&log).unwrap();
        }
        assert_eq!(db.get_logs(Some(3)).unwrap().len(), 3);
    }

    #[test]
    fn get_logs_with_none_limit_returns_all() {
        let db = test_db();
        for i in 0..4 {
            db.insert_log(&sample_log(&format!("Log {}", i))).unwrap();
        }
        assert_eq!(db.get_logs(None).unwrap().len(), 4);
    }

    #[test]
    fn insert_log_with_null_optional_fields() {
        let db = test_db();
        let log = RawLog {
            content: "minimal entry".to_string(),
            energy: None,
            mood: None,
            weather: None,
            location: None,
            time_stamp: "2024-06-01T00:00:00Z".to_string(),
            device: Some("unknown".to_string()),
            log_type: None,
        };
        db.insert_log(&log).unwrap();
        let logs = db.get_logs(None).unwrap();
        assert_eq!(logs[0].energy, None);
        assert_eq!(logs[0].mood, None);
    }

    // ── Sessions ─────────────────────────────────────────────────────────────

    #[test]
    fn create_and_get_session_round_trip() {
        let db = test_db();
        let id = db.create_session("My Session", "llama3").unwrap();
        let session = db.get_session(id).unwrap();
        assert_eq!(session.id, id);
        assert_eq!(session.title, "My Session");
        assert_eq!(session.status, "active");
        assert_eq!(session.model, Some("llama3".to_string()));
        assert_eq!(session.user_id, 1);
    }

    #[test]
    fn update_session_title_persists() {
        let db = test_db();
        let id = db.create_session("Old Title", "llama3").unwrap();
        db.update_session_title(id, "New Title").unwrap();
        assert_eq!(db.get_session(id).unwrap().title, "New Title");
    }

    #[test]
    fn end_session_sets_status_to_ended_and_records_timestamp() {
        let db = test_db();
        let id = db.create_session("Test", "model").unwrap();
        db.end_session(id).unwrap();
        let session = db.get_session(id).unwrap();
        assert_eq!(session.status, "ended");
        assert!(session.ended_at_ms.is_some());
    }

    #[test]
    fn get_recent_sessions_returns_most_recent_first() {
        let db = test_db();
        let id1 = db.create_session("First", "model").unwrap();
        let id2 = db.create_session("Second", "model").unwrap();
        let sessions = db.get_recent_sessions(10).unwrap();
        assert_eq!(sessions.len(), 2);
        // Both sessions must be present (order may vary if timestamps collide)
        let ids: Vec<i64> = sessions.iter().map(|s| s.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn get_recent_sessions_respects_limit() {
        let db = test_db();
        for i in 0..5 {
            db.create_session(&format!("Session {}", i), "model")
                .unwrap();
        }
        assert_eq!(db.get_recent_sessions(3).unwrap().len(), 3);
    }

    #[test]
    fn delete_session_removes_session() {
        let db = test_db();
        let id = db.create_session("ToDelete", "model").unwrap();
        db.delete_session(id).unwrap();
        assert!(db.get_session(id).is_err());
    }

    #[test]
    fn delete_session_cascades_to_messages() {
        let db = test_db();
        let id = db.create_session("ToDelete", "model").unwrap();
        db.insert_message(id, "user", "Hello", None).unwrap();
        db.insert_message(id, "assistant", "Hi", None).unwrap();
        db.delete_session(id).unwrap();
        assert!(db.get_session_messages(id).unwrap().is_empty());
    }

    // ── Session messages ─────────────────────────────────────────────────────

    #[test]
    fn insert_and_get_messages_round_trip() {
        let db = test_db();
        let sid = db.create_session("Test", "model").unwrap();
        db.insert_message(sid, "user", "Hello", None).unwrap();
        db.insert_message(sid, "assistant", "Hi there", Some("llama3"))
            .unwrap();
        let msgs = db.get_session_messages(sid).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "Hello");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "Hi there");
        assert_eq!(msgs[1].model, Some("llama3".to_string()));
    }

    #[test]
    fn messages_seq_auto_increments_from_one() {
        let db = test_db();
        let sid = db.create_session("Test", "model").unwrap();
        for _ in 0..4 {
            db.insert_message(sid, "user", "msg", None).unwrap();
        }
        let seqs: Vec<i32> = db
            .get_session_messages(sid)
            .unwrap()
            .iter()
            .map(|m| m.seq)
            .collect();
        assert_eq!(seqs, vec![1, 2, 3, 4]);
    }

    #[test]
    fn messages_ordered_by_seq_asc() {
        let db = test_db();
        let sid = db.create_session("Test", "model").unwrap();
        db.insert_message(sid, "user", "First", None).unwrap();
        db.insert_message(sid, "assistant", "Second", None).unwrap();
        db.insert_message(sid, "user", "Third", None).unwrap();
        let msgs = db.get_session_messages(sid).unwrap();
        assert_eq!(msgs[0].content, "First");
        assert_eq!(msgs[1].content, "Second");
        assert_eq!(msgs[2].content, "Third");
    }

    #[test]
    fn messages_isolated_between_sessions() {
        let db = test_db();
        let sid1 = db.create_session("S1", "model").unwrap();
        let sid2 = db.create_session("S2", "model").unwrap();
        db.insert_message(sid1, "user", "S1 msg", None).unwrap();
        db.insert_message(sid2, "user", "S2 msg", None).unwrap();
        assert_eq!(db.get_session_messages(sid1).unwrap().len(), 1);
        assert_eq!(db.get_session_messages(sid2).unwrap().len(), 1);
        assert_eq!(db.get_session_messages(sid1).unwrap()[0].content, "S1 msg");
    }

    #[test]
    fn get_session_messages_empty_for_new_session() {
        let db = test_db();
        let sid = db.create_session("Empty", "model").unwrap();
        assert!(db.get_session_messages(sid).unwrap().is_empty());
    }
}
