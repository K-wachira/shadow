use chrono::Utc;

// ─── Utility ─────────────────────────────────────────────────────────────────
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

pub fn format_timestamp(ts: &str) -> String {
    // try rfc3339 first (2026-03-29T10:00:43+03:00)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        return dt.format("%Y-%m-%d - %H:%M").to_string();
    }
    // try parsing as milliseconds integer
    if let Ok(ms) = ts.parse::<i64>() {
        if let Some(dt) = chrono::DateTime::from_timestamp_millis(ms) {
            return dt.format("%Y-%m-%d - %H:%M").to_string();
        }
    }
    // fallback — return as-is
    ts.to_string()
}

pub fn format_duration(secs: &u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    match (hours, minutes, seconds) {
        (0, 0, s) => format!("{}s", s),
        (0, m, s) => format!("{}m {}s", m, s),
        (h, m, s) => format!("{}h {}m {}s", h, m, s),
    }
}

pub fn today() -> String {
    format_timestamp(Utc::now().timestamp_millis().to_string().as_ref())
}

pub fn model_name_format(model_name: String) -> String {
    model_name
        .split('-')
        .next()
        .unwrap_or(&model_name)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_returns_same_string_when_shorter_than_max() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_returns_same_string_when_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_truncates_and_adds_ellipsis_when_longer() {
        assert_eq!(truncate("hello world", 5), "hello…");
    }

    #[test]
    fn truncate_handles_multibyte_characters() {
        let s = "你好世界";
        assert_eq!(truncate(s, 2), "你好…");
    }

    #[test]
    fn truncate_with_zero_max_returns_ellipsis() {
        assert_eq!(truncate("hello", 0), "…");
    }

    #[test]
    fn format_timestamp_parses_rfc3339() {
        let result = format_timestamp("2026-03-29T10:00:43+03:00");
        assert!(result.contains("2026-03-29"));
        assert!(result.contains("10:00"));
    }

    #[test]
    fn format_timestamp_parses_millis() {
        // Derive epoch-millis from a known UTC instant so the constant can't drift.
        let ms = chrono::DateTime::parse_from_rfc3339("2026-03-29T10:00:00Z")
            .unwrap()
            .timestamp_millis();
        let result = format_timestamp(&ms.to_string());
        assert!(result.contains("2026-03-29"));
        assert!(result.contains("10:00"));
    }

    #[test]
    fn format_timestamp_falls_back_to_input() {
        assert_eq!(format_timestamp("not-a-date"), "not-a-date");
    }

    #[test]
    fn format_timestamp_handles_invalid_millis() {
        // Too large to be a valid timestamp
        assert_eq!(format_timestamp("999999999999999999"), "999999999999999999");
    }

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(&45), "45s");
    }

    #[test]
    fn format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(&125), "2m 5s");
    }

    #[test]
    fn format_duration_hours_minutes_seconds() {
        assert_eq!(format_duration(&3661), "1h 1m 1s");
    }

    #[test]
    fn format_duration_exact_hour() {
        assert_eq!(format_duration(&3600), "1h 0m 0s");
    }

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(&0), "0s");
    }

    #[test]
    fn model_name_format_splits_on_dash() {
        assert_eq!(
            model_name_format("deepseek-r1-latest".to_string()),
            "deepseek"
        );
    }

    #[test]
    fn model_name_format_no_dash_returns_original() {
        assert_eq!(model_name_format("gpt4".to_string()), "gpt4");
    }

    #[test]
    fn model_name_format_empty_string() {
        assert_eq!(model_name_format("".to_string()), "");
    }
}
