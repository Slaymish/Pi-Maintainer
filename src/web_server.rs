use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use crate::scheduler::Scheduler;
use crate::config::WebConfig;

pub async fn start_web_server(
    scheduler: Arc<Scheduler>,
    config: WebConfig,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/run", post(run_handler))
        .with_state(Arc::clone(&scheduler));

    let addr = format!("{}:{}", config.host, config.port).parse()?;
    tracing::info!("Starting web server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn status_handler(State(scheduler): State<Arc<Scheduler>>) -> Json<serde_json::Value> {
    let projects = scheduler.list_projects().await;
    Json(json!({
        "status": "ok",
        "projects": projects,
    }))
}

async fn run_handler(State(scheduler): State<Arc<Scheduler>>) -> Json<serde_json::Value> {
    match scheduler.manual_run().await {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}
