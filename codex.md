Here’s a high-level overview of the entire PiMainteno codebase, with every entrypoint, feature and important detail pulled straight from the source:

1. Project Goal  
   • Provide a “self-healing” maintenance agent that uses an external LLM (via the `codex` CLI) to  
     – Summarize arbitrary code projects into a `codex.md` file  
     – Propose code patches  
     – (Stub) apply patches to the project  
     – Monitor systemd failures (stub)  
     – Expose a minimal web dashboard showing status  

2. Repository Layout  
   – PiMainteno.toml (or .json) – user‐supplied configuration  
   – codex.md – generated per‐project summary file  
   – Cargo.toml / Cargo.lock – Rust crate metadata  
   – src/  
     • main.rs  
     • config.rs  
     • cache.rs  
     • codex_client.rs  
     • summarizer.rs  
     • patcher.rs  
     • scheduler.rs  
     • systemd_monitor.rs  
     • web_server.rs  

3. Configuration (src/config.rs)  
   • Supports TOML or JSON, auto‐expands `~/`  
   • Sections:  
     – `[llm]` provider, api_key, model, extra CLI args  
     – `[cache]` path to sled DB  
     – `[scheduler]` enabled flag, cron spec (currently ignored), list of project paths  
     – `[systemd_monitor]` enabled flag, list of systemd unit names  
     – `[web]` host & port  

4. CLI Entry Point (src/main.rs)  
   • Uses Clap to parse:  
     – `--config <path>` (default `PiMainteno.toml`)  
     – `--one-shot` (run scheduler once and exit)  
   • Startup sequence:  
     1. Load & validate config  
     2. Initialize tracing/logging  
     3. Open sled cache  
     4. Create CodexClient (shells out to `codex`)  
     5. If `--one-shot`: call `scheduler.run_once()` then exit  
     6. Else spawn Tokio tasks:  
        a. Scheduler loop (runs every 24 h)  
        b. Systemd‐monitor loop (stub)  
        c. Axum HTTP server for dashboard  
     7. Graceful Ctrl-C shutdown  

5. Cache Layer (src/cache.rs)  
   • Thin wrapper on sled::Db for String→String storage  
   • Not yet used by higher-level logic (reserved for dedupe, retries, etc.)  

6. LLM Integration (src/codex_client.rs)  
   • Runs `codex` CLI in the target project directory, streams JSON-L output  
   • Exposed methods:  
     – `summarize_project(path: &Path) -> Result<String>`  
     – `generate_patch(context: &str) -> Result<String>` (stub returning empty)  

7. Summarizer (src/summarizer.rs)  
   • Calls `summarize_project()` for each configured path  
   • Writes (or overwrites) `<project>/codex.md` with the raw LLM summary  

8. Patcher (src/patcher.rs)  
   • `PatchGenerator.generate()` – stub (no real diff logic)  
   • `PatchApplier.apply()` – logs the patch but does not modify files or git  

9. Scheduler (src/scheduler.rs)  
   • `run_once()`:  
     – Checks each project path exists  
     – Runs summarizer → patcher.generate → patcher.apply  
     – Logs and skips missing paths  
   • Daemon mode: calls `run_once()`, sleeps 24 h, repeats (ignores configured cron)  

10. Systemd Monitor (src/systemd_monitor.rs)  
    • Stub loop that logs which units would be monitored  
    • No real Journal or failure detection yet  

11. Web Dashboard (src/web_server.rs)  
    • Axum-based HTTP server bound to `[web].host`:`[web].port`  
    • Single GET `/` route returning static HTML header  

12. Key Dependencies  
    • Rust + Tokio async runtime  
    • Clap for CLI parsing  
    • Anyhow for error handling  
    • Tracing + tracing-subscriber for structured logs  
    • Serde (TOML/JSON) for config  
    • Sled for embedded key/value cache  
    • Axum & Tower for the HTTP dashboard  
    • External `codex` CLI for all LLM work  

13. Current Limitations & TODOs  
    • Patch generation & application are just stubs—no real git or diff support  
    • Scheduler ignores the configured cron spec (hard-coded to 24 h)  
    • Systemd monitor has no real integration—just a placeholder loop  
    • Dashboard UI is static and minimal  
    • Cache layer exists but isn’t leveraged  
    • No retry/backoff or error-handling around LLM calls  

In sum, PiMainteno scaffolds a scheduled, LLM-driven workflow for project summarization, patch proposal, and monitoring, but leaves the core diffing/patching and systemd failure detection to be implemented in future iterations.