use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub cache: CacheConfig,
    pub llm: LLMConfig,
    pub scheduler: SchedulerConfig,
    pub systemd_monitor: SystemdMonitorConfig,
    pub web: WebConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    #[serde(flatten)]
    pub options: Option<toml::value::Table>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub cron: String,
    pub projects: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SystemdMonitorConfig {
    pub enabled: bool,
    pub units: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

pub struct ConfigLoader;

impl ConfigLoader {
    pub async fn from_path(path: impl AsRef<Path>) -> Result<Config> {
        let content = fs::read_to_string(path.as_ref()).await?;
        let ext = path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("toml");
        let mut cfg: Config = if ext.eq_ignore_ascii_case("json") {
            serde_json::from_str(&content)?
        } else {
            toml::from_str(&content)?
        };

        // Expand tilde (~) to home directory in cache path and scheduler projects
        if let Some(home) = std::env::var("HOME").ok() {
            if let Some(s) = cfg.cache.path.to_str() {
                if let Some(stripped) = s.strip_prefix("~/") {
                    cfg.cache.path = PathBuf::from(&home).join(stripped);
                }
            }
            for project in &mut cfg.scheduler.projects {
                if let Some(stripped) = project.strip_prefix("~/") {
                    *project = PathBuf::from(&home)
                        .join(stripped)
                        .to_string_lossy()
                        .into_owned();
                }
            }
        }

        Ok(cfg)
    }
}
