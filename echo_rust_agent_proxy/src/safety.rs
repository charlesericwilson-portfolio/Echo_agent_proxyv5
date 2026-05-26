// safety.rs
use anyhow::Result;
use crate::CONFIG;

pub fn is_command_safe(command: &str) -> Result<()> {
    let cmd_lower = command.to_lowercase();

    for keyword in &CONFIG.security.denylist {
        if cmd_lower.contains(&*keyword) {
            return Err(anyhow::anyhow!(
                "🚫 Command blocked by safety filter: contains dangerous keyword '{}'\n\
                 This command was blocked to prevent potential system damage.",
                keyword
            ));
        }
    }

    // Additional simple checks
    if cmd_lower.contains("sudo") && cmd_lower.contains("rm") {
        return Err(anyhow::anyhow!(
            "🚫 Command blocked: sudo rm is not allowed for safety reasons."
        ));
    }

    Ok(())
}
