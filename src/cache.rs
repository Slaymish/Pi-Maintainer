     pub async fn new(path: &Path) -> Result<Self> {
         let path_buf: PathBuf = path.to_path_buf();
         // Open sled database in a blocking task to avoid blocking the async runtime
         let db = task::spawn_blocking(move || sled::open(&path_buf)).await??;
         Ok(DataCache { db: Arc::new(db) })
     }
