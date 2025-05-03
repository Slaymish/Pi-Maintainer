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
    pub model: String,
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

        // Expand tilde in cache path (e.g., "~/path" -> "/home/user/path")
        if let Some(s) = cfg.cache.path.to_str() {
            if let Some(stripped) = s.strip_prefix("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    let mut new_path = home;
                    new_path.push('/');
                    new_path.push_str(stripped);
                    cfg.cache.path = PathBuf::from(new_path);
                }
            }
        }

        // Expand tilde in scheduler project paths
        for project in &mut cfg.scheduler.projects {
            if let Some(stripped) = project.strip_prefix("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    let mut new_proj = home.clone();
                    new_proj.push('/');
                    new_proj.push_str(stripped);
                    *project = new_proj;
                }
            }
        }

        Ok(cfg)
    }
}