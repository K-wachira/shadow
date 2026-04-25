use crate::db::Database;
use crate::llm::ChatMessage;
use crate::model::Message;
use crate::model::MessageKind;
use crate::setup::ShadowPaths;
use shadow_services::models::EntryLog;

pub fn ask(
    conn: &Database, curr_content: &[Message], paths: &ShadowPaths,
) -> color_eyre::Result<Vec<ChatMessage>> {
    let logs = conn.get_logs(Some(100)).unwrap_or_default();
    let log_context = format_logs(logs);
    let mind = std::fs::read_to_string(&paths.mind)?;

    let system = format!(
        "You are Shadow, a personal assistant with access to the user's logs.\n\n\
        - Match the user's tone and density
        - Skip affirmations and filler
        - Push back when evidence warrants it
        - Use structure only when it genuinely helps
        - Treat the user as the expert on their own life

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
