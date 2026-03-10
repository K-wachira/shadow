use crate::db::{Database};
use crate::models::EntryLog;

pub fn ask(query: &String, conn: &Database) -> Result<String, String>{
    let log_limit = Some(100);
    let results: String = match conn.get_logs(log_limit)  {
        Ok(context) => build_prompt(&format_context(context), &query),
        Err(err) => err.to_string()
    };

    Ok(results)
}

