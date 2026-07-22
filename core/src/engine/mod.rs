use tokio::process::Command;
use anyhow::Context;

pub struct McpRuntime;

impl McpRuntime {
    pub fn new() -> Self {
        Self
    }

    /// Executes a system command securely.
    /// In a production environment, this should have strict whitelist checks.
    pub async fn execute_command(&self, command: &str, args: &[&str]) -> anyhow::Result<String> {
        tracing::info!("MCP Executor running command: {} {:?}", command, args);
        
        let output = Command::new(command)
            .args(args)
            .output()
            .await
            .with_context(|| format!("Failed to execute command: {}", command))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(anyhow::anyhow!("Command failed: {}", stderr))
        }
    }
}
