use crate::db::Database;
use crate::db::EntryLog;
use crate::model::Message;
use crate::model::MessageKind;

pub fn ask(
    query: &String, conn: &Database, curr_content: &Vec<Message>,
) -> color_eyre::Result<String> {
    let log_limit = Some(100);
    let results: String = match conn.get_logs(log_limit) {
        Ok(context) => build_prompt(
            &format_context(context),
            &query,
            build_current_history(curr_content)?,
        ),
        Err(err) => err.to_string(),
    };

    Ok(results)
}

fn build_current_history(curr_messages: &Vec<Message>) -> color_eyre::Result<String> {
    let mut history_blob = String::new();
    for message in curr_messages {
        match &message.kind {
            MessageKind::UserInput { text } => {
                history_blob.push_str(&format!("User: {}", &text));
            }
            MessageKind::AssistantText { text } => {
                history_blob.push_str(&format!("Shadow: {}", &text));
            }
            _ => {
                continue;
            }
        }
    }
    Ok(history_blob)
}

fn build_prompt(context: &str, query: &str, history_blob: String) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::RawLog;

    fn test_db() -> Database {
        Database::new(":memory:").expect("in-memory db should open")
    }

    #[test]
    fn ask_with_empty_db_returns_prompt_containing_query() {
        let db = test_db();
        let result = ask(&"What should I do today?".to_string(), &db, &vec![]).unwrap();
        assert!(result.contains("What should I do today?"));
        assert!(result.contains("Shadow"));
    }

    #[test]
    fn ask_includes_user_messages_in_history() {
        let db = test_db();
        let messages = vec![
            Message::user("How are you?"),
            Message::agent("I'm doing well!"),
        ];
        let result = ask(&"Follow up".to_string(), &db, &messages).unwrap();
        assert!(result.contains("How are you?"));
        assert!(result.contains("I'm doing well!"));
    }

    #[test]
    fn ask_with_logs_in_db_includes_context() {
        let db = test_db();
        let log = RawLog {
            content: "Went for a long run".to_string(),
            energy: Some(9),
            mood: Some(8),
            weather: Some("Clear".to_string()),
            location: Some("Park".to_string()),
            time_stamp: "2024-03-01T08:00:00Z".to_string(),
            device: Some("iPhone".to_string()),
            log_type: None,
        };
        db.insert_log(&log).unwrap();
        let result = ask(&"Any fitness insights?".to_string(), &db, &vec![]).unwrap();
        assert!(result.contains("Went for a long run"));
    }

    #[test]
    fn ask_ignores_logo_and_tool_messages_in_history() {
        let db = test_db();
        let messages = vec![
            Message::logo("Model Name"),
            Message::user("Hello"),
            Message::thought("thinking..."),
        ];
        let result = ask(&"test".to_string(), &db, &messages).unwrap();
        // Logo and thoughts are excluded from history; only user messages included
        assert!(result.contains("Hello"));
    }

    #[test]
    fn ask_prompt_contains_system_role_text() {
        let db = test_db();
        let result = ask(&"ping".to_string(), &db, &vec![]).unwrap();
        assert!(result.contains("personal assistant"));
    }
}
