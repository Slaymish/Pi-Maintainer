use std::path::{Path, PathBuf};
use std::fs;
use crate::codex_client::CodexClient;
use crate::cache::Cache;
use tracing::info;

pub struct Summarizer<'a> {
    pub project_path: &'a Path,
    pub cache: &'a Cache,
    pub codex: &'a CodexClient,
}

impl<'a> Summarizer<'a> {
    /// Update summary if repo HEAD changed
    pub async fn maybe_update(&self) -> anyhow::Result<()> {
        let head = self.get_head_hash()?;
        if let Some(cached) = self.cache.get_summary(self.project_path, &head)? {
            let codex_md = self.project_path.join("codex.md");
            fs::write(&codex_md, cached)?;
            return Ok(());
        }
        let summary = self.codex.summarize_repository(self.project_path).await?;
        self.cache.set_summary(self.project_path, &head, &summary)?;
        let codex_md = self.project_path.join("codex.md");
        fs::write(&codex_md, summary)?;
        Ok(())
    }

    fn get_head_hash(&self) -> anyhow::Result<String> {
        let head_path = self.project_path.join(".git/HEAD");
        let head = fs::read_to_string(&head_path)?;
        if let Some(branch_ref) = head.strip_prefix("ref: ") {
            let ref_path = self.project_path.join(".git").join(branch_ref.trim());
            Ok(fs::read_to_string(ref_path)?.trim().to_string())
        } else {
            Ok(head.trim().to_string())
        }
    }
}
