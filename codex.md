Here’s a high-level, end-to-end summary of PiMainteno, based on crawling the entire repo:

1. Purpose  
   • Fully automate routine code maintenance (summaries, refactors, dependency bumps, etc.) via an LLM + Git workflow  
   • Integrates with systemd services and exposes a simple web dashboard  

2. Entry Points & Launch Modes  
   • `src/main.rs` is the single binary entry point  
     – CLI flags:  
       • `--config <path>` (default “PiMainteno.toml”)  
       • `--one-shot` (run the scheduler once and exit)  
     – In daemon mode (no `--one-shot`): spawns three async tasks under Tokio:  
       1. Scheduler loop (driven by a cron spec)  
       2. Systemd-unit monitor  
       3. Axum-based web server  
     – Graceful shutdown on Ctrl+C  

3. Configuration  
   • File format: TOML (or JSON) with `~` expansion  
   • Sections:  
     – `[llm]`: provider, API key, model, extra flags  
     – `[cache]`: path to on-disk sled DB  
     – `[scheduler]`: enabled flag, cron schedule, list of project paths  
     – `[systemd_monitor]`: enabled flag, list of systemd unit names  
     – `[web]`: host, port  

4. Caching Layer (`src/cache.rs`)  
   • `DataCache` wraps a sled key/value store  
   • Tracks: last run timestamps, Git HEAD hashes, cached LLM outputs (summaries, patches, commit logs), error flags  

5. LLM Interface (`src/codex_client.rs`)  
   • `CodexClient` shells out to the external `codex` CLI  
   • Core methods:  
     1. `summarize_project(path) -> String`  
     2. `generate_patch(path) -> unified-diff`  
     3. `generate_commit_message(path, diff) -> String`  
   • Post-processes JSON-L or fenced-Markdown, strips ANSI codes  

6. Project Summarization (`src/summarizer.rs`)  
   • `ProjectSummarizer` computes a cache key from Git HEAD  
   • If unchanged, reuses cached summary; otherwise calls LLM and writes `codex.md` in the project root  

7. Patch Generation & Application (`src/patcher.rs`)  
   • `PatchGenerator` invokes LLM for a diff and caches it  
   • `PatchApplier` applies the diff via `git apply --whitespace=fix`  
   • After applying, stages changes, calls LLM for a commit message, commits, and pushes  

8. Scheduler (`src/scheduler.rs`)  
   • `Scheduler::run_once()` does, for each configured project in order:  
     1. Summarize  
     2. Generate patch  
     3. If patch non-empty:  
        – Apply patch, `git add`, commit with LLM-generated message, `git push`  
        – Record last 50 commit messages in cache  
        – Restart related systemd unit (`<project-dirname>.service`)  
     4. Log start/end times, success/failure statuses in the sled cache  

9. Systemd Monitor (`src/systemd_monitor.rs`)  
   • Stubbed listener for systemd unit failures  
   • Intended to detect crashes via D-Bus or journal and enqueue auto-fixes, but actual integration is unimplemented  

10. Web Dashboard (`src/web_server.rs`)  
    • Built on Axum + Tower  
    • Routes:  
      – GET `/` → HTML UI with status panels, summaries, patch previews, commit log  
      – GET `/api/status` → scheduler & systemd JSON status  
      – GET `/api/projects` → list of projects  
      – GET `/api/projects/:id/summary` → single project summary  
    • UI has a “Run Now” button, but the POST `/api/run` handler is not yet wired up  

11. Deployment Helper (`upload_service.sh`)  
    • `cargo build --release` → installs binary to `/usr/local/bin/pi-mainteno`  
    • Copies `PiMainteno.toml` to `/etc/pi-mainteno/`  
    • Restarts (`Pi-Maintainer.service`) and shows status  

12. Observability & Logging  
    • Uses `tracing` + `tracing-subscriber` for structured logs  
    • Verbosity controlled via `RUST_LOG` environment variable  
    • All LLM calls, Git operations, errors, and status changes are logged  

13. Known Gaps & TODOs  
    • Real systemd-failure detection (D-Bus/journal integration) is missing  
    • Manual “Run Now” API endpoint is unregistered  
    • Could embed an LLM client directly (instead of shelling out)  
    • Patch-review or “dry-run” workflow is not yet supported  

—  
PiMainteno ties together a clap-driven CLI, a sled cache, an external codex-based LLM interface, Git-centric patch application, a cron-style scheduler plus systemd listener, and an Axum web dashboard to automate (and audit) continuous code maintenance.