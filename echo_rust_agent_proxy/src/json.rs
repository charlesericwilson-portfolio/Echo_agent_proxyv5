use serde_json::Value;
use chrono::Local;

/// Accepts a string containing a JSON tool call and handles it
pub async fn handle_json_tool_call_str(tool_call_str: &str) -> Result<String, String> {
    // Parse the string into a JSON Value first
    let tool_call: Value = match serde_json::from_str(tool_call_str) {
        Ok(v) => v,
        Err(e) => return Err(format!("Failed to parse JSON: {}", e)),
    };

    // Now process the parsed JSON
    handle_json_tool_call(&tool_call).await
}

/// Internal function that works with already-parsed JSON
async fn handle_json_tool_call(tool_call: &Value) -> Result<String, String> {
    let tool_calls = &tool_call["tool_calls"];

    if let Some(first_call) = tool_calls.get(0) {
        // Correct path: look inside the first tool call object
        let function = &first_call["function"];
        let name = function["name"].as_str().unwrap_or("");
        let _arguments = function["arguments"].as_str().unwrap_or("{}");

        match name {
            "get_current_datetime" => get_current_datetime().await,
            _ => Err(format!("Unknown JSON tool: {}", name)),
        }
    } else {
        Err("No tool_calls found in JSON".to_string())
    }
}
/// Simple JSON tool example
async fn get_current_datetime() -> Result<String, String> {
    let now = Local::now();
    let formatted = now.format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(serde_json::json!({
        "result": formatted,
        "status": "success"
    }).to_string())
}
