Here’s a high-level overview of the **PiMainteno** project—a Rust-based “self-healing” code-maintenance daemon with a companion web UI:

1. Purpose  
   • Automatically summarize, analyze, patch, commit and push improvements to one or more code projects using an LLM (via the external `codex` CLI).  
   • Optionally monitor systemd services for failures and trigger fixes.  
   • Expose a web dashboard to view summaries, patch suggestions, commit history and trigger manual runs.

2. Entry Point & CLI  
   • **src/main.rs**  
     – Uses **clap** to parse:  
       • `--config <path>` (default “PiMainteno.toml”)  
       • `--one-shot` (run a single scan & exit)  
     – Initializes tracing/logging, loads configuration, builds shared components, then either:  
       • Runs one-shot (`scheduler.run_once()`) and exits  
       • Or spawns three long-lived Tokio tasks:  
         1. **Scheduler** loop (daily scans)  
         2. **SystemdMonitor** listener (stubbed)  
         3. **WebServer** HTTP UI  
       – Handles graceful shutdown on Ctrl+C.

3. Configuration  
   • **PiMainteno.toml** (TOML or JSON) with sections:  
     – `[llm]`: provider, api_key, model, extra options  
     – `[cache]`: path to sled database  
     – `[scheduler]`: enabled, cron schedule, list of project paths  
     – `[systemd_monitor]`: enabled, list of units  
     – `[web]`: host & port for dashboard  
   • **src/config.rs** implements `ConfigLoader::from_path`, deserializes via `serde`, expands `~/`.

4. Core Components  
   a. **DataCache** (`src/cache.rs`)  
      – Wraps a **sled** key-value DB for caching prompts, summaries, patch texts, status flags, commit logs, etc.  

   b. **CodexClient** (`src/codex_client.rs`)  
      – Shells out to the external `codex` CLI in the project directory.  
      – Three main methods:  
        1. `summarize_project(path)`: returns a text summary  
        2. `generate_patch(path)`: returns a raw unified-diff patch  
        3. `generate_commit_message(path, diff)`: returns a one-sentence commit message  
      – Strips ANSI/control chars and Markdown fences; parses JSON-L lines to extract assistant outputs.

   c. **ProjectSummarizer** (`src/summarizer.rs`)  
      – Computes a cache key based on `git rev-parse HEAD`.  
      – If HEAD hasn’t changed since last summary, skips LLM call.  
      – Otherwise, invokes `CodexClient::summarize_project`, writes `codex.md` in the project root, updates cache.

   d. **PatchGenerator & PatchApplier** (`src/patcher.rs`)  
      – **PatchGenerator**: calls `CodexClient::generate_patch`, caches the diff under `scheduler.patch.<project>`.  
      – Generates a commit message via LLM when asked.  
      – **PatchApplier**: applies a unified diff via `git apply --whitespace=fix` in the project directory.

   e. **Scheduler** (`src/scheduler.rs`)  
      – On each run:  
        1. Records start time/status in cache  
        2. Iterates configured projects:  
           • Ensures path exists  
           • Summarizes via `ProjectSummarizer`  
           • Generates AI patch (`PatchGenerator`)  
           • If non-empty patch:  
             – Applies it (`PatchApplier`)  
             – Stages, commits (LLM-generated message), pushes to remote  
             – Logs last 50 commit messages in cache under `scheduler.commit_log.<project>`  
             – Restarts the corresponding systemd unit (`<project-dirname>.service`)  
        3. Records end time/status  
      – Exposes `run_once()` for manual triggering; loops on a 24-h interval when run as a daemon.

   f. **SystemdMonitor** (`src/systemd_monitor.rs`)  
      – Stubbed: marks status “listening” and periodically updates `systemd.last_checked` in cache.  
      – Intended to detect failures for configured units and enqueue patch workflows, but actual detection is unimplemented.

   g. **WebServer** (`src/web_server.rs`)  
      – Built with **axum**. Serves:  
        • **GET /** → HTML dashboard showing:  
          – Scheduler & systemd monitor statuses  
          – For each project: summary (from `codex.md`), latest patch suggestion, patch errors, commit history  
          – “Run Now” button (JS calls POST `/api/run`)  
        • **GET /api/status** → JSON of scheduler & systemd statuses  
        • **GET /api/projects** → JSON list of configured projects  
        • **GET /api/projects/:id/summary** → JSON with a single project’s summary  
      – Note: the POST `/api/run` route invoked by the dashboard’s button is not actually registered in the router.

5. Deployment Helper  
   • **upload_service.sh**  
     – `cargo build --release`  
     – Installs the binary (`PiMainteno`) to `/usr/local/bin/pi-mainteno`  
     – Copies `PiMainteno.toml` to `/etc/pi-mainteno/`  
     – Restarts (& checks) `Pi-Maintainer.service` in systemd.

6. Logs & Observability  
   • Uses **tracing** + **tracing-subscriber** for structured logging.  
   • Cache keys (e.g. `scheduler.status`, `systemd.status`, `scheduler.patch_error.<proj>`) feed both the dashboard and JSON API.

7. Notable Details & To-Dos  
   • The systemd failure detection loop is currently a placeholder (no actual failure polling).  
   • The web UI’s manual run button points at `/api/run`, but no POST handler is wired up.  
   • All LLM calls funnel through the `codex` CLI; you’ll need that installed & configured in PATH.  
   • Sled DB files (“cache” dir) store all state; deleting it will force re-summaries and lose history.  
   • Project summaries live in `<project>/codex.md`.

In short, **PiMainteno** ties together: a TOML-driven config, a sled cache, a Codex-backed LLM interface, Git operations to auto-patch projects, a scheduler & systemd listener, and an Axum-powered web dashboard.