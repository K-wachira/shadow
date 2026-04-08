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
