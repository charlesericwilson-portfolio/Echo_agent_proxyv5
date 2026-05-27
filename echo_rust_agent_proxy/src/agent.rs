use std::path::PathBuf;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{Value, json};
use anyhow::Result;
use std::collections::HashMap;
use dirs_next as dirs;
use std::sync::atomic::Ordering;

use crate::config::Config;
use crate::db::ToolDatabase;
use crate::summary::summarize_context;
use crate::sessions::{extract_session_command, extract_end_command, clean_up_sessions};
use crate::log::save_chat_log_entry;
use crate::commands::extract_command;
use crate::json::extract_json_tool;

pub const LIGHT_BLUE: &str = "\x1b[94m";
pub const YELLOW: &str = "\x1b[33m";
pub const RESET_COLOR: &str = "\x1b[0m";

pub struct EchoAgent {
    pub config: Config,
    pub messages: Vec<Value>,
    pub db: ToolDatabase,
    pub home_dir: PathBuf,
    pub active_sessions: Arc<Mutex<HashMap<String, (String, String)>>>,
    pub stop_generation: Arc<std::sync::atomic::AtomicBool>,
}

impl EchoAgent {
    pub async fn new(config: Config) -> Result<Self> {
        let home_dir = match &config.paths.home_dir {
            Some(path) if !path.trim().is_empty() => PathBuf::from(path),
            _ => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/user/Documents")),
        };

        let context_path = if config.paths.context_file.starts_with('/') {
            PathBuf::from(&config.paths.context_file)
        } else {
            home_dir.join(&config.paths.context_file)
        };

        let db_path = if config.paths.database.starts_with('/') {
            PathBuf::from(&config.paths.database)
        } else {
            home_dir.join(&config.paths.database)
        };

        let db = ToolDatabase::new(db_path)?;

        let mut messages = vec![];

        let mut context_content = String::new();
        if tokio::fs::metadata(&context_path).await.is_ok() {
            context_content = tokio::fs::read_to_string(&context_path).await.unwrap_or_default();
            println!("✅ Loaded context file: {}", context_path.display());
        } else {
            println!("⚠️ Context file not found at: {}", context_path.display());
        }

        let main_prompt = tokio::fs::read_to_string(&config.prompts.main_system)
            .await
            .expect("Failed to read main system prompt");

        let full_system_prompt = format!("{}\n\n{}", main_prompt.trim(), context_content.trim());

        messages.push(json!({"role": "system", "content": full_system_prompt}));

        Ok(Self {
            config,
            messages,
            db,
            home_dir,
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
            stop_generation: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("Echo: Ready. Type 'quit' or 'exit' to end session.\n");

        let mut quit = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::quit())
            .expect("Failed to set up SIGQUIT handler");

        let stop_flag = self.stop_generation.clone();

        tokio::spawn(async move {
            while quit.recv().await.is_some() {
                stop_flag.store(true, Ordering::SeqCst);
                println!("\n[Generation interrupted by Ctrl+\\]");
            }
        });

        loop {
            print!("You: ");
            std::io::stdout().flush()?;

            let mut user_input = String::new();
            std::io::stdin().read_line(&mut user_input)?;
            let trimmed_input = user_input.trim();

            if trimmed_input.eq_ignore_ascii_case("quit") || trimmed_input.eq_ignore_ascii_case("exit") {
                println!("Session ended.");
                save_chat_log_entry(&self.home_dir, "", "", "SESSION_END").await?;
                break;
            }

            self.messages.push(json!({"role": "user", "content": trimmed_input}));

            let final_response = self.process_turn(trimmed_input).await?;

            println!("{}Echo:\n{}\n{}", LIGHT_BLUE, final_response.trim(), RESET_COLOR);
        }

        clean_up_sessions(&self.active_sessions).await?;
        Ok(())
    }
        #[allow(unused_assignments)]
        async fn process_turn(&mut self, user_input: &str) -> Result<String> {
            let mut current_response = String::new();

            loop {
                let payload = json!({
                    "model": self.config.endpoint.model,
                    "messages": &self.messages,
                    "temperature": self.config.endpoint.temperature,
                    "max_tokens": self.config.endpoint.max_tokens
                });

                if self.stop_generation.load(Ordering::SeqCst) {
                    self.stop_generation.store(false, Ordering::SeqCst);
                    return Ok("[Generation stopped by user]".to_string());
                }

                let response_text = reqwest::Client::new()
                    .post(&self.config.endpoint.url)
                    .json(&payload)
                    .send()
                    .await?
                    .json::<Value>()
                    .await?["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string();

                current_response = response_text.clone();

                // Log + store in history
                save_chat_log_entry(&self.home_dir, user_input, &current_response, "assistant").await?;
                self.messages.push(json!({"role": "assistant", "content": &current_response}));

                // Tool handling - NO printing in handlers
                if let Some(json_content) = extract_json_tool(&current_response) {
                    crate::json::handle_json_tool(self, user_input, &current_response, &json_content).await?;
                    continue;
                }
                else if let Some((session_name, command)) = extract_session_command(&current_response) {
                    crate::sessions::handle_session_command(self, user_input, &session_name, Some(&command)).await?;
                    continue;
                }
                else if let Some(session_name) = extract_end_command(&current_response) {
                    crate::sessions::handle_session_command(self, user_input, &session_name, None).await?;
                    continue;
                }
                else if let Some(command) = extract_command(&current_response) {
                    crate::commands::handle_command(self, user_input, &command).await?;
                    continue;
                }
                else {
                    // FINAL ANSWER
                    let total_chars: usize = self.messages.iter()
                        .map(|m| m["content"].as_str().unwrap_or("").len())
                        .sum();

                    if total_chars > self.config.context.summarize_threshold {
                        summarize_context(&mut self.messages, &self.config).await?;
                    }
                    return Ok(current_response);
                }
            }
        }
}
