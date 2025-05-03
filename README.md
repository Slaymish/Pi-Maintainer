# PiMainteno System Architecture

The **PiMainteno** Rust project is structured as a command-line daemon plus a companion web UI for monitoring and logging. A typical high-level layout might look like this:

* `Cargo.toml`, `Cargo.lock`, `README.md`: package metadata and documentation.
* `pi-maintainer-history.jsonl` (or similar): an append-only log of actions/results.
* `src/`: Rust source modules. Key files include `main.rs` (CLI entrypoint) and modules for Codex integration, systemd monitoring, project scanning, summarization, patching, caching, and the web server.
* `tests/`: integration tests (e.g. testing the Codex client or sandbox executor).
* `data/` or `cache/`: a directory (outside of `src`) to store cached metadata, summaries (`codex.md`), and patch diffs.

For example, the `src/` directory might contain files like `codex_client.rs`, `systemd_monitor.rs`, `project_scanner.rs`, `summarizer.rs`, `patcher.rs`, `data_cache.rs`, and `web_server.rs`, each implementing a distinct subsystem. Persistent project summaries (one per project) live in each project’s root as `codex.md`.  (An illustrative tree is shown below.)

&#x20;*Figure: Example PiMainteno folder structure with key files and directories. Root-level files (`Cargo.toml`, `README.md`, logs) sit alongside `src/` (modules) and `tests/`.*

## Core Modules and Responsibilities

* **Configuration & Orchestration**: The `main.rs` (or `lib.rs`) handles startup, parses command-line options (using a CLI parser like **clap**), and loads a configuration file (e.g. listing project paths, Codex settings). It spawns background tasks (systemd listener, daily scanner) under an async runtime (Tokio).

* **SystemdMonitor**: Listens for service failures via systemd. This can use the `systemd` crate’s D-Bus or journal APIs or the `systemctl` crate (which “manages and monitors services through `systemctl`”). When a unit fails, this module identifies the associated project and triggers the patch workflow for that project.

* **Scheduler/ProjectScanner**: Runs periodically (e.g. daily). It iterates over all registered projects, checking for updates or missing summaries. For each project, it ensures a summary is present and looks for improvement opportunities. An async task scheduler (e.g. **tokio-cron-scheduler**) can invoke this scan on a fixed schedule.

* **ProjectSummarizer**: For each project, checks if `codex.md` exists. If not (or if outdated), it calls the LLM to generate one: it lists entry points, describes key modules, etc. This uses the Codex client to ask “Explain this project and list entry points.” The resulting markdown is written to `codex.md` in the project root.

* **CodexClient (LLM Interface)**: Encapsulates calls to the Codex LLM. By default this invokes the **Codex CLI** as a subprocess (via Rust’s `std::process::Command` or a helper like the `duct` crate). The CLI can be run in “full-auto” mode to propose edits. In future the same interface could be implemented using a Rust LLM library (e.g. `allms` or other Rust LLM crates) to talk directly to Codex/OpenAI or an open-source model. The client handles assembling prompts (possibly with context from the project) and parsing outputs.

* **PatchAnalyzer / Generator**: When triggered (by a failure or scan), this module formulates a prompt to Codex to fix the issue or improve code. For example, on a failed systemd service it gathers the failing log and context and asks “Apply minimal patch to fix this failure.” On proactive scans it might ask “Suggest low-risk improvements for this project.” The Codex client returns diff suggestions or patched code.

* **PatchApplier**: Takes Codex’s suggested changes (diffs or new code) and applies them. This may use the **git2** crate (Rust bindings for libgit2) to create a new commit or to apply a patch safely. Low-risk patches (e.g. lint fixes) can be auto-committed. All patches (even auto-applied) are logged. If a patch is rejected or requires review, it is left in a pending state.

* **DataCache / Metadata Store**: Centralizes caching and state. It tracks, per project: last scan timestamp, last known Git commit or code fingerprint, list of applied patch IDs, and cached Codex outputs. Implementation options include an embedded database (e.g. **sled** for a key-value store) or simple JSON files. Using `serde` to (de)serialize structs into JSON allows storing metadata (e.g. `last_scan: <timestamp>`, `summary_hash`, etc.) persistently. Cached data includes previous LLM responses, so that repeated scans with no code change do not re-run Codex.

* **Logger**: Uses the Rust `log` crate for structured logging. To integrate with systemd’s journal, the `systemd-journal-logger` crate can be used to send logs to journald. All automatic actions (patches suggested/applied, errors, etc.) are logged with context. The web UI reads from this log history as well.

* **WebUI Server**: A small HTTP server (using a Rust framework like **Axum** or **Actix Web**) provides a dashboard. It lists all projects, shows recent activities, proposed patches, and summaries. For example, clicking a project shows its `codex.md`, pending patches with diffs, and a log of events. The server can expose routes like `/projects/{name}` or use websockets for live updates. (Axum emphasizes ergonomics and composability; Actix Web is another high-performance choice.) Templating (e.g. \[Tera]) and static files can be used for frontend assets.

## Codex Integration and Caching

All calls to the Codex LLM are funneled through an abstract interface (e.g. a `CodexClient` trait). By default this implementation simply shells out to the `codex` CLI tool: for example:

```bash
codex --provider openai --approval-mode full-auto "Explain this codebase and list its entry points."
```

Rust’s `std::process::Command` (or the `duct` crate) can capture the output or diff from this command. We recommend running Codex in **sandboxed/full-auto** mode to let it generate edits autonomously. Optionally, an open-source Rust LLM integration (e.g. via the `allms` or `llm` crates) could be added later to invoke Codex or another model directly without the CLI layer.

To avoid redundant LLM calls, **caching** is essential. Each Codex prompt and its result are keyed by the project state. For example, include the project’s Git commit hash (or a hash of its files) in the key. If nothing has changed since the last call, the cached response can be reused. Practically, after generating a summary or patch, store it (in `codex.md` or a cache DB) along with a checksum. On the next scan, compute the current checksum and skip the Codex invocation if it matches. A high-performance embedded store like **sled** (a pure Rust key-value DB) works well for this purpose. Alternatively, store JSON records with `serde_json` (e.g. `project_cache.json`) and load them at startup. In summary, the workflow is:

1. **Check cache:** For a given project and task (summarize or patch), see if there’s a cached result for the current code state.
2. **Invoke Codex if needed:** If no cache hit, call the Codex CLI with an appropriate prompt.
3. **Save output:** Store the LLM’s output (summary text or patch diff) in a cache (DB or file) and update `codex.md` or diff logs.
4. **Apply if auto:** If the patch is “auto-approval”, use Git to commit; else leave it for review.

This ensures that PiMainteno only re-queries the LLM for new or changed code, saving time and API usage.

## Project Data: Metadata, Diffs, and Summaries

Each monitored project has associated metadata and generated content:

* **Project Metadata:** Maintain a record for each project with fields like `path`, `last_checked`, `last_commit_hash`, etc. This can be a struct serialized by **serde** into JSON (e.g. one JSON file per project or an embedded DB entry). On startup or scan, the scheduler compares the current state to this metadata.

* **Summaries (`codex.md`):** The summarizer writes a human-readable summary in Markdown to `<project>/codex.md`. This serves as a quick overview of the codebase (list of binaries, libraries, APIs, etc.). If the file exists and the project’s code is unchanged, PiMainteno skips re-generating it.

* **Patch Diffs:** When Codex suggests changes, the diff is captured. Applied diffs are either committed to the project’s Git repo (using `git2`) or stored as patch files. Each patch can be given an ID or stored by timestamp. A global log (`pi-maintainer-history.jsonl` or a database) records: timestamp, project, change description, and the patch contents. This allows auditing what was done and avoids re-applying the same patch twice.

By systematically recording metadata and outputs, PiMainteno avoids duplicate work. For example, if the same failed service recurs without code changes, PiMainteno will notice the last attempt and skip re-applying the identical fix.

## Technology Stack and Crates

Key Rust crates and tools for PiMainteno include:

* **CLI Parsing:** Use **Clap** (v3+) for robust argument parsing. It auto-generates help messages and supports subcommands, which is useful for future extensibility.

* **Subprocess Calls:** Use Rust’s standard library (`std::process::Command`) or the [`duct`](https://crates.io/crates/duct) crate to run external commands (Codex CLI, systemctl, shell scripts).

* **Systemd Integration:** The [`systemd`](https://docs.rs/systemd) crate provides APIs for interacting with systemd’s journal and units (and even D-Bus). The [`systemctl`](https://docs.rs/systemctl) crate can also be used to programmatically query or manage units (“manage and monitor services through `systemctl`”). These let PiMainteno detect service failures and inspect service definitions.

* **Git Operations:** Use the **git2** crate (Rust/libgit2) for repository manipulation. It can open repos, make commits, create branches, and generate diffs. This is key for applying patches and tracking project versions.

* **Async Runtime/Scheduling:** Use **Tokio** for async execution. For periodic tasks (daily scans), a crate like `tokio-cron-scheduler` (Cron-like scheduling in Tokio) or simply an async loop with `tokio::time::sleep` can trigger jobs.

* **Caching/DB:** **sled** is a high-performance embedded database (key-value store) ideal for caching data. It supports atomic transactions and is thread-safe. Alternatively, **SQLite** via `rusqlite` could be used if relational storage is preferred.

* **Data Serialization:** Use **Serde** and **serde\_json** to (de)serialize metadata, logs, and patch records. Serde makes it easy to map Rust structs to JSON for files like project metadata or log entries.

* **Logging:** Use the `log` facade with a backend. For example, **systemd-journal-logger** can direct logs into journald. This ensures PiMainteno’s logs appear in the system journal.

* **Web Framework:** For the UI, use **Axum** or **Actix Web**. *Axum* is ergonomic and modular, built on Tokio. *Actix Web* is high-performance and feature-rich (HTTP/2, etc.). Either allows writing REST endpoints or serving HTML templates. Additionally, a templating engine (e.g. \[Tera]) or a single-page app (with e.g. Yew or React) can render the frontend.

* **Configuration:** Use a crate like **config** or **clap’s** config file features to manage settings (e.g., Codex API key, project list).

* **Notifications (future):** For team alerts, crates like `slack_api` or calling a webhook via `reqwest` can post messages to Slack/Teams. For email, the `lettre` crate is available. These would be part of an extensibility layer (see below).

## Extensibility and Future Enhancements

PiMainteno’s modular design allows for many extensions:

* **Multi-Agent Planning:** The system could orchestrate multiple LLM agents. For example, one agent could propose a high-level plan of action (e.g. “update dependencies, refactor this module”), then spawn sub-agents to implement parts. A task queue or actor model (using Tokio tasks or a library like *capnp* RPC) could manage this.

* **Advanced Version Control Integration:** Beyond local Git commits, PiMainteno could create pull requests or issues on GitHub/GitLab. For this, use crates like [`octocrab`](https://crates.io/crates/octocrab) (GitHub API) to push changes and open PRs. Tight integration with CI/CD could allow automatic testing of patches before merge.

* **Customizable Policies:** Right now “low-risk” patches might be auto-applied. In future, make patch approval pluggable (e.g. using machine learning classifiers or human review workflows).

* **Broader Notifications:** Integrate with chat/email to notify developers of fixes. A `Notifier` module could support multiple channels (Slack, email, SMS) configured at runtime.

* **Web UI Enhancements:** Add authentication, role-based access (so team members can review patches), and real-time updates (via WebSockets).

* **Additional Analysis Agents:** Plug in static analysis tools (Linters, Clippy, security scanners) alongside Codex to cross-validate suggestions. Results can be fed into the summary/patch workflow.

Each new feature can be added as a separate module or service. For example, a “NotificationManager” module could encapsulate all team comms, and integrate with the core scheduler to send alerts when certain events occur. Because the code is written in Rust with clear interfaces between components (e.g. `CodexClient` trait, `ProjectStore` for metadata, etc.), new agents can be wired in with minimal changes to existing code.

**Summary:** PiMainteno’s architecture centers on a set of Rust modules (Codex interface, systemd listener, scanner, patch applier, data/cache store, and web server), organized under a common project. It uses well-known crates—Clap for CLI arguments, `systemd`/`systemctl` for service integration, git2 for Git tasks, and Axum or Actix for the web UI. By abstracting Codex calls and caching their results (using Serde-serialized JSON or an embedded DB), PiMainteno efficiently monitors projects, auto-generates improvements, and logs all activity. This modular design makes it portable and extensible: it can run on any machine with Rust and systemd, and can grow to include team notifications, advanced planning, and tighter VCS workflows.

**Sources:** Crate documentation for Clap, systemd integration, Axum, Actix Web, git2, Serde/serde\_json, and sled has been consulted to inform this design. These references illustrate the recommended Rust libraries and APIs for each subsystem.

