use crate::config::LLMConfig;
use anyhow::{Result, anyhow};
use tokio::process::Command;
use std::process::Stdio;
use serde_json::Value;

/// Strip ANSI/control characters and leading/trailing Markdown code fences (e.g., ``` or ```diff) from the patch text
// Remove ANSI escape sequences and control characters (except newline and tab).
// Also strip leading/trailing Markdown code fences (```).
fn strip_markdown_fences(s: &str) -> String {
    let mut filtered = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        // Remove ANSI escape sequences and other control characters (except newline and tab)
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    // Skip '['
                    chars.next();
                    // Skip until final byte (ASCII '@' to '~')
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&nc) {
                            break;
                        }
                    }
                } else {
                    // Skip next char of unknown escape sequence
                    chars.next();
                }
            }
            // Skip the escape character itself
            continue;
        } else if c.is_control() && c != '\n' && c != '\t' {
            continue;
        }
        filtered.push(c);
    }
    let lines: Vec<&str> = filtered.lines().collect();
    let mut start = 0;
    // Skip leading fence lines
    while start < lines.len() && lines[start].trim_start().starts_with("```") {
        start += 1;
    }
    // Skip trailing fence lines
    let mut end = lines.len();
    while end > start && lines[end - 1].trim_start().starts_with("```") {
        end -= 1;
    }
    let mut result = lines[start..end].join("\n");
    // Ensure trailing newline
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

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
        let prompt = "Summarise this project. Search all files, find every entry point, feature, and important detail, then return a comprehensive summary.";
        let mut cmd = Command::new("codex");
        cmd.arg("-q")
            .arg("--provider").arg(&self.config.provider)
            .arg("--no-project-doc")
            .arg(prompt)
            .current_dir(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = cmd.output().await?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "codex CLI returned non-zero exit status: {}; stderr: {}",
                output.status,
                err.trim()
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
        // Prompt should instruct the model to include full diff headers and output only raw unified diff
        let prompt = "Based on the current code in this directory, generate a minimal patch in unified diff format to improve code quality. Include full unified diff headers (diff --git, --- a/<path>, +++ b/<path>) before each hunk. Output only the raw unified diff without markdown fences or any extra explanation.";
        let mut cmd = Command::new("codex");
        cmd.arg("-q")
            .arg("--provider").arg(&self.config.provider)
            .arg(prompt)
            .current_dir(context)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = cmd.output().await?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "codex CLI returned non-zero exit status: {}; stderr: {}",
                output.status,
                err.trim()
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
        // Combine collected output or fallback to raw
        let result = if !collected.is_empty() {
            collected.join("")
        } else {
            raw
        };
        // Trim and remove any Markdown fences
        let trimmed = result.trim();
        let cleaned = strip_markdown_fences(trimmed);
        Ok(cleaned)
    }
    /// Generate a concise one-sentence commit message for the given unified diff using the LLM
    pub async fn generate_commit_message(&self, project: &str, diff: &str) -> Result<String> {
        tracing::info!(project = project, "Codex generating commit message via CLI");
        let prompt_base = "Given the following unified diff, write a concise one-sentence commit message summarizing the change. Do not include the diff, only output the commit message without quotes.";
        let prompt = format!("{}\n\n{}", prompt_base, diff);
        let mut cmd = Command::new("codex");
        cmd.arg("-q")
            .arg("--provider").arg(&self.config.provider)
            .arg(prompt)
            .current_dir(project)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = cmd.output().await?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "codex CLI returned non-zero exit status: {}; stderr: {}",
                output.status,
                err.trim()
            ));
        }
        let raw = String::from_utf8(output.stdout)?;
        // Extract assistant messages from JSON lines
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
        let result = if !collected.is_empty() {
            collected.join("")
        } else {
            raw
        };
        // Clean up any Markdown fences or control characters
        let cleaned = strip_markdown_fences(result.trim());
        Ok(cleaned)
    }
}
