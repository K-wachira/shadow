use shadow_services::models::EntryLog;
use crate::mind::mind_model::Belief;
use shadow_utils::utils;
use crate::mind::mind_model::ShadowMind;

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