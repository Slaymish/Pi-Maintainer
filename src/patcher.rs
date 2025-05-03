use crate::codex_client::CodexClient;
use crate::cache::DataCache;
use anyhow::Result;

#[derive(Clone)]
pub struct PatchGenerator {
    codex: CodexClient,
    cache: DataCache,
}

#[derive(Clone)]
pub struct PatchApplier {
    cache: DataCache,
}

impl PatchGenerator {
    pub fn new(codex: CodexClient, cache: DataCache) -> Self {
        PatchGenerator { codex, cache }
    }

    pub async fn generate(&self, context: &str) -> Result<String> {
        tracing::info!(context = context, "Generating patch");
        let patch = self.codex.generate_patch(context).await?;
        // Cache the patch suggestion for display in the web UI
        let key = format!("scheduler.patch.{}", context);
        // Insert patch into cache; patch may be large, storing full text
        if let Err(err) = self.cache.insert(&key, &patch) {
            tracing::warn!(error = %err, "Failed to cache patch suggestion for project {}", context);
        }
        Ok(patch)
    }
}

impl PatchApplier {
    pub fn new(cache: DataCache) -> Self {
        PatchApplier { cache }
    }

    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("PatchApplier shutting down");
        Ok(())
    }

    pub fn apply(&self, patch: &str) -> Result<()> {
        tracing::info!(patch = patch, "Applying patch");
        // Placeholder: apply the patch to the repository
        Ok(())
    }
}