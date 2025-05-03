use crate::config::{Config, SystemdMonitorConfig};
use crate::patcher::PatchGenerator;
use crate::cache::DataCache;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct SystemdMonitor {
    cfg: SystemdMonitorConfig,
    patch_gen: PatchGenerator,
    cache: DataCache,
}

impl SystemdMonitor {
    pub fn new(config: Config, patch_gen: PatchGenerator, cache: DataCache) -> Self {
        SystemdMonitor {
            cfg: config.systemd_monitor,
            patch_gen,
            cache,
        }
    }

    pub async fn listen(&mut self) -> Result<()> {
        if !self.cfg.enabled {
            self.cache.insert("systemd.status", "disabled")?;
            return Ok(());
        }
        // Initialize status and empty failures list
        self.cache.insert("systemd.status", "listening")?;
        self.cache.insert("systemd.failures", "[]")?;
        tracing::info!(units = ?self.cfg.units, "SystemdMonitor listening for failures");
        loop {
            // Update last checked timestamp
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs().to_string();
            self.cache.insert("systemd.last_checked", &now)?;
            // TODO: detect and record failures in systemd.failures
            sleep(Duration::from_secs(60)).await;
        }
    }
}