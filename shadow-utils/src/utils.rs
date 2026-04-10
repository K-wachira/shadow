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
