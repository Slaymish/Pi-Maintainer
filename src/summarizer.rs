use crate::codex_client::CodexClient;
use crate::cache::DataCache;
use anyhow::Result;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use std::process::Stdio;

#[derive(Clone)]
pub struct ProjectSummarizer {
    codex: CodexClient,
    cache: DataCache,
}

impl ProjectSummarizer {
    pub fn new(codex: CodexClient, cache: DataCache) -> Self {
        ProjectSummarizer { codex, cache }
    }

    pub async fn summarize(&self, project_path: &str) -> Result<()> {
        // Compute cache key for project summary based on Git commit hash
        let cache_key = format!("summary_hash:{}", project_path);
        // Attempt to get current Git HEAD hash
        let git_hash = match tokio::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(project_path)
            .stdout(std::process::Stdio::piped())
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let h = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Some(h)
            }
            _ => {
                tracing::warn!(project = project_path, "Could not determine Git HEAD; skipping cache check");
                None
            }
        };
        // If we have a previous hash and it matches, skip summarization
        if let Some(ref h) = git_hash {
            if let Ok(Some(prev)) = self.cache.get(&cache_key) {
                if &prev == h {
                    tracing::info!(project = project_path, "No changes since last summary, skipping");
                    return Ok(());
                }
            }
        }
        // Generate new summary via Codex
        let summary = self.codex.summarize_project(project_path).await?;
        let path = Path::new(project_path).join("codex.md");
        tracing::info!(path = ?path, "Writing summary to file");
        fs::write(&path, summary).await?;
        // Update cache with new hash
        if let Some(h) = git_hash {
            let _ = self.cache.insert(&cache_key, &h);
            let _ = self.cache.flush();
        }
        Ok(())
    }
}