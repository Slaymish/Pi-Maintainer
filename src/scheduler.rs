pub struct Scheduler {
    // ... fields ...
}

impl Scheduler {
    pub async fn run_once(&self) -> Result<(), SchedulerError> {
        for project_path in &self.projects {
            if let Err(err) = self.run_project_maintenance(project_path).await {
                tracing::error!(
                    "Error maintaining project {}: {}",
                    project_path.display(),
                    err
                );
            }
        }
        Ok(())
    }
    // ... other methods ...
}
