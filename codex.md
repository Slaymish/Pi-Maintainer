Here’s a comprehensive summary of PiMainteno, gathered by inspecting every file and entry point:

1. Project Layout  
• Root  
  – Cargo.toml, Cargo.lock, README.md  
  – PiMainteno.toml (main configuration, TOML or JSON)  
  – codex.md (auto-generated project summary)  
  – upload_service.sh (build & deploy helper)  
  – cache/ (sled key–value DB storage)  
• src/ – Rust modules  
  – main.rs  
    • CLI entry-point (`cargo run [--config <file>] [--one-shot]`) & daemon bootstrap  
    • Parses args, loads config, sets up logging/tracing  
    • Builds DataCache, CodexClient, ProjectSummarizer, PatchGenerator, PatchApplier, Scheduler, SystemdMonitor, WebServer  
    • One-shot mode: runs scheduler once and exits  
    • Daemon mode: spawns async loops for scheduler, systemd monitor, web server; handles graceful shutdown  
  – config.rs – loads `[llm]`, `[cache]`, `[scheduler]`, `[systemd_monitor]`, `[web]` sections  
  – cache.rs – DataCache abstraction over sled DB (caching LLM outputs keyed by project path + Git HEAD)  
  – codex_client.rs – shells out to the external `codex` CLI for LLM calls  
  – summarizer.rs – generates or updates `<project>/codex.md` based on Git HEAD  
  – patcher.rs  
    • PatchGenerator: asks the LLM for a minimal unified-diff suggestion (and caches it)  
    • PatchApplier: post-processes the diff, invokes the LLM to apply it, writes files, stages & commits changes  
  – scheduler.rs – orchestrates per-project workflows: summarize → generate patch → apply → commit & push → restart service  
  – systemd_monitor.rs – monitors configured systemd units for failures (stubbed for future reactive fixes)  
  – web_server.rs – HTTP server (Axum + Tower) exposing:  
    • HTML dashboard with scheduler controls (enable/disable, interval, “Run Now”), system overview (last run, current project, unit status/failures), per-project panels (summary, patch suggestion, errors, commit log)  
    • JSON API endpoints:  
      – GET /api/status  
      – GET /api/projects  
      – GET /api/projects/:id/summary  
      – POST /api/run (manual trigger)  

2. Entry Points  
• `cargo run [--config <file>] [--one-shot]` – loads config, sets up components, then either runs once or starts the daemon  
• Web UI & JSON API (via web_server.rs)  

3. Core Features & Workflows  
• Proactive Scans (“always-on”): on a cron interval per config  
  1. Summarize each project (skip if Git HEAD unchanged)  
  2. Ask LLM for minimal improvements (unified diff)  
  3. If patch returned, clean & apply via LLM, write files  
  4. `git add .`, generate commit message via LLM, `git commit`, `git push`  
  5. Restart corresponding systemd service (`<basename>.service`)  
• Reactive Fixes (future): detect systemd unit failures and trigger patch workflow  
• Human-in-the-loop: web dashboard allows manual “Run Now”, toggling scheduler, adjusting interval  
• Caching: avoids redundant LLM calls by keying on project paths + Git HEAD; stores summaries, patches, commit logs, error flags in sled DB  
• Web dashboard & API: provides visibility into every step  

4. Technology Stack & Tools  
• Rust + Tokio for async runtime  
• Clap v4 for CLI parsing  
• LLM integration via external `codex` CLI (spawned with `std::process::Command`)  
• sled embedded DB for caching  
• Axum + Tower for HTTP server  
• tracing + tracing-subscriber for logging  
• Git and system commands via shelling out to `git` and `systemctl`  
• Deployment via `upload_service.sh` (builds binary, installs to `/usr/local/bin/pi-mainteno`, installs config, restarts systemd service)  

5. Configuration Highlights (PiMainteno.toml)  
• [llm]: provider, api_key  
• [cache]: path to sled DB  
• [scheduler]: enabled (bool), cron interval (minutes), list of project paths  
• [systemd_monitor]: enabled (bool), list of unit names  
• [web]: host, port  

In sum, PiMainteno is an LLM-driven, always-on maintenance agent that autonomously summarizes codebases, generates & applies patches, commits & pushes changes, restarts services, and exposes full control & visibility via a web UI and JSON API.