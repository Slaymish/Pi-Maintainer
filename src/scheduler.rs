use crate::config::{Config, SchedulerConfig};
use crate::summarizer::ProjectSummarizer;
use crate::patcher::{PatchGenerator, PatchApplier};
use crate::cache::DataCache;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use tokio::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Scheduler {
    cfg: SchedulerConfig,
    summarizer: ProjectSummarizer,
    patch_gen: PatchGenerator,
    patch_applier: PatchApplier,
    cache: DataCache,
}

impl Scheduler {
    pub fn new(
        config: Config,
        summarizer: ProjectSummarizer,
        patch_gen: PatchGenerator,
        patch_applier: PatchApplier,
        cache: DataCache,
    ) -> Self {
        Scheduler {
            cfg: config.scheduler,
            summarizer,
            patch_gen,
            patch_applier,
            cache,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if !self.cfg.enabled {
            return Ok(());
        }
        loop {
            self.run_once().await?;
            sleep(Duration::from_secs(60 * 60 * 24)).await;
        }
    }

    pub async fn run_once(&mut self) -> Result<()> {
        use std::path::Path;
        // Record run start time and status
        let start_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs().to_string();
        self.cache.insert("scheduler.last_run_start", &start_ts)?;
        self.cache.insert("scheduler.status", "running")?;
        // Process each project, updating current project status
        for proj in &self.cfg.projects {
            tracing::info!(project = proj, "Scheduler processing project");
            self.cache.insert("scheduler.current_project", proj)?;
            // Skip non-existent project paths
            if !Path::new(proj).exists() {
                tracing::warn!(project = proj, "Project path does not exist, skipping");
                continue;
            }
            // Summarize project, continue on error
            if let Err(err) = self.summarizer.summarize(proj).await {
                tracing::error!(project = proj, error = %err, "Failed to summarize project, skipping");
                continue;
            }
            // Generate patch
            let patch = match self.patch_gen.generate(proj).await {
                Ok(p) => p,
                Err(err) => {
                    tracing::error!(project = proj, error = %err, "Failed to generate patch for project, skipping");
                    continue;
                }
            };
            if patch.trim().is_empty() {
                tracing::info!(project = proj, "No patch generated; skipping apply and service restart");
                continue;
            }
            // Apply patch to the project directory
            if let Err(err) = self.patch_applier.apply(proj, &patch) {
                tracing::error!(project = proj, error = %err, "Failed to apply patch for project, skipping service restart");
                continue;
            }
            tracing::info!(project = proj, "Patch applied successfully");
            // Generate commit message via LLM
            let commit_msg = match self.patch_gen.commit_message(proj, &patch).await {
                Ok(msg) => msg,
                Err(err) => {
                    tracing::error!(project = proj, error = %err, "Failed to generate commit message");
                    "Apply patch".to_string()
                }
            };
            tracing::info!(project = proj, "Committing changes: {}", commit_msg);
            // Stage changes
            match Command::new("git").arg("add").arg(".").current_dir(proj).status().await {
                Ok(status) if status.success() => tracing::info!(project = proj, "Staged changes"),
                Ok(status) => tracing::error!(project = proj, status = ?status, "git add failed"),
                Err(err) => tracing::error!(project = proj, error = %err, "Failed to run git add"),
            }
            // Commit changes
            match Command::new("git").arg("commit").arg("-m").arg(&commit_msg).current_dir(proj).status().await {
                Ok(status) if status.success() => tracing::info!(project = proj, "Commit succeeded"),
                Ok(status) => tracing::error!(project = proj, status = ?status, "git commit failed"),
                Err(err) => tracing::error!(project = proj, error = %err, "Failed to run git commit"),
            }
            // Push to remote
            match Command::new("git").arg("push").current_dir(proj).status().await {
                Ok(status) if status.success() => {
                    tracing::info!(project = proj, "Push succeeded");
                    // Update commit log in cache
                    let log_key = format!("scheduler.commit_log.{}", proj);
                    // Load existing commit log or default
                    let mut log_list: Vec<String> = self.cache.get(&log_key)
                        .ok().flatten()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();
                    // Append new commit message
                    log_list.push(commit_msg.clone());
                    // Limit to last 50 entries
                    if log_list.len() > 50 {
                        let remove_count = log_list.len() - 50;
                        log_list.drain(0..remove_count);
                    }
                    if let Ok(serialized) = serde_json::to_string(&log_list) {
                        if let Err(err) = self.cache.insert(&log_key, &serialized) {
                            tracing::warn!(project = proj, error = %err, "Failed to cache commit log");
                        }
                    }
                }
                Ok(status) => tracing::error!(project = proj, status = ?status, "git push failed"),
                Err(err) => tracing::error!(project = proj, error = %err, "Failed to run git push"),
            }
            // Restart systemd service corresponding to project
            if let Some(name_os) = std::path::Path::new(proj).file_name() {
                let service = format!("{}.service", name_os.to_string_lossy());
                match Command::new("systemctl").arg("restart").arg(&service).status().await {
                    Ok(status) => {
                        if status.success() {
                            tracing::info!(project = proj, service = %service, "Service restarted successfully");
                        } else {
                            tracing::error!(project = proj, service = %service, status = ?status, "Service restart returned non-zero status");
                        }
                    }
                    Err(err) => {
                        tracing::error!(project = proj, error = %err, "Failed to execute systemctl restart for service {}", service);
                    }
                }
            } else {
                tracing::warn!(project = proj, "Failed to derive service name from path; skipping restart");
            }
        }
        // Record run end time and reset status
        let end_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs().to_string();
        self.cache.insert("scheduler.last_run_end", &end_ts)?;
        self.cache.insert("scheduler.status", "idle")?;
        self.cache.insert("scheduler.current_project", "")?;
        Ok(())
    }
    /// Returns whether the scheduler is enabled in configuration
    pub fn is_enabled(&self) -> bool {
        self.cfg.enabled
    }
}