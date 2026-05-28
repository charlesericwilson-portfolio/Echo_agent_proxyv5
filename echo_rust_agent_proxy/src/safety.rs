use crate::config::Config;

pub fn is_command_safe(command: &str, config: &Config) -> Result<(), String> {
    let lower_cmd = command.to_lowercase();

    for dangerous in &config.security.denylist {
        if lower_cmd.contains(&dangerous.to_lowercase()) {
            return Err(format!("Command contains dangerous keyword: {}", dangerous));
        }
    }

    // Extra safety checks
    if lower_cmd.contains("sudo rm") || lower_cmd.contains("rm -rf /") {
        return Err("Dangerous rm command blocked for safety.".to_string());
    }

    Ok(())
}
