async fn api_run(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let scheduler = app_state.scheduler.clone();
    tokio::spawn(async move {
        if let Err(err) = scheduler.run_once().await {
            tracing::error!("API-triggered run failed: {}", err);
        }
    });
    (StatusCode::ACCEPTED, "Run triggered".to_owned())
}
