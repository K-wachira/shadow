use crate::db::Database;
use crate::model::Message;
use crate::model::MessageKind;
use crate::db::EntryLog;

pub fn ask(query: &String, conn: &Database, curr_content: &Vec<Message>) -> color_eyre::Result<String>{
    let log_limit = Some(100);
    let results: String = match conn.get_logs(log_limit)  {
        Ok(context) => build_prompt(&format_context(context), &query, build_current_history(curr_content)?),
        Err(err) => err.to_string()
    };

    Ok(results)
}

fn build_current_history(curr_messages: &Vec<Message>) -> color_eyre::Result<String> {
    let mut history_blob = String::new();
    for message in curr_messages {
          match &message.kind {
              MessageKind::UserInput { text }  => {
                  history_blob.push_str(&format!("User: {}", &text));
              },
              MessageKind::AssistantText { text }  => {
                  history_blob.push_str(&format!("Shadow: {}", &text));
              },
              _ => {continue;}
          }
    }
    Ok(history_blob)
}

fn build_prompt(context: &str, query: &str, history_blob: String ) -> String {
    format!(
        "
        You are Shadow, a personal assistant with access to the user's logs. 
        Current Chat History:\n {} \n\n\
         Context:\n{}\n\n\
         Question: {}",
        history_blob, context, query
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