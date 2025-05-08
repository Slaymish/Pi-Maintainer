 async fn run_handler(State(scheduler): State<Arc<Scheduler>>) -> Json<serde_json::Value> {
     match scheduler.manual_run().await {
         Ok(_) => Json(json!({ "status": "ok" })),
         Err(e) => Json(json!({ "status": "error", "error": e.to_string() })),
     }
 }
