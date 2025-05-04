Here’s a high-level overview of PiMainteno, distilled from the codebase and configuration:

1. Project Layout  
   • Cargo.toml, Cargo.lock, README.md  
   • PiMainteno.toml – main configuration file (TOML)  
   • codex.md – auto-generated project summary  
   • upload_service.sh – helper to build, install binary & config, and restart its systemd service  
   • cache/ – sled key–value DB files for caching LLM outputs  
   • src/  
     – main.rs – CLI entry point, parses args, loads config, sets up logging, and either runs one-shot or spawns the daemon  
     – config.rs – deserializes `[llm]`, `[cache]`, `[scheduler]`, `[systemd_monitor]`, and `[web]` sections  
     – cache.rs – wraps sled to cache summaries, patches, and error flags keyed by project path + Git HEAD  
     – codex_client.rs – spawns the external `codex` CLI for LLM calls  
     – summarizer.rs – generates or updates each project’s `codex.md` summary (skips if HEAD unchanged)  
     – patcher.rs  
         • PatchGenerator – asks the LLM for a minimal unified-diff suggestion, caches it  
         • PatchApplier – post-processes the diff, asks the LLM to rebase/apply changes cleanly, writes files, stages, commits & pushes them  
     – scheduler.rs – orchestrates the “always-on” workflow on a cron interval or manual trigger  
     – systemd_monitor.rs – (stub) watches configured systemd units for failures to trigger reactive fixes in future  
     – web_server.rs – Axum/Tower-based HTTP server exposing:  
         • HTML dashboard (scheduler controls, per-project panels with summaries, patches, errors, commit log, unit status)  
         • JSON API  
             – GET  /api/status  
             – GET  /api/projects  
             – GET  /api/projects/:id/summary  
             – POST /api/run  

2. Entry Points  
   • `cargo run [--config <file>] [--one-shot]`  
     – One-shot mode: run scheduler once & exit  
     – Daemon mode: spawn async loops for scheduler, systemd monitor, and web server  
   • Web UI & JSON API for dashboard and manual triggers  
   • upload_service.sh for deploying the binary as a systemd service  

3. Core Workflow (“Always-On” Mode)  
   1. For each configured project path:  
       a. Summarize codebase if Git HEAD changed  
       b. Ask LLM for minimal code-improvement patch (unified diff)  
       c. If non-empty patch: post-process & apply patch via LLM, write files  
       d. `git add .`, ask LLM for commit message, `git commit`, `git push`  
       e. Restart the corresponding systemd service (`<project-name>.service`)  
   2. Repeat on configured cron interval (per-project)  
   3. Manual “Run Now” or toggling via web UI/API  

4. Reactive Monitoring (Future)  
   • systemd_monitor watches units for failures  
   • Intended to trigger an emergency repair workflow by summarizing the failure and generating a fix  

5. Caching & Efficiency  
   • sled DB caches LLM outputs (summaries, patches, commit logs) by project + Git HEAD  
   • Avoids redundant LLM calls when nothing has changed  

6. Technology Stack  
   • Rust + Tokio for async  
   • Clap v4 for CLI parsing  
   • sled embedded DB  
   • Axum + Tower for HTTP server  
   • tracing + tracing-subscriber for structured logging  
   • External `codex` CLI for LLM integration  
   • Git & systemctl via shell commands  

7. Configuration Highlights (PiMainteno.toml)  
   • [llm]: provider, api_key  
   • [cache]: path to sled database  
   • [scheduler]: enabled (bool), cron interval (minutes), list of project paths  
   • [systemd_monitor]: enabled (bool), list of unit names to watch  
   • [web]: host, port for dashboard/API  

In short, PiMainteno is an autonomous Rust-based maintenance agent that continually (or on-demand) summarizes codebases, solicits minimal LLM-driven improvements, applies and commits them, pushes changes, and restarts services—with full visibility and control via a built-in web dashboard and JSON API.