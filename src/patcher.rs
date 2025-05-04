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
    codex: CodexClient,
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

use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use std::process::Stdio;
use anyhow::anyhow;

impl PatchApplier {
    pub fn new(codex: CodexClient, cache: DataCache) -> Self {
        PatchApplier { codex, cache }
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

    /// Apply a unified diff patch to the given project by prompting the LLM to apply it,
    /// write updated files, stage, and commit the changes.
    pub async fn apply(&self, project: &str, patch: &str) -> Result<String> {
        tracing::info!(project = project, "Applying patch via LLM applier");
        // Clean the raw patch to extract a valid unified diff
        let cleaned = Self::clean_patch(patch);
        if cleaned.trim().is_empty() {
            return Err(anyhow!("Patch contained no valid diff; skipping apply"));
        }
        // Prompt LLM to apply the patch and return updated file contents
        let updated = self.codex.apply_patch(project, &cleaned).await?;
        // Parse updated contents into file paths and data
        let mut files: HashMap<String, String> = HashMap::new();
        let mut current: Option<String> = None;
        let mut buffer: Vec<String> = Vec::new();
        for line in updated.lines() {
            if line.starts_with("<<<FILE: ") && line.ends_with(">>>") {
                // flush previous file
                if let Some(prev) = current.take() {
                    let mut data = buffer.join("\n"); data.push('\n');
                    files.insert(prev, data);
                    buffer.clear();
                }
                // start new file
                let fname = &line["<<<FILE: ".len()..line.len() - 3];
                current = Some(fname.to_string());
            } else if line.trim() == "<<<END>>>" {
                if let Some(prev) = current.take() {
                    let mut data = buffer.join("\n"); data.push('\n');
                    files.insert(prev, data);
                    buffer.clear();
                }
            } else if current.is_some() {
                buffer.push(line.to_string());
            }
        }
        // flush last file if no END marker
        if let Some(prev) = current.take() {
            let mut data = buffer.join("\n"); data.push('\n');
            files.insert(prev, data);
        }
        // Write each file to disk
        for (path, data) in &files {
            let full = Path::new(project).join(path);
            if let Some(dir) = full.parent() {
                fs::create_dir_all(dir).await?;
            }
            fs::write(&full, data).await?;
        }
        // Stage all changes
        let status = Command::new("git")
            .arg("add").arg(".")
            .current_dir(project)
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow!("git add failed with status {:?}", status));
        }
        // Generate commit message via LLM
        let msg = self.codex.generate_commit_message(project, patch).await?;
        // Commit staged changes
        let status = Command::new("git")
            .arg("commit").arg("-m").arg(&msg)
            .current_dir(project)
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow!("git commit failed with status {:?}", status));
        }
        Ok(msg)
    }
}