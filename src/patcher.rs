impl PatchGenerator {
    pub async fn generate_patch(&self, project_path: &Path) -> Result<Option<String>, PatchError> {
        let head_hash = git::get_head_hash(project_path).await?;
        if let Some(cached_patch) = self.cache.get_patch(project_path, &head_hash)? {
            return Ok(Some(cached_patch));
        }

        let prompt = self.prompt_factory.create_patch_prompt(project_path).await?;
        let llm_output = self.llm_client.query_llm(&prompt).await?;

        let patch = llm_output.trim();
        if !patch.is_empty() {
            self.cache
                .insert_patch(project_path, &head_hash, patch.to_string())?;
            Ok(Some(patch.to_string()))
        } else {
            Ok(None)
        }
    }
}
