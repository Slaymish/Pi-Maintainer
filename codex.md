Here’s a comprehensive overview of PiMainteno after inspecting the codebase:

1. Purpose  
   • Automate routine code‐maintenance tasks—summaries, refactors, dependency bumps, etc.—via an LLM+Git workflow  
   • Optionally run as a daemon (cron‐style scheduler + systemd monitor + web dashboard) or one‐shot  

2. Tech Stack  
   • Language: Rust, async via Tokio  
   • Storage: sled key/value DB for caching  
   • Web: Axum + Tower for the dashboard/API  
   • LLM: external “codex” CLI invoked via shell  
   • Git: patch application, commit & push  
   • Logging: tracing + tracing‐subscriber  

3. Entry Point & Launch Modes (`src/main.rs`)  
   • CLI flags  
     – `--config <path>` (default “PiMainteno.toml”)  
     – `--one-shot` (run scheduler once and exit)  
   • Daemon mode: spawns three async tasks  
     1. Scheduler loop (cron spec)  
     2. Systemd‐unit monitor (stubbed)  
     3. Web server  

4. Configuration (`PiMainteno.toml`)  
   • `[llm]`: provider, API key, model, extra flags  
   • `[cache]`: path to sled DB  
   • `[scheduler]`: enabled flag, cron schedule, project paths  
   • `[systemd_monitor]`: enabled flag, unit names  
   • `[web]`: host, port  

5. Caching Layer (`src/cache.rs`)  
   • `DataCache` wraps sled  
   • Tracks per-project: last run timestamps, Git HEAD hashes, cached LLM outputs (summaries, patches, commit logs), error flags  

6. LLM Interface (`src/codex_client.rs`)  
   • `CodexClient` shells out to `codex` CLI  
   • Main methods  
     – `summarize_project(path) → String`  
     – `generate_patch(path) → unified‐diff`  
     – `generate_commit_message(path, diff) → String`  
   • Strips JSONL or fenced Markdown and ANSI codes  

7. Project Summarization (`src/summarizer.rs`)  
   • Computes cache key from Git HEAD; reuses if unchanged  
   • Writes `codex.md` at project root  

8. Patch Generation & Application (`src/patcher.rs`)  
   • `PatchGenerator` fetches diff from LLM, caches it  
   • `PatchApplier` runs `git apply --whitespace=fix`  
   • Stages, then calls LLM for commit message; commits & pushes  

9. Scheduler (`src/scheduler.rs`)  
   • `run_once()` iterates configured projects:  
     1. Summarize  
     2. Generate patch  
     3. If patch non-empty: apply, `git add`, commit, push, cache last 50 commits, restart `<project>.service`  
   • Logs start/end times and statuses in sled  

10. Systemd Monitor (`src/systemd_monitor.rs`)  
   • Stub for detecting unit failures (planned D-Bus/journal integration)  
   • Intended to trigger auto-fixes but not yet implemented  

11. Web Dashboard & API (`src/web_server.rs`)  
   • GET `/` → HTML UI with status panels, summaries, patch previews, commit history  
   • GET `/api/status` → scheduler & systemd status JSON  
   • GET `/api/projects` → list of projects  
   • GET `/api/projects/:id/summary` → project summary  
   • “Run Now” button in UI exists but POST `/api/run` handler is not yet wired  

12. Deployment Helper (`upload_service.sh`)  
   • Builds with `cargo build --release`, installs binary to `/usr/local/bin/pi-mainteno`  
   • Copies `PiMainteno.toml` to `/etc/pi-mainteno/`  
   • Restarts systemd service  

13. Observability & Logging  
   • Structured logs via `tracing`  
   • Verbosity controlled by `RUST_LOG`  
   • Logs all LLM calls, Git ops, errors, status changes  

14. Known Gaps & TODOs  
   • Real systemd‐failure detection unimplemented  
   • Manual “Run Now” API endpoint not registered  
   • Could embed an LLM client natively instead of shelling out  
   • No dry‐run or patch‐review workflow  

—  
In sum, PiMainteno is a Rust‐based, Git‐centric automation tool that orchestrates LLM‐driven code maintenance (summaries, patches, commits), schedules it via cron, monitors systemd, and exposes a dashboard—all wired together through sled caching and structured logging.