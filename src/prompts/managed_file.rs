use crate::managed_block::{
    LLMAN_PROMPTS_MARKER_END, LLMAN_PROMPTS_MARKER_START, has_llman_prompt_markers,
    update_file_with_markers,
};
use crate::path_utils::safe_parent_for_creation;
use crate::prompts::confirm::confirm_inject_twice;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub fn write_llman_managed_block(
    path: &Path,
    body: &str,
    force: bool,
    interactive: bool,
) -> Result<bool> {
    if let Some(parent) = safe_parent_for_creation(path) {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let existing = fs::read_to_string(path).map_err(|e| {
            anyhow!(t!(
                "errors.file_read_failed",
                path = path.display(),
                error = e
            ))
        })?;

        let needs_confirm = !existing.trim().is_empty() && !has_llman_prompt_markers(&existing);
        if needs_confirm && !force && !confirm_inject_twice(path, interactive)? {
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(false);
        }
    }

    update_file_with_markers(
        path,
        body.trim_end(),
        LLMAN_PROMPTS_MARKER_START,
        LLMAN_PROMPTS_MARKER_END,
    )?;

    Ok(true)
}
