use crate::protocol::ToolDefinition;
use crate::util::required_string;
use chromiumoxide::browser::Browser;
use color_eyre::eyre::WrapErr;
use color_eyre::eyre::eyre;
use futures::StreamExt;
use reqwest::Url;
use scraper::Html;
use scraper::Selector;
use serde_json::Value;

pub fn tool() -> ToolDefinition {
    ToolDefinition::new(
        "search_web",
        "Retrieve live, real-time information from the internet to answer queries about current events, news, weather, stock prices, or any factual data occurring after January 2025 (your knowledge cutoff).

        Use this tool if: the user asks for 'latest', 'current', 'today', or 'now'.
        This tool:
        - is used to provide up-to-date information for current events and recent data
        - Returns search result information formatted as search result blocks, including links as markdown hyperlinks

        - Searches are performed automatically within a single API call

        Enriched semantic triggers: Use this to determine qualitative states that change over time, such as 'is it hot/cold', 'who is winning', or 'what is the price of'.
        Do not use this tool for: general reasoning, mathematical proofs, creative writing, or explaining historical facts established before 2025.
        If the internal knowledge is sufficient and definitive, prioritize it over a search.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The exact search query to run."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
        |args| async move { search_web(args).await },
    )
}

async fn search_web(args: Value) -> color_eyre::Result<String> {
    let query = required_string(&args, "query")?;
    let (browser, mut handler) = Browser::connect("ws://127.0.0.1:9222")
        .await
        .wrap_err("failed to connect to Lightpanda CDP at ws://127.0.0.1:9222")?;
    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(err) = event {
                eprintln!("lightpanda handler error: {err}");
                break;
            }
        }
    });

    let result = async {
        let search_url = Url::parse_with_params(
            "https://html.duckduckgo.com/html/",
            &[("q", query.as_str())],
        )
        .wrap_err("failed to build DuckDuckGo search URL")?;
        let page = browser
            .new_page(search_url.as_str())
            .await
            .wrap_err_with(|| format!("failed to open search URL in Lightpanda: {search_url}"))?;
        let html = page
            .content()
            .await
            .wrap_err("failed to read page HTML from Lightpanda")?;
        let _ = page.close().await;
        extract_duckduckgo_results(&query, &html)
    }
    .await;

    handler_task.abort();
    result
}

fn extract_duckduckgo_results(query: &str, html: &str) -> color_eyre::Result<String> {
    let document = Html::parse_document(html);
    let result_selector = selector(".result")?;
    let title_selector = selector(".result__title a.result__a")?;
    let snippet_selector = selector(".result__snippet")?;
    let url_selector = selector(".result__url")?;

    let mut lines = vec![format!("Search query: {query}")];
    let mut found = 0usize;

    for result in document.select(&result_selector) {
        let Some(title_node) = result.select(&title_selector).next() else {
            continue;
        };

        let title = collect_text(title_node.text());
        if title.is_empty() {
            continue;
        }

        let href = title_node.value().attr("href").unwrap_or_default();
        let resolved_url = normalize_duckduckgo_href(href);
        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|node| collect_text(node.text()))
            .filter(|text| !text.is_empty())
            .or_else(|| {
                result
                    .select(&url_selector)
                    .next()
                    .map(|node| collect_text(node.text()))
                    .filter(|text| !text.is_empty())
            })
            .unwrap_or_else(|| "No snippet available.".to_string());

        if found == 0 {
            lines.push("Results:".into());
        }

        lines.push(format!("- {title}"));
        lines.push(format!("  {resolved_url}"));
        lines.push(format!("  {snippet}"));
        found += 1;

        if found >= 5 {
            break;
        }
    }

    if found == 0 {
        return Ok(format!(
            "Search query: {query}\nNo search results were extracted from the page."
        ));
    }

    Ok(lines.join("\n"))
}

fn selector(css: &str) -> color_eyre::Result<Selector> {
    Selector::parse(css).map_err(|err| eyre!("invalid selector `{css}`: {err}"))
}

fn collect_text<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    parts
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_duckduckgo_href(href: &str) -> String {
    if href.is_empty() {
        return "No URL available.".into();
    }

    let parsed = match Url::parse(href)
        .or_else(|_| Url::parse(&format!("https://html.duckduckgo.com{href}")))
    {
        Ok(url) => url,
        Err(_) => return href.to_string(),
    };

    parsed
        .query_pairs()
        .find(|(key, _)| key == "uddg")
        .map(|(_, value)| value.into_owned())
        .unwrap_or_else(|| parsed.to_string())
}
