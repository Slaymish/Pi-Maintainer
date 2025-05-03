use std::path::PathBuf;
use clap::Parser;
use tokio::signal;
use tracing_subscriber::prelude::*;
// Initialize tracing with environment filter for log level configuration

mod config;
mod scheduler;
mod systemd_monitor;
mod codex_client;
mod summarizer;
mod patcher;
mod cache;
mod web_server;

/// PiMainteno CLI arguments
#[derive(Parser, Debug)]
#[clap(name = "PiMainteno", version, about = "Self-healing code maintainer daemon")]
struct Cli {
    /// Path to the configuration file (TOML/JSON)
    #[clap(short, long, value_parser = clap::value_parser!(PathBuf), default_value = "PiMainteno.toml")]
    config: PathBuf,

    /// Run a single scan & exit (no long-lived daemon)
    #[clap(long)]
    one_shot: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Parse CLI & initialize tracing
    let args = Cli::parse();
    // Configure tracing subscriber with environment filter and fmt layer
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
    tracing::info!(?args, "Starting PiMainteno");

    // 2. Load configuration
    let cfg = config::ConfigLoader::from_path(&args.config).await?;
    tracing::info!(config = ?cfg, "Configuration loaded");

    // 3. Initialize shared components
    // Use cache.path from configuration
    let data_cache = cache::DataCache::new(&cfg.cache.path).await?;
    let codex = codex_client::CodexClient::new(&cfg.llm);

    // 4. Build domain modules
    let summarizer = summarizer::ProjectSummarizer::new(codex.clone(), data_cache.clone());
    let patch_gen   = patcher::PatchGenerator::new(codex.clone(), data_cache.clone());
    let patch_app   = patcher::PatchApplier::new(data_cache.clone());

    // 5. Scheduler for periodic scans
    // Wrap scheduler in Arc<Mutex<>> to allow manual triggers from the web UI
    use std::sync::Arc;
    use tokio::sync::Mutex;
    let scheduler = Arc::new(Mutex::new(
        scheduler::Scheduler::new(
            cfg.clone(),
            summarizer.clone(),
            patch_gen.clone(),
            patch_app.clone(),
            data_cache.clone(),
        )
    ));

    // 6. Systemd-based failure listener
    let mut systemd = systemd_monitor::SystemdMonitor::new(
        cfg.clone(), patch_gen.clone(), data_cache.clone()
    );

    // 7. Web UI server
    let webui = web_server::WebServer::new(
        cfg.web.clone(),
        data_cache.clone(),
        cfg.scheduler.projects.clone(),
        scheduler.clone(),
    );

    // 8. One-shot mode for CI or manual run
    if args.one_shot {
        tracing::info!("Running one-shot scan and exiting");
        // Directly run a single scan
        scheduler.lock().await.run_once().await?;
        return Ok(());
    }

    // 9. Dispatch tasks
    // Spawn periodic scheduler loop
    {
        let scheduler_loop = scheduler.clone();
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            // interval of 24 hours
            let interval = Duration::from_secs(60 * 60 * 24);
            loop {
                {
                    let mut sched = scheduler_loop.lock().await;
                    // Skip run if scheduler disabled
                    if !sched.is_enabled() {
                        break;
                    }
                    if let Err(err) = sched.run_once().await {
                        tracing::error!(error = %err, "Scheduler encountered an error");
                    }
                }
                sleep(interval).await;
            }
        });
    }

    tokio::spawn(async move {
        if let Err(err) = systemd.listen().await {
            tracing::error!(error = %err, "SystemdMonitor encountered an error");
        }
    });

    tokio::spawn(async move {
        if let Err(err) = webui.run().await {
            tracing::error!(error = %err, "WebServer encountered an error");
        }
    });

    // 10. Wait for shutdown signal (Ctrl+C)
    tracing::info!("Daemon running; press Ctrl+C to stop");
    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received, exiting gracefully");

    // 11. (Optional) perform cleanup here
    patch_app.shutdown().await?;
    Ok(())
}

