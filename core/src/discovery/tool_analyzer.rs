use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

pub struct ToolAnalyzer;

impl ToolAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Auto-discovers a tool by running it with `--help`, grabbing the output,
    /// and generating a prompt for the Zero-Node to parse it into a `cat_registry.yaml` format.
    pub async fn generate_tool_definition_prompt(
        &self,
        tool_name: &str,
        nix_pkg: &str,
    ) -> Result<String> {
        info!(
            "Running auto-discovery on tool '{}' via nix pkg '{}'",
            tool_name, nix_pkg
        );

        // Run the tool with --help inside nix shell
        let mut cmd = Command::new("nix");
        cmd.arg("shell")
            .arg(nix_pkg)
            .arg("--command")
            .arg(tool_name)
            .arg("--help");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context(format!("Failed to execute {} --help", tool_name))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let help_text = if !stdout.trim().is_empty() {
            stdout
        } else {
            stderr
        };

        if help_text.trim().is_empty() {
            warn!("Tool {} did not produce any output for --help", tool_name);
            return Err(anyhow::anyhow!(
                "No help text produced by tool {}",
                tool_name
            ));
        }

        // Generate the strict prompt for the LLM
        let prompt = format!(
            r#"Ты — системный анализатор (Eva OS Tool Discovery).
Твоя задача прочитать мануал (--help) для утилиты `{}` и сгенерировать конфигурацию для `cat_registry.yaml`.
Тебе разрешено выделить только самые важные и безопасные флаги (только для чтения данных).

Мануал утилиты:
{}

Сгенерируй ТОЛЬКО YAML в следующем формате, без markdown-кода:
{}:
  description: "Краткое описание инструмента"
  allowed_flags:
    "-флаг1": "Описание флага 1"
    "--флаг2": "Описание флага 2"
"#,
            tool_name, help_text, tool_name
        );

        Ok(prompt)
    }
}
