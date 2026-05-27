use anyhow::Result;

mod config;
mod db;
mod log;
mod safety;
mod commands;
mod sessions;
mod json;
mod summary;
mod agent;

use agent::EchoAgent;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Echo Rust Agent v2 – Starting...\n");

    let config = config::load_config("config.toml")
        .expect("Failed to load config.toml");

    let mut agent = EchoAgent::new(config).await?;
    agent.run().await?;

    println!("\nEcho session ended normally. Goodbye!");
    Ok(())
}
