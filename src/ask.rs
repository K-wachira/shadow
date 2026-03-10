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

fn build_prompt(context: &str, query: &str) -> String {
    format!(
        "You are Shadow, a personal assistant with access to the user's logs. 
         Context:\n{}\n\n\
         Question: {}",
        context, query
    )
}

fn format_context(logs: Vec<EntryLog>) -> String {
    logs.iter()
        .map(|log| {
            format!(
                "[{}] {}\nMood: {:?} | Energy: {:?} | Weather: {:?} | Location: {:?}",
                log.time_stamp, log.content, log.mood, log.energy, log.weather, log.location
            )
        })
        .collect::<Vec<String>>()
        .join("\n\n")
}