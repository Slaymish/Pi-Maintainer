use crate::codex_client::CodexClient;
use crate::cache::DataCache;
use anyhow::Result;

#[derive(Clone)]
pub struct PatchGenerator {
    codex: CodexClient,
    cache: DataCache,
}

#[derive(Clone)]
pub struct PatchApplier {
    cache: DataCache,
}

impl PatchGenerator {
    pub fn new(codex: CodexClient, cache: DataCache) -> Self {
        PatchGenerator { codex, cache }
    }

    pub async fn generate(&self, context: &str) -> Result<String> {
        tracing::info!(context = context, "Generating patch");
        let patch = self.codex.generate_patch(context).await?;
        // Cache the patch suggestion for display in the web UI
        let key = format!("scheduler.patch.{}", context);
        // Insert patch into cache; patch may be large, storing full text
        if let Err(err) = self.cache.insert(&key, &patch) {
            tracing::warn!(error = %err, "Failed to cache patch suggestion for project {}", context);
        }
        Ok(patch)
    }
    /// Generate a one-sentence commit message summarizing the provided unified diff using the LLM
    pub async fn commit_message(&self, project: &str, diff: &str) -> Result<String> {
        tracing::info!(project = project, "Generating commit message");
        let msg = self.codex.generate_commit_message(project, diff).await?;
        Ok(msg)
    }
}

use std::process::{Command, Stdio};
use std::io::Write;
use anyhow::anyhow;

impl PatchApplier {
    pub fn new(cache: DataCache) -> Self {
        PatchApplier { cache }
    }

    /// Clean up the raw patch text, stripping markdown fences and any leading/trailing garbage,
    /// retaining only the unified diff content.
    fn clean_patch(raw: &str) -> String {
        let mut cleaned = Vec::new();
        let mut started = false;
        for line in raw.lines() {
            // Skip code fences; break on closing fence after diff started
            if line.starts_with("```") {
                if started {
                    break;
                } else {
                    continue;
                }
            }
            if !started {
                // Detect start of diff or hunk
                if line.starts_with("diff --git")
                    || line.starts_with("--- ")
                    || line.starts_with("+++ ")
                    || line.starts_with("@@ ")
                    || line.starts_with("index ")
                {
                    started = true;
                    cleaned.push(line);
                }
            } else {
                cleaned.push(line);
            }
        }
        if cleaned.is_empty() {
            String::new()
        } else {
            let mut s = cleaned.join("\n");
            s.push('\n');
            s
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("PatchApplier shutting down");
        Ok(())
    }

    /// Apply a unified diff patch to the given project directory using `git apply`.
    /// The patch is provided on stdin to `git apply`.
    pub fn apply(&self, project: &str, patch: &str) -> Result<()> {
        tracing::info!(project = project, "Applying patch");
        // Clean the raw patch to extract a valid unified diff
        let cleaned = Self::clean_patch(patch);
        if cleaned.trim().is_empty() {
            return Err(anyhow!("Patch contained no valid diff; skipping apply"));
        }
        // Spawn `git apply` to apply the cleaned patch, capturing output for diagnostics
        let mut child = Command::new("git")
            .arg("apply")
            .arg("--whitespace=fix")
            .arg("-")
            .current_dir(project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn `git apply`: {}", e))?;
        // Write cleaned patch to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(cleaned.as_bytes())
                .map_err(|e| anyhow!("Failed to write patch to stdin: {}", e))?;
        } else {
            return Err(anyhow!("Failed to open stdin for git apply"));
        }
        // Wait for git apply to finish and capture output
        let output = child
            .wait_with_output()
            .map_err(|e| anyhow!("Failed to wait on git apply: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "`git apply` failed with status: {}; stdout: {}; stderr: {}",
                output.status,
                stdout.trim(),
                stderr.trim()
            ))
        }
    }
}