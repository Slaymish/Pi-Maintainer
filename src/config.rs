         if let Some(home) = std::env::var("HOME").ok() {
             if let Some(s) = cfg.cache.path.to_str() {
                 if let Some(stripped) = s.strip_prefix("~/") {
                     cfg.cache.path = PathBuf::from(&home).join(stripped);
                 }
             }
             for project in &mut cfg.scheduler.projects {
                 if let Some(stripped) = project.strip_prefix("~/") {
                     *project = PathBuf::from(&home)
                         .join(stripped)
                         .to_string_lossy()
                         .into_owned();
                 }
             }
         }
@@
 // Default values for web configuration
 impl WebConfig {
     /// Default directory for static frontend assets
     fn default_static_dir() -> PathBuf {
         PathBuf::from("frontend/dist")
     }
 }
