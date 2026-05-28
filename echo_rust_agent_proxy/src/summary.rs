use serde_json::{Value, json};
use anyhow::Result;
use crate::config::Config;

pub async fn summarize_output(raw_output: &str, config: &Config) -> Result<String> {
    let tool_summarizer_prompt = tokio::fs::read_to_string(&config.prompts.summarizer)
        .await
        .expect("Failed to read summarizer prompt");

    let payload = json!({
        "model": "summarizer",
        "messages": [
            {
                "role": "system",
                "content": tool_summarizer_prompt
            },
            {
                "role": "user",
                "content": raw_output
            }
        ],
        "temperature": 0.2,
        "max_tokens": 1500
    });

    let response = reqwest::Client::new()
        .post(&config.summarizer.url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let body = response.text().await.unwrap_or_default();
    let parsed: Value = serde_json::from_str(&body)?;

    Ok(parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Summary failed.")
        .trim()
        .to_string())
}

pub async fn summarize_context(messages: &mut Vec<Value>, config: &Config) -> Result<()> {
    let summary_prompt = "Summarize the entire conversation so far in a concise way. Keep key facts, decisions, and important details. Output ONLY the summary, nothing else.";

    // Build new message list with the summary instruction
    let mut summary_messages = vec![
        json!({
            "role": "system",
            "content": summary_prompt
        })
    ];

    // Add the recent conversation history (you can limit this if needed)
    summary_messages.extend(messages.clone());

    let payload = json!({
        "model": &config.endpoint.model,
        "messages": summary_messages,
        "temperature": 0.3,           // Lower temp = more focused summary
        "max_tokens": 1024            // Keep summaries short
    });

    // Call the model
    let response = reqwest::Client::new()
        .post(&config.endpoint.url)
        .json(&payload)
        .send()
        .await?
        .json::<Value>()
        .await?;

    let summary_text = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if summary_text.is_empty() {
        return Ok(());
    }

    // Replace the old messages with: [system summary] + last few turns
    let last_turns: Vec<Value> = messages.iter().rev().take(4).cloned().collect();
    let mut new_messages = vec![
        json!({
            "role": "system",
            "content": format!("Previous conversation summary:\n{}", summary_text)
        })
    ];
    new_messages.extend(last_turns.into_iter().rev());

    *messages = new_messages;

    Ok(())
}
