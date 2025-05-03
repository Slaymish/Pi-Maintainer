use crate::cache::DataCache;
use crate::config::WebConfig;
use crate::scheduler::Scheduler;
use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    response::{Html, Json},
    extract::Path,
    http::StatusCode,
};
use serde::Serialize;
use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::SocketAddr;

/// WebServer serves the HTTP dashboard and API
#[derive(Clone)]
pub struct WebServer {
    config: WebConfig,
    cache: DataCache,
    projects: Vec<String>,
    /// Shared scheduler for manual trigger
    scheduler: Arc<Mutex<Scheduler>>,
}

impl WebServer {
    /// Create a new WebServer with given configuration, cache, projects, and scheduler
    pub fn new(
        config: WebConfig,
        cache: DataCache,
        projects: Vec<String>,
        scheduler: Arc<Mutex<Scheduler>>,
    ) -> Self {
        WebServer { config, cache, projects, scheduler }
    }

    pub async fn run(self) -> Result<()> {
        #[derive(Serialize)]
        struct SummaryResponse {
            project: String,
            summary: String,
        }
        // Prepare project lists for handlers
        let html_projects = self.projects.clone();
        let json_projects = self.projects.clone();
        let summary_projects = self.projects.clone();
        // Helper to escape HTML special characters
        fn html_escape(s: &str) -> String {
            s.replace('&', "&amp;")
             .replace('<', "&lt;")
             .replace('>', "&gt;")
        }
        // Prepare cache clones for handlers
        let cache_for_html = self.cache.clone();
        let status_cache = self.cache.clone();
        // Clone scheduler for manual trigger API
        let scheduler_for_api = self.scheduler.clone();
        // Build HTTP app with dynamic dashboard content and JSON API
        let app = Router::new()
            // HTML dashboard
            .route(
                "/",
                get(move || {
                    let projects = html_projects.clone();
                    let cache = cache_for_html.clone();
                    async move {
                        // Fetch scheduler status
                        let sched_status = match cache.get("scheduler.status") {
                            Ok(Some(s)) => s,
                            _ => "unknown".to_string(),
                        };
                        let last_start = match cache.get("scheduler.last_run_start") {
                            Ok(Some(s)) => s,
                            _ => "-".to_string(),
                        };
                        let last_end = match cache.get("scheduler.last_run_end") {
                            Ok(Some(s)) => s,
                            _ => "-".to_string(),
                        };
                        let current = match cache.get("scheduler.current_project") {
                            Ok(Some(s)) if !s.is_empty() => s,
                            _ => "-".to_string(),
                        };
                        // Fetch systemd monitor status
                        let sys_status = match cache.get("systemd.status") {
                            Ok(Some(s)) => s,
                            _ => "unknown".to_string(),
                        };
                        let sys_checked = match cache.get("systemd.last_checked") {
                            Ok(Some(s)) => s,
                            _ => "-".to_string(),
                        };
                        // Build HTML with improved styling and content
                        let mut html = String::new();
                        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>PiMainteno Dashboard</title>\n<meta http-equiv=\"refresh\" content=\"300\">\n<style>\n");
                        html.push_str("body { font-family: Arial, sans-serif; background-color: #f5f5f5; color: #333; margin: 0; padding: 0;}\n");
                        html.push_str("header { background-color: #282c34; padding: 1rem; color: white; text-align: center;}\n");
                        html.push_str(".container { max-width: 1200px; margin: 2rem auto; padding: 0 1rem;}\n");
                        html.push_str(".section { background: white; padding: 1.5rem; margin-bottom: 2rem; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1);}\n");
                        html.push_str(".section h2 { margin-top: 0;}\n");
                        html.push_str("pre { background: #f0f0f0; padding: 1rem; border-radius: 4px; overflow-x: auto;}\n");
                        html.push_str("ul { list-style: none; padding-left: 0;}\n");
                        html.push_str("li { margin-bottom: 0.5rem;}\n");
                        html.push_str(".run-button { background-color: #4CAF50; border: none; color: white; padding: 0.5rem 1rem; text-align: center; font-size: 1rem; margin-left: 1rem; cursor: pointer; border-radius: 4px; }\n");
                        html.push_str("</style>\n</head>\n<body>\n<header><h1>PiMainteno Dashboard</h1><button id=\"run-btn\" class=\"run-button\">Run Now</button></header>\n<div class=\"container\">\n");
                        // System Overview
                        html.push_str("<div class=\"section\">\n<h2>System Overview</h2>\n<ul>\n");
                        html.push_str(&format!("<li>Scheduler: {} (last start: {}, last end: {}, current: {})</li>\n", sched_status, last_start, last_end, current));
                        html.push_str(&format!("<li>Systemd Monitor: {} (last checked: {})</li>\n", sys_status, sys_checked));
                        html.push_str("</ul>\n");
                        // Systemd Failures
                        let sys_failures: Vec<String> = match cache.get("systemd.failures") {
                            Ok(Some(f)) => serde_json::from_str(&f).unwrap_or_default(),
                            _ => vec![],
                        };
                        if !sys_failures.is_empty() {
                            html.push_str("<h3>Systemd Failures</h3>\n<ul>\n");
                            for failure in sys_failures {
                                html.push_str(&format!("<li>{}</li>\n", html_escape(&failure)));
                            }
                            html.push_str("</ul>\n");
                        } else {
                            html.push_str("<h3>Systemd Failures</h3>\n<p>None</p>\n");
                        }
                        html.push_str("</div>\n");
                        // Projects
                        for project in projects {
                            html.push_str("<div class=\"section\">\n");
                            html.push_str(&format!("<h2>Project: {}</h2>\n", html_escape(&project)));
                            // Summary
                            let path = std::path::Path::new(&project).join("codex.md");
                            let summary = tokio::fs::read_to_string(path)
                                .await
                                .unwrap_or_else(|_| "Summary not available.".to_string());
                            let escaped_summary = html_escape(&summary);
                            html.push_str("<h3>Summary</h3>\n<pre>");
                            html.push_str(&escaped_summary);
                            html.push_str("</pre>\n");
                            // Patch Suggestion
                            let patch_key = format!("scheduler.patch.{}", project);
                            let patch = cache.get(&patch_key).ok().flatten().unwrap_or_default();
                            if !patch.is_empty() {
                                let escaped_patch = html_escape(&patch);
                                html.push_str("<h3>Recent Patch Suggestion</h3>\n<pre>");
                                html.push_str(&escaped_patch);
                                html.push_str("</pre>\n");
                            } else {
                                html.push_str("<h3>Recent Patch Suggestion</h3>\n<p>None</p>\n");
                            }
                            // Commit History
                            let commit_key = format!("scheduler.commit_log.{}", project);
                            let commit_log: Vec<String> = match cache.get(&commit_key) {
                                Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
                                _ => Vec::new(),
                            };
                            if !commit_log.is_empty() {
                                html.push_str("<h3>Commit History</h3>\n<ul>\n");
                                for msg in commit_log.iter().rev() {
                                    html.push_str(&format!("<li>{}</li>\n", html_escape(msg)));
                                }
                                html.push_str("</ul>\n");
                            } else {
                                html.push_str("<h3>Commit History</h3>\n<p>None</p>\n");
                            }
                            html.push_str("</div>\n");
                        }
                        // Close container
                        html.push_str("</div>\n");
                        // Inject JavaScript for manual run trigger
                        html.push_str("<script>\n");
                        html.push_str("async function triggerRun() {\n");
                        html.push_str("  const res = await fetch('/api/run', { method: 'POST' });\n");
                        html.push_str("  if (res.ok) { alert('Run scheduled'); } else { alert('Run failed'); }\n");
                        html.push_str("}\n");
                        html.push_str("document.getElementById('run-btn').addEventListener('click', triggerRun);\n");
                        html.push_str("</script>\n");
                        // Close body and html
                        html.push_str("</body>\n</html>");
                        Html(html)
                    }
                }),
            )
            // Manual run endpoint
            .route(
                "/api/run",
                post(move || {
                    let scheduler = scheduler_for_api.clone();
                    async move {
                        // Spawn manual scheduler run
                        tokio::spawn(async move {
                            let mut sched = scheduler.lock().await;
                            if let Err(err) = sched.run_once().await {
                                tracing::error!(error = %err, "Manual scheduler run failed");
                            }
                        });
                        // Return accepted status
                        (StatusCode::ACCEPTED, Json(serde_json::json!({"status": "scheduled"})))
                    }
                }),
            )
            // JSON status endpoint
            .route(
                "/api/status",
                get(move || {
                    let cache = status_cache.clone();
                    async move {
                        #[derive(Serialize)]
                        struct SchedulerStatus {
                            status: String,
                            last_run_start: Option<String>,
                            last_run_end: Option<String>,
                            current_project: Option<String>,
                        }
                        #[derive(Serialize)]
                        struct SystemdStatus {
                            status: String,
                            last_checked: Option<String>,
                            failures: Vec<String>,
                        }
                        #[derive(Serialize)]
                        struct StatusResponse {
                            scheduler: SchedulerStatus,
                            systemd: SystemdStatus,
                        }
                        // Gather scheduler info
                        let scheduler = SchedulerStatus {
                            status: cache.get("scheduler.status").ok().flatten().unwrap_or_else(|| "unknown".to_string()),
                            last_run_start: cache.get("scheduler.last_run_start").ok().flatten(),
                            last_run_end: cache.get("scheduler.last_run_end").ok().flatten(),
                            current_project: cache.get("scheduler.current_project").ok().flatten().filter(|s| !s.is_empty()),
                        };
                        // Gather systemd info
                        let failures: Vec<String> = match cache.get("systemd.failures").ok().flatten() {
                            Some(f) => serde_json::from_str(&f).unwrap_or_default(),
                            None => vec![],
                        };
                        let systemd = SystemdStatus {
                            status: cache.get("systemd.status").ok().flatten().unwrap_or_else(|| "unknown".to_string()),
                            last_checked: cache.get("systemd.last_checked").ok().flatten(),
                            failures,
                        };
                        let resp = StatusResponse { scheduler, systemd };
                        Json(resp)
                    }
                }),
            )
            // Existing project endpoints
            .route(
                "/api/projects",
                get(move || {
                    let list = json_projects.clone();
                    async move { Json(list) }
                }),
            )
            .route(
                "/api/projects/:id/summary",
                get(move |Path(id): Path<usize>| {
                    let projects = summary_projects.clone();
                    async move {
                        if id >= projects.len() {
                            Err((StatusCode::NOT_FOUND, "Project not found".to_string()))
                        } else {
                            let project = &projects[id];
                            let path = std::path::Path::new(project).join("codex.md");
                            let summary = tokio::fs::read_to_string(path)
                                .await
                                .unwrap_or_else(|_| "Summary not available.".to_string());
                            let resp = SummaryResponse { project: project.clone(), summary };
                            Ok(Json(resp))
                        }
                    }
                }),
            );
        let addr = SocketAddr::new(self.config.host.parse()?, self.config.port);
        // Attempt to bind to the configured address and handle errors gracefully
        match std::net::TcpListener::bind(addr) {
            Ok(listener) => {
                // Ensure non-blocking for async server
                listener.set_nonblocking(true)?;
                tracing::info!(address = %addr, "Starting WebServer");
                // Use from_tcp to propagate bind errors instead of panicking
                axum::Server::from_tcp(listener)?
                    .serve(app.into_make_service())
                    .await?;
            }
            Err(err) => {
                tracing::error!(error = %err, address = %addr,
                    "Failed to bind WebServer; address may be in use. Disabling Web UI");
            }
        }
        Ok(())
    }
}