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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_execute_echo() {
        let runtime = McpRuntime::new();
        let res = runtime.execute_command("echo", &["test_mcp"]).await.unwrap();
        assert_eq!(res.trim(), "test_mcp");
    }
    
    #[tokio::test]
    async fn test_mcp_execute_failure() {
        let runtime = McpRuntime::new();
        // Trying to run a command that exits with code 1
        let res = runtime.execute_command("false", &[]).await;
        assert!(res.is_err());
    }
}
