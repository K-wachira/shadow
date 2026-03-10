use crate::ask;
use crate::db::Database;
use crate::ingest::file_ingest;
use crate::ollama::LlmClient;

use tokio_stream::StreamExt;
use tracing::error;
use tracing::info;

pub async fn handle_ask(db_conn: &Database, ollama_conn: LlmClient, query: Option<String>) {
    match query {
        Some(text) => {
            info!("Question: {}", text);
            match ask(&text, &db_conn) {
                Ok(prompt) => {
                    match ollama_conn.ollama_ask_stream(&prompt).await {
                        Ok(mut stream) => {
                            while let Some(chunk) = stream.next().await {
                                match chunk {
                                    Ok(res) => {
                                        for resp in res {
                                            // res is Result<Vec<GenerationResponse>, ...>
                                            print!("{}", termimad::inline(&resp.response));
                                        }
                                    }
                                    Err(err) => error!("{err}"),
                                }
                            }
                        }
                        Err(err) => error!("{err}"),
                    }
                }
                Err(err) => error!("{err}"),
            }
        }
        None => {
            info!("No Question");
        }
    };
}

pub fn handle_ingests(db_conn: &Database) {
    info!("\n \n Ingesting files:");
    let dir = dirs::home_dir()
        .unwrap()
        .join("Library/Mobile Documents/com~apple~CloudDocs/ShadowLogs/");
    let _ = file_ingest(&db_conn, &dir);
}

pub async fn handle_recent(content: Option<i32>, conn: &Database) {
    info!("\n \nRecent  logs:");
    match conn.get_logs(content) {
        Ok(logs) => {
            for log in logs {
                info!("Log: {:?}", log);
            }
        }
        // Handle the error
        Err(_) => {}
    }
}

pub async fn handle_log(content: Option<String>) {
    match content {
        Some(new_log) => {
            info!("New Log {}", new_log);
        }
        None => {
            info!("No Log");
        }
    };
}

pub async fn handle_stats() {
    info!("Stats")
}
