use crate::registry::RegistryManager;
use anyhow::{Context, Result};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn};

pub struct CatExecutor {
    registry: Arc<RegistryManager>,
}

impl CatExecutor {
    pub fn new(registry: Arc<RegistryManager>) -> Self {
        Self { registry }
    }

    /// Validates and executes a CAT tool via nix shell.
    /// Returns the combined stdout/stderr as a String to inject into the RAG context.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        flags: Vec<String>,
        args: Vec<String>,
    ) -> Result<String> {
        let tool_def = self.registry.cat.tools.get(tool_name).ok_or_else(|| {
            anyhow::anyhow!("Tool '{}' is not defined in CAT registry.", tool_name)
        })?;

        // Validate flags against allowed_flags in the registry
        for flag in &flags {
            if !tool_def.allowed_flags.contains_key(flag) {
                warn!(
                    "Blocked unauthorized flag '{}' for tool '{}'",
                    flag, tool_name
                );
                return Err(anyhow::anyhow!("Unauthorized flag: {}", flag));
            }
        }

        // We wrap every command in `nix shell` to guarantee reproducibility and access.
        // We map the tool to its corresponding nixpkgs package.
        let nix_pkg = match tool_name {
            "ls" | "cat" | "df" | "free" => "nixpkgs#coreutils",
            "grep" => "nixpkgs#gnugrep",
            "dmesg" => "nixpkgs#util-linux",
            "journalctl" => "nixpkgs#systemd",
            _ => "nixpkgs#coreutils",
        };

        info!("Executing CAT tool: {} with flags: {:?}", tool_name, flags);

        // Build the command: nix shell <nix_pkg> --command <tool_name> <flags> <args>
        let mut cmd = Command::new("nix");
        cmd.arg("shell")
            .arg(nix_pkg)
            .arg("--command")
            .arg(tool_name);

        for flag in flags {
            cmd.arg(flag);
        }
        for arg in args {
            cmd.arg(arg);
        }

        // Execute command
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context(format!("Failed to execute {}", tool_name))?;

        let mut result = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // If there is error output but the command technically succeeded or returned useful info, include it.
        if !stderr.is_empty() {
            result.push_str("\n--- STDERR ---\n");
            result.push_str(&stderr);
        }

        if !output.status.success() {
            warn!(
                "Tool {} exited with non-zero status: {}",
                tool_name, output.status
            );
            // We still return the output, because LLMs need to see the error messages to fix them.
        }

        Ok(result)
    }
}
