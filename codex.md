Here’s a consolidated, up‐to‐date overview of PiMainteno, the Rust-based “always-on” code‐maintenance agent:

1. Purpose  
   • Automate routine project upkeep via Git + LLM, with minimal human intervention  
   • Summaries, refactors, dependency bumps, commit+push, and service restarts  
   • Expose status & controls via a web dashboard and optional systemd monitoring  

2. Configuration  
   – PiMainteno.toml (defaults to `$CWD/PiMainteno.toml` or `/etc/pi-mainteno/`):  
     • [llm]: provider, API key, model, extra CLI flags  
     • [cache]: path to sled key/value DB  
     • [scheduler]: enabled flag, cron spec, list of project paths  
     • [systemd_monitor]: enabled flag, list of unit names  
     • [web]: host, port  

3. Entry Points & Modes  
   – CLI (`src/main.rs`):  
     • `--config <path>` to override config  
     • `--one-shot` to run scheduler once and exit  
   – Daemon mode (no `--one-shot`):  
     1. Scheduler loop (cron-driven)  
     2. Systemd-unit monitor (stub)  
     3. Web server + API  

4. Core Components  
   • Cache (`src/cache.rs`): wraps sled DB; tracks per-project HEAD, last run, LLM outputs, error flags/retries  
   • LLM Interface (`src/codex_client.rs`): shells out to `codex` CLI; key methods:  
     – `summarize_project(path) -> String`  
     – `generate_patch(path) -> unified-diff String`  
     – `generate_commit_message(path, diff) -> String`  
     Post-processing strips JSONL, Markdown fences, ANSI escapes.  
   • Summarizer (`src/summarizer.rs`): if HEAD changed, asks LLM for fresh summary; writes/updates `codex.md` at project root.  
   • Patcher (`src/patcher.rs`):  
     – Fetch or reuse cached diff from LLM  
     – `git apply --whitespace=fix`; stage changes  
     – Invoke LLM for commit message; `git commit` + `git push`  
   • Scheduler (`src/scheduler.rs`): `run_once()` drives 1) summarize, 2) patch gen, 3) apply+commit+push 4) cache pruning + service restart via `systemctl`  
   • Systemd Monitor (`src/systemd_monitor.rs`): placeholder for future D-Bus/journal-based failure detection & auto-remediation  
   • Web Dashboard/API (`src/web_server.rs`):  
     • HTML UI (GET `/`): status panels, summaries, diffs, recent commits  
     • JSON endpoints:  
       – GET `/api/status` → scheduler + systemd status  
       – GET `/api/projects` → project list  
       – GET `/api/projects/:id/summary` → summary markdown  
       – POST `/api/run` (handler present but not yet wired into router)  

5. Auxiliary  
   • `upload_service.sh`: builds (`cargo build --release`), installs binary to `/usr/local/bin/pi-mainteno`, copies config, restarts systemd unit  
   • Logging: structured via `tracing`; verbosity via `RUST_LOG`  
   • Storage: sled K/V; retains last N outputs for caching & rollback  

6. External Dependencies  
   • Rust (async via Tokio)  
   • Axum & Tower (HTTP server)  
   • sled (embedded key/value DB)  
   • External tools: `codex` CLI, `git`, `systemctl`  

7. Known Gaps & TODOs  
   • Real systemd-failure detection/unix-socket integration unimplemented  
   • “Run Now” API endpoint isn’t wired into the HTTP router yet  
   • No built-in dry-run or human-in-the-loop patch review  
   • Potential future native LLM client (vs. shelling out)  

In a nutshell, PiMainteno is a self-service, Git-centric automation tool that leverages an LLM to keep projects healthy—summarizing, generating/applying patches, committing, pushing and monitoring services—while surfacing everything via a web dashboard.