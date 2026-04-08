use crate::db::Database;
use shadow_services::models::EntryLog;
use crate::llm::ChatMessage;
use crate::model::Message;
use crate::model::MessageKind;
use crate::setup::ShadowPaths;

pub fn ask(
    conn: &Database, curr_content: &[Message], paths: &ShadowPaths,
) -> color_eyre::Result<Vec<ChatMessage>> {
    let logs = conn.get_logs(Some(100)).unwrap_or_default();
    let log_context = format_logs(logs);
    let mind = std::fs::read_to_string(&paths.mind)?;

    let system = format!(
        "You are Shadow, a personal assistant with access to the user's logs.\n\n\
         You may use tools for live weather, web search, and URL fetching when current or external information is needed.\n\n\
         Current shadow.mind:\n---\n{mind}\n---\n\n\
         Recent Logs:\n---\n{log_context}\n---"
    );

    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: system.clone(),
        ..ChatMessage::default()
    }];

    // map conversation history
    for msg in curr_content {
        match &msg.kind {
            MessageKind::UserInput { text } => messages.push(ChatMessage::user(text.clone())),
            MessageKind::AssistantText { text } => {
                messages.push(ChatMessage::assistant(text.clone()))
            }
            _ => {}
        }
    }
    Ok(messages)
}

fn format_logs(logs: Vec<EntryLog>) -> String {
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::db::Database;
//     use crate::db::RawLog;

//     fn test_db() -> Database {
//         Database::init(":memory:").expect("in-memory db should open")
//     }

//     #[test]
//     fn ask_with_empty_db_returns_prompt_containing_query() {
//         let db = test_db();
//         let result = ask(&"What should I do today?".to_string(), &db, &vec![]).unwrap();
//         assert!(result.contains("What should I do today?"));
//         assert!(result.contains("Shadow"));
//     }

//     #[test]
//     fn ask_includes_user_messages_in_history() {
//         let db = test_db();
//         let messages = vec![
//             Message::user("How are you?"),
//             Message::agent("I'm doing well!"),
//         ];
//         let result = ask(&"Follow up".to_string(), &db, &messages).unwrap();
//         assert!(result.contains("How are you?"));
//         assert!(result.contains("I'm doing well!"));
//     }

//     #[test]
//     fn ask_with_logs_in_db_includes_context() {
//         let db = test_db();
//         let log = RawLog {
//             content: "Went for a long run".to_string(),
//             energy: Some(9),
//             mood: Some(8),
//             weather: Some("Clear".to_string()),
//             location: Some("Park".to_string()),
//             time_stamp: "2024-03-01T08:00:00Z".to_string(),
//             device: Some("iPhone".to_string()),
//             log_type: None,
//         };
//         db.insert_log(&log).unwrap();
//         let result = ask(&"Any fitness insights?".to_string(), &db, &vec![]).unwrap();
//         assert!(result.contains("Went for a long run"));
//     }

//     #[test]
//     fn ask_ignores_logo_and_tool_messages_in_history() {
//         let db = test_db();
//         let messages = vec![
//             Message::logo("Model Name"),
//             Message::user("Hello"),
//             Message::thought("thinking..."),
//         ];
//         let result = ask(&"test".to_string(), &db, &messages).unwrap();
//         // Logo and thoughts are excluded from history; only user messages included
//         assert!(result.contains("Hello"));
//     }

//     #[test]
//     fn ask_prompt_contains_system_role_text() {
//         let db = test_db();
//         let result = ask(&"ping".to_string(), &db, &vec![]).unwrap();
//         assert!(result.contains("personal assistant"));
//     }
// }
