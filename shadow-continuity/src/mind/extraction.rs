use crate::mind::mind_model::Belief;
use crate::mind::mind_model::ShadowMind;
use shadow_services::models::EntryLog;
use shadow_utils::utils;

pub fn collect_field_paths(mind: &ShadowMind) -> Vec<String> {
    let mut paths = Vec::new();
    for key in mind.surface.keys() {
        paths.push(format!("surface.{}", key));
    }
    for key in mind.behavioural.keys() {
        paths.push(format!("behavioural.{}", key));
    }
    for key in mind.mental_model.keys() {
        paths.push(format!("mental_model.{}", key));
    }
    for key in mind.values.keys() {
        paths.push(format!("values.{}", key));
    }
    paths
}

pub fn build_extraction_prompt(log_str: String, fields: &[String]) -> String {
    let fields_list = fields.join("\n- ");
    format!(
        r#"You are the extraction stage of a personal model update pipeline.

Your only job is to identify which fields in shadow.mind are touched by this log entry.
You are not updating anything. You are not summarizing. You are only identifying.

## Log entry

{log_str}

## Known mind fields

- {fields_list}

## Task

Return a JSON array of field paths that this log contains meaningful evidence for.
Only include fields where the log could change or reinforce the current belief.

Rules:
- Use dot notation exactly as shown above
- Do not include fields the log has no evidence for
- Do not explain your choices
- If the log touches nothing meaningful, return an empty array

## Output

Raw JSON array only. No markdown, no explanation, no preamble."#
    )
}

pub fn build_update_prompt(field_path: &str, belief: &Belief, log: &EntryLog) -> String {
    let belief_str = serde_json::to_string_pretty(belief).unwrap_or_default();
    let log_str = serde_json::to_string_pretty(log).unwrap_or_default();
    let today = utils::today();

    format!(
        r#"You are the update stage of a personal model update pipeline.

You will be given a single belief from shadow.mind and a new log entry that is relevant to it.
Your job is to return an updated version of the belief.

## Field

{field_path}

## Current belief

{belief_str}

## New log entry

{log_str}

## Task

Return an updated version of this belief reflecting what the new log adds or changes.

Rules:
- If the log strengthens the current belief, raise confidence slightly and append the log id to source_logs
- If the log changes the belief, update value and recalibrate confidence
- If the log contradicts the belief, narrow the wording and lower confidence
- If the log adds nothing new, return the belief unchanged
- Always append an operation to the operations array explaining what changed and why
- Set last_updated to {today}
- Never remove existing operations
- op must be "update" unless this is the first operation, in which case "create"
- confidence must stay within 0.0 to 1.0
- Do not exceed 0.95 from inference alone

## Output

Raw JSON only. No markdown, no explanation, no preamble.
Return the complete updated Belief object."#
    )
}
pub fn parse_field_array(response: &str) -> color_eyre::Result<Vec<String>> {
    let trimmed = response.trim();
    // strip markdown fences if the model ignored instructions
    let clean = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    Ok(serde_json::from_str(clean)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::mind_model::ShadowMind;

    fn make_test_mind() -> ShadowMind {
        let mut mind = ShadowMind::default();
        mind.surface.insert("interests".into(), Belief {
            value: "loves reading".into(),
            confidence: 0.8,
            source_logs: vec![],
            last_updated: "2026-01-01".into(),
            operations: vec![],
        });
        mind.behavioural.insert("habits".into(), Belief {
            value: "morning jog".into(),
            confidence: 0.6,
            source_logs: vec![],
            last_updated: "2026-01-01".into(),
            operations: vec![],
        });
        mind.mental_model.insert("self_view".into(), Belief {
            value: "capable".into(),
            confidence: 0.9,
            source_logs: vec![],
            last_updated: "2026-01-01".into(),
            operations: vec![],
        });
        mind.values.insert("core".into(), Belief {
            value: "honesty".into(),
            confidence: 0.95,
            source_logs: vec![],
            last_updated: "2026-01-01".into(),
            operations: vec![],
        });
        mind
    }

    #[test]
    fn collect_field_paths_returns_all_field_paths() {
        let mind = make_test_mind();
        let paths = collect_field_paths(&mind);
        assert_eq!(paths.len(), 4);
        assert!(paths.contains(&"surface.interests".to_string()));
        assert!(paths.contains(&"behavioural.habits".to_string()));
        assert!(paths.contains(&"mental_model.self_view".to_string()));
        assert!(paths.contains(&"values.core".to_string()));
    }

    #[test]
    fn collect_field_paths_empty_for_empty_mind() {
        let mind = ShadowMind::default();
        let paths = collect_field_paths(&mind);
        assert!(paths.is_empty());
    }

    #[test]
    fn parse_field_array_parses_simple_array() {
        let response = r#"["surface.interests", "values.core"]"#;
        let result = parse_field_array(response).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "surface.interests");
        assert_eq!(result[1], "values.core");
    }

    #[test]
    fn parse_field_array_parses_empty_array() {
        let response = "[]";
        let result = parse_field_array(response).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_field_array_strips_json_fences() {
        let response = "```json\n[\"surface.interests\"]\n```";
        let result = parse_field_array(response).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "surface.interests");
    }

    #[test]
    fn parse_field_array_strips_plain_code_fences() {
        let response = "```\n[\"surface.interests\"]\n```";
        let result = parse_field_array(response).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_field_array_handles_whitespace() {
        let response = "  \n  [\"surface.interests\"]  \n  ";
        let result = parse_field_array(response).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_field_array_returns_error_for_invalid_json() {
        let response = "not json at all";
        assert!(parse_field_array(response).is_err());
    }

    #[test]
    fn parse_field_array_returns_error_for_non_array() {
        let response = "\"just a string\"";
        assert!(parse_field_array(response).is_err());
    }

    #[test]
    fn build_extraction_prompt_contains_log_and_fields() {
        let log_str = "I went for a run today".to_string();
        let fields = vec!["surface.interests".to_string(), "behavioural.habits".to_string()];
        let prompt = build_extraction_prompt(log_str, &fields);
        assert!(prompt.contains("I went for a run today"));
        assert!(prompt.contains("surface.interests"));
        assert!(prompt.contains("behavioural.habits"));
    }

    #[test]
    fn build_extraction_prompt_has_required_sections() {
        let prompt = build_extraction_prompt("test".into(), &vec!["field1".into()]);
        assert!(prompt.contains("Log entry"));
        assert!(prompt.contains("Known mind fields"));
        assert!(prompt.contains("Task"));
        assert!(prompt.contains("Output"));
    }

    #[test]
    fn build_update_prompt_contains_all_sections() {
        let belief = Belief {
            value: "test belief".into(),
            confidence: 0.8,
            source_logs: vec!["log1".into()],
            last_updated: "2026-01-01".into(),
            operations: vec![],
        };
        let log = EntryLog {
            id: 1,
            content: "new log".into(),
            energy: Some(8),
            mood: Some(7),
            weather: Some("sunny".into()),
            location: Some("home".into()),
            time_stamp: "2026-01-01".into(),
            device: None,
            log_type: None,
        };
        let prompt = build_update_prompt("surface.test", &belief, &log);
        assert!(prompt.contains("surface.test"));
        assert!(prompt.contains("test belief"));
        assert!(prompt.contains("new log"));
    }
}
