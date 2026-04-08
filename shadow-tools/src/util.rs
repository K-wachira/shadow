use color_eyre::eyre::eyre;
use serde_json::Value;

pub fn required_string(args: &Value, key: &str) -> color_eyre::Result<String> {
    let object = coerce_argument_object(args);

    if let Some(value) = lookup_string_arg(&object, key) {
        return Ok(value);
    }

    if let Some(fuzzy_key) = find_close_key(&object, key) {
        if let Some(value) = lookup_string_arg(&object, fuzzy_key) {
            return Ok(value);
        }
    }

    Err(eyre!("missing required string argument `{key}`"))
}

pub fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn strip_html_tags(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut inside_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut truncated = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}

fn coerce_argument_object(args: &Value) -> Value {
    match args {
        Value::String(raw) => serde_json::from_str::<Value>(raw).unwrap_or_else(|_| args.clone()),
        _ => args.clone(),
    }
}

fn lookup_string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn find_close_key<'a>(args: &'a Value, expected: &str) -> Option<&'a str> {
    let object = args.as_object()?;
    object
        .keys()
        .filter_map(|candidate| {
            let distance = bounded_edit_distance(candidate, expected, 2)?;
            (distance <= 1).then_some((distance, candidate.as_str()))
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, key)| key)
}

fn bounded_edit_distance(left: &str, right: &str, max_distance: usize) -> Option<usize> {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();

    if left.len().abs_diff(right.len()) > max_distance {
        return None;
    }

    let mut prev: Vec<usize> = (0..=right.len()).collect();
    let mut curr = vec![0; right.len() + 1];

    for (i, left_ch) in left.iter().enumerate() {
        curr[0] = i + 1;
        let mut min_in_row = curr[0];

        for (j, right_ch) in right.iter().enumerate() {
            let cost = usize::from(left_ch != right_ch);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            min_in_row = min_in_row.min(curr[j + 1]);
        }

        if min_in_row > max_distance {
            return None;
        }

        std::mem::swap(&mut prev, &mut curr);
    }

    let distance = prev[right.len()];
    (distance <= max_distance).then_some(distance)
}
