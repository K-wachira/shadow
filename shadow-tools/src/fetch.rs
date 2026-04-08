use crate::protocol::ToolDefinition;
use crate::util::collapse_whitespace;
use crate::util::required_string;
use crate::util::strip_html_tags;
use crate::util::truncate_chars;
use color_eyre::eyre::eyre;
use reqwest::Client;
use reqwest::Url;
use serde_json::Value;

pub fn tool() -> ToolDefinition {
    ToolDefinition::new(
        "fetch_url",
        "Fetch the contents of a public HTTP or HTTPS URL and return a readable excerpt.
        Use this after search_web when a page needs to be inspected directly.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The full HTTP or HTTPS URL to fetch."
                }
            },
            "required": ["url"],
            "additionalProperties": false
        }),
        |args| async move { fetch_url(args).await },
    )
}

async fn fetch_url(args: Value) -> color_eyre::Result<String> {
    let raw_url = required_string(&args, "url")?;
    let url = Url::parse(&raw_url)?;

    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(eyre!("unsupported URL scheme `{scheme}`")),
    }

    let text = Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let excerpt = truncate_chars(&collapse_whitespace(&strip_html_tags(&text)), 4_000);
    if excerpt.is_empty() {
        return Ok("Fetched the URL successfully, but no readable text was extracted.".into());
    }

    Ok(excerpt)
}
