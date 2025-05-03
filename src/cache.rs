use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct DataCache {
    db: Arc<sled::Db>,
}

impl DataCache {
    pub async fn new(path: &Path) -> Result<Self> {
        // Synchronous open; consider spawn_blocking for heavy I/O
        let db = sled::open(path)?;
        Ok(DataCache { db: Arc::new(db) })
    }
    /// Insert a string value for a given key into the cache.
    pub fn insert(&self, key: &str, value: &str) -> Result<()> {
        self.db.insert(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    /// Retrieve a string value for a given key from the cache.
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        if let Some(iv) = self.db.get(key.as_bytes())? {
            let s = String::from_utf8(iv.to_vec())?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Flush pending writes to disk.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}