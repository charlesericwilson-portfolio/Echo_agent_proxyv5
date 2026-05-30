use serde_json::Value;
use anyhow::Result;
use chrono::Local;
use crate::log::save_chat_log_entry;
use scraper::{Html, Selector};

pub async fn handle_json_tool_call_str(tool_call: &str, _web_search_url: Option<&str>, enabled_tools: &[String],) -> Result<String> {
    let parsed: Value = serde_json::from_str(tool_call)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON tool call: {}", e))?;

    // Support the format the model is actually outputting
    let function = if parsed["tool_calls"].is_array() && parsed["tool_calls"][0]["function"].is_object() {
        &parsed["tool_calls"][0]["function"]
    } else if parsed["function"].is_object() {
        &parsed["function"]
    } else {
        &parsed
    };

    let tool_name = function["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tool name found in JSON"))?;

    // Check if tool is enabled in config
    if !enabled_tools.contains(&tool_name.to_string()) {
        return Err(anyhow::anyhow!("Tool '{}' is not enabled in config", tool_name));
    }

    let arguments: Value = if function["arguments"].is_string() {
        let args_str = function["arguments"].as_str().unwrap();
        serde_json::from_str(args_str).unwrap_or(Value::Object(serde_json::Map::new()))
    } else if function["arguments"].is_object() {
        function["arguments"].clone()
    } else {
        Value::Object(serde_json::Map::new())
    };
    // These are place holders for examples of how to define your json tools the logic works.
    match tool_name {
        "get_current_datetime" => {
            let now = Local::now();
            Ok(format!("Current datetime: {}", now.format("%Y-%m-%d %H:%M:%S %Z")))
        }

        "web_search" => {
    let query = arguments["query"]
        .as_str()
        .unwrap_or("No query provided");

    match web_search(query).await {
        Ok(results) => Ok(format!("Web search results for '{}':\n\n{}", query, results)),
        Err(e) => Ok(format!("Web search failed: {}", e)),
    }
}

    "browse_page" => {
    let url = arguments["url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument for browse_page"))?;

    let max_chars = arguments["max_chars"].as_u64().map(|v| v as usize);

    match browse_page(url, max_chars).await {
        Ok(content) => Ok(format!(
            "Content from {}:\n\n{}",
            url, content
        )),
        Err(e) => Ok(format!("Failed to browse page: {}", e)),
    }
}

        _ => Err(anyhow::anyhow!("Unknown JSON tool: {}", tool_name)),
    }
}

pub async fn web_search(query: &str) -> Result<String, anyhow::Error> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; EchoAgent/1.0)")
        .send()
        .await?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let result_selector = Selector::parse(".result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();

    for (i, element) in document.select(&result_selector).take(5).enumerate() {
        let title = element.text().collect::<String>();
        let link = element.value().attr("href").unwrap_or("").to_string();

        // Try to get snippet
        let snippet = document
            .select(&snippet_selector)
            .nth(i)
            .map(|s| s.text().collect::<String>())
            .unwrap_or_default();

        results.push(format!(
            "{}. {}\n   {}\n   {}",
            i + 1,
            title.trim(),
            link,
            snippet.trim()
        ));
    }

    if results.is_empty() {
        Ok("No search results found.".to_string())
    } else {
        Ok(results.join("\n\n"))
    }
}

pub async fn browse_page(url: &str, max_chars: Option<usize>) -> Result<String, anyhow::Error> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; EchoAgent/1.0)")
        .build()?;

    let response = client.get(url).send().await?;
    let html = response.text().await?;

    let document = Html::parse_document(&html);

    // Try to get the main content
    let body_selector = Selector::parse("body").unwrap();
    let text_content = document
        .select(&body_selector)
        .next()
        .map(|body| {
            body.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "Could not extract page content.".to_string());

    let max = max_chars.unwrap_or(8000); // default limit
    let truncated = if text_content.len() > max {
        format!("{}...\n\n[Content truncated. Page was very long.]", &text_content[..max])
    } else {
        text_content
    };

    Ok(truncated)
}

pub async fn handle_json_tool(
    agent: &mut crate::agent::EchoAgent,
    user_input: &str,
    _current_response: &str,
    json_content: &str,
) -> Result<()> {
    println!("{}Echo: Detected JSON tool call{}", crate::agent::YELLOW, crate::agent::RESET_COLOR);

    // Pull the web search URL from config so the field is actually used
    let web_search_url = agent.config.web_search.as_ref().map(|w| w.url.as_str());
    let enabled_tools = &agent.config.json_tools.enabled;

    match handle_json_tool_call_str(json_content, web_search_url, enabled_tools).await {
        Ok(result) => {
            let tool_content = format!("Tool output:\n{}", result);
            save_chat_log_entry(&agent.home_dir, user_input, &tool_content, "assistant").await?;
            agent.messages.push(serde_json::json!({"role": "tool", "content": tool_content}));
        }
        Err(e) => {
            let error_msg = format!("JSON Tool error: {}", e);
            agent.messages.push(serde_json::json!({"role": "tool", "content": error_msg}));
        }
    }

    Ok(())
}

/// Extracts JSON tool call content from <json>...</json> tags
pub fn extract_json_tool(response: &str) -> Option<String> {
    if let Some(start) = response.find("<json>") {
        if let Some(end) = response[start..].find("</json>") {
            let inner = &response[start + 6..start + end];
            return Some(inner.trim().to_string());
        }
    }
    None
}
