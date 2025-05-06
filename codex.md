Here is a comprehensive summary of the PiMainteno project, based on a thorough examination of all files, features, and entry points:

---

## Project Purpose

**PiMainteno** is an autonomous Rust-based tool for maintaining and improving codebases in-place. It leverages LLMs (via the external `codex` CLI) to periodically summarize codebases, propose unified-diff patches for improvements, apply and commit those patches, push changes, and restart system services. It offers observability and manual control through a web dashboard and JSON API.

---

## Major Features

### 1. Always-On Maintenance Workflow
- **Scheduling**: Regularly triggers a workflow per configured project (via cron interval or manual API/web trigger).
- **Summarization**: Uses LLM to generate/update project summaries (`codex.md`) when the Git HEAD changes.
- **Patch Generation**: LLM suggests a minimal, atomic unified diff to improve the project.
- **Patch Application**: Post-processes, applies (potentially rebasing or amending), writes, stages, commits, and pushes changes.
- **Automated Service Restart**: Restarts corresponding systemd units after changes.

### 2. Caching & Efficiency
- **sled DB-based cache**: Stores LLM outputs (summaries, patches, errors) keyed by project and Git HEAD, to prevent redundant computation and LLM queries.

### 3. Reactive Monitoring (Planned/Future)
- **Systemd monitoring**: Intended to react to systemd unit failures, triggering a summary and emergency repair patch via LLM.

### 4. Observability & Control
- **Web Dashboard**: UI for status panels, manual control of project workflow, summaries, patches, commit logs, and unit status.
- **JSON API**: Endpoints to get status, projects, summaries, and manually trigger the maintenance workflow.

---

## Project Structure & Files

```
PiMainteno.toml        # Main configuration (llm, cache, scheduler, systemd_monitor, web)
cache/                 # sled DB files (LLM output and cache)
src/
  ├── main.rs          # CLI entry point, top-level control flow, logging, config, dispatcher
  ├── config.rs        # Loads and parses PiMainteno.toml sections into Rust structs
  ├── cache.rs         # sled wrappers for LLM output cache
  ├── codex_client.rs  # Spawns the external `codex` CLI for LLM calls
  ├── summarizer.rs    # Summarizes project, manages codex.md
  ├── patcher.rs       # Patch generation (LLM), post-processing, patch application (LLM), git ops
  ├── scheduler.rs     # Orchestrates main workflow on schedule or manual trigger
  ├── systemd_monitor.rs # (Stub) Watches systemd units (for future extensions)
  └── web_server.rs    # HTTP API & dashboard (Axum/Tower-based)
codex.md               # (Auto-generated) Current project summary from LLM
upload_service.sh      # Helper: build, install binary+config, restart systemd service
Cargo.toml, Cargo.lock, README.md # Standard Rust/cargo files
```

---

## Key Entry Points

1. **CLI / Daemon:**
   - `cargo run [--config <file>] [--one-shot]`
     - Parses config, sets up logging.
     - **One-shot**: Executes workflow once, exits.
     - **Daemon**: Spawns async routines for scheduler, systemd monitor, and web server.

2. **Web API/Server:**
   - **/api/status**: System/agent status.
   - **/api/projects**: List of monitored projects.
   - **/api/projects/:id/summary**: Get project summary.
   - **/api/run**: Trigger maintenance routine.

3. **Web Dashboard:**
   - UI includes: scheduler controls, per-project panels (summary, patch, commit log, errors, unit status, run now button).

4. **Upload Script:**
   - `upload_service.sh`: Compiles and installs the PiMainteno binary, config, and (re)starts the associated systemd service.

---

## Configuration (`PiMainteno.toml`)

- `[llm]`: LLM provider, API key.
- `[cache]`: sled DB path.
- `[scheduler]`: enabled, cron interval, project list.
- `[systemd_monitor]`: enabled, units to watch.
- `[web]`: server host/port.

---

## Workflow (Detail)

1. On schedule (or manual run):
   - For each project:
     - If Git HEAD updated → summarize with LLM → update `codex.md`.
     - Ask LLM for a minimal, atomic improvement (unified diff).
     - If patch generated:
       - Post-process the diff, apply via LLM, write files.
       - `git add .`, ask LLM for commit message, `git commit` and `git push`.
       - `systemctl restart <project>.service`.
   - All major steps (and their LLM outputs, errors) are cached for efficiency and traceability.

---

## Technology Stack

- Rust (Tokio async, standard libraries)
- Axum/Tower (web server & API)
- sled (embedded K/V database)
- tracing (structured logging)
- Clap v4 (CLI args)
- External `codex` CLI (LLM access)
- Git via shell
- systemctl via shell

---

## Notable Features & Future Roadmap

- **Minimal Patch Philosophy**: Only minimal, atomic improvements are applied per run (minimizing merge risk).
- **LLM-Powered Everything**: Both summary and patch-application are LLM-driven, including commit messages.
- **System Service Integration**: Maintainer for both the code and systemd service.
- **Built-in Web UI**: Full visibility and manual override (unusual for code-improvement bots).
- **Reactive recovery from service failures** (future): Will auto-summarize failure and attempt a fix.
- **Safe-by-Design**: Only operates in Git repos; all modifications are versioned and logged.

---

## Summary Table

| Component         | Function                                                 |
|-------------------|---------------------------------------------------------|
| main.rs           | CLI/daemon entry, config, dispatcher, logging           |
| config.rs         | PiMainteno.toml parsing                                 |
| cache.rs          | sled DB wrapper, LLM output cache                       |
| codex_client.rs   | Spawns LLM CLI, handles requests                        |
| summarizer.rs     | Generates/updates codebase summary                      |
| patcher.rs        | Patch gen, post-process, apply, git ops                 |
| scheduler.rs      | Orchestrates workflow on schedule/manual                |
| systemd_monitor.rs| Watches/restarts failed services (stub for now)         |
| web_server.rs     | Axum/Tower web & JSON API                               |

---

## In One Sentence

**PiMainteno autonomously maintains, improves, and manages Git-based projects and their services using LLMs, offering full automation, observability, and control.**