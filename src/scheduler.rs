use crate::config::{Config, SchedulerConfig};
use crate::summarizer::ProjectSummarizer;
use crate::patcher::PatchGenerator;
use crate::cache::DataCache;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Scheduler {
    cfg: SchedulerConfig,
    summarizer: ProjectSummarizer,
    patch_gen: PatchGenerator,
    cache: DataCache,
}

impl Scheduler {
    pub fn new(config: Config, summarizer: ProjectSummarizer, patch_gen: PatchGenerator, cache: DataCache) -> Self {
        Scheduler {
            cfg: config.scheduler,
            summarizer,
            patch_gen,
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
            // Generate patch, continue on error
            if let Err(err) = self.patch_gen.generate(proj).await {
                tracing::error!(project = proj, error = %err, "Failed to generate patch for project, skipping");
                continue;
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