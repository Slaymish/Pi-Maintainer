use crate::config::LLMConfig;
use anyhow::{Result, anyhow};
use tokio::process::Command;
use std::process::Stdio;
use serde_json::Value;

#[derive(Clone)]
pub struct CodexClient {
    config: LLMConfig,
}

impl CodexClient {
    pub fn new(cfg: &LLMConfig) -> Self {
        CodexClient { config: cfg.clone() }
    }

    pub async fn summarize_project(&self, path: &str) -> Result<String> {
        tracing::info!(project = path, "Codex summarizing project via CLI");
        // Run 'codex' CLI to generate project summary
        let output = Command::new("codex")
            .arg("-q")
            .arg("--no-project-doc")
            .arg("-a full-auto")
            .arg(
                "Summarise this project. Search all files, find every entry point, feature, \
                and important detail, then return a comprehensive summary.",
            )
            .current_dir(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow!(
                "codex CLI returned non-zero exit status: {}",
                output.status
            ));
        }
        let raw = String::from_utf8(output.stdout)?;
        // Extract only the assistant's final output from the JSON rollout
        // Collect extracted assistant output as owned Strings
        let mut collected = Vec::new();
        for line in raw.lines() {
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if json.get("role").and_then(Value::as_str) == Some("assistant") {
                    if let Some(contents) = json.get("content").and_then(Value::as_array) {
                        for item in contents {
                            if item.get("type").and_then(Value::as_str) == Some("output_text") {
                            if let Some(text) = item.get("text").and_then(Value::as_str) {
                                collected.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        if !collected.is_empty() {
            let result = collected.join("");
            Ok(result.trim().to_string())
        } else {
            // Fallback to raw output if parsing fails
            Ok(raw)
        }
    }

    pub async fn generate_patch(&self, context: &str) -> Result<String> {
        tracing::info!(project = context, "Codex generating patch via CLI");
        // Generate a minimal unified diff patch for this project
        let prompt = "Based on the current code in this directory, generate a minimal patch in unified diff format to improve code quality. Only output the diff.";
        let output = Command::new("codex")
            .arg("-q")
            .arg("-a full-auto")
            .arg(prompt)
            .current_dir(context)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow!(
                "codex CLI returned non-zero exit status: {}",
                output.status
            ));
        }
        let raw = String::from_utf8(output.stdout)?;
        // Extract assistant diff output from JSON lines
        let mut collected = Vec::new();
        for line in raw.lines() {
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if json.get("role").and_then(Value::as_str) == Some("assistant") {
                    if let Some(contents) = json.get("content").and_then(Value::as_array) {
                        for item in contents {
                            if item.get("type").and_then(Value::as_str) == Some("output_text") {
                                if let Some(text) = item.get("text").and_then(Value::as_str) {
                                    collected.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        if !collected.is_empty() {
            let result = collected.join("");
            Ok(result.trim().to_string())
        } else {
            // Fallback to raw output if parsing fails
            Ok(raw)
        }
    }
}
