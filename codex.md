Here’s a high‐level, end‐to‐end summary of PiMainteno’s architecture, entry points, features and important details:

1. Project Layout  
   • Root  
     – Cargo.toml, Cargo.lock, README.md  
     – PiMainteno.toml (config; TOML or JSON)  
     – codex.md (auto-generated project summary)  
     – upload_service.sh (build & deploy helper)  
     – cache/ (sled DB storage)  
   • src/ – Rust modules:  
     – main.rs – CLI entrypoint & daemon bootstrap  
     – config.rs – loads `[llm]`, `[cache]`, `[scheduler]`, `[systemd_monitor]`, `[web]` sections  
     – cache.rs – DataCache (sled key–value store)  
     – codex_client.rs – shells out to `codex` CLI for LLM calls  
     – summarizer.rs – writes/updates `<project>/codex.md` based on Git HEAD  
     – patcher.rs –  
         • PatchGenerator: ask LLM for a unified-diff suggestion (and cache it)  
         • PatchApplier: clean the diff, invoke LLM to apply it, write files, stage & commit  
     – scheduler.rs – orchestrates per‐project flows: summarize → generate patch → apply → commit & push → service restart  
     – systemd_monitor.rs – stub loop that records status/failures; intended to detect failed units and trigger fixes  
     – web_server.rs – Axum HTTP server exposing:  
         • HTML dashboard with:  
             – Scheduler controls (enable/disable toggle, interval in minutes, “Run Now”)  
             – System overview (last run, current project, systemd status/failures)  
             – Per‐project panels (codex.md summary, patch suggestion, errors, commit log)  
         • JSON API endpoints:  
             – GET /api/status  
             – GET /api/projects  
             – GET /api/projects/:id/summary  
             – POST /api/run  (manual trigger)  

2. Entry Points  
   • `cargo run -- [--config <file>] [--one-shot]`  
     – Loads config, initializes tracing/logging (tracing + env‐filter)  
     – Builds shared `DataCache` and `CodexClient`  
     – Constructs `ProjectSummarizer`, `PatchGenerator`, `PatchApplier`, `Scheduler`, `SystemdMonitor`, `WebServer`  
     – One-shot mode: runs `scheduler.run_once()` and exits  
     – Daemon mode:  
         – Spawns async loops for scheduler (sleep interval from config.cron, default back-off 24h), systemd listener, and web server  
         – Listens for Ctrl+C to shut down gracefully  

3. Core Features & Workflows  
   • Proactive Scans (“always on”): periodically (interval in minutes from cron config)  
       1. Summarize each project, writing or skipping if Git HEAD unchanged  
       2. Ask LLM for minimal unified‐diff improvements  
       3. If a patch is returned, clean and apply it via LLM, write files  
       4. `git add .`, generate commit message via LLM, `git commit`, `git push`  
       5. Restart corresponding systemd service (`<basename>.service`)  
   • Reactive Fixes (future): detect systemd unit failures and trigger patch workflow  
   • Human‐in-the-loop: web UI allows manual “Run Now”, toggling scheduler on/off, adjusting interval  
   • Caching: avoids redundant LLM calls by keying on project paths and Git HEAD hashes; stores summaries, patches, commit logs, error flags in sled DB  
   • Web dashboard & API: visibility into summaries, patch suggestions, errors, service status, commit history  

4. Technology Stack & Tools  
   • Rust async runtime: Tokio  
   • CLI parsing: Clap v4  
   • LLM integration: external `codex` CLI (via `std::process::Command`)  
   • Caching: sled embedded key–value DB  
   • HTTP server: Axum + Tower  
   • Logging/tracing: tracing + tracing-subscriber  
   • Git operations & system commands: shelling out to `git` and `systemctl`  
   • Deployment: `upload_service.sh` builds, installs binary as `/usr/local/bin/pi-mainteno`, copies config, restarts systemd service  

5. Configuration Highlights (PiMainteno.toml)  
   • [llm] provider, api_key  
   • [cache] path to sled DB  
   • [scheduler] enabled (bool), cron interval (minutes), list of project paths  
   • [systemd_monitor] enabled (bool), list of unit names  
   • [web] host, port  

In sum, PiMainteno is an LLM-driven, always-on maintenance agent that summarizes codebases, autonomously generates and applies patches, commits & pushes changes, restarts services, and exposes control & visibility via a rich web UI and JSON API.