use anyhow::{Result, anyhow};
use inquire::Confirm;
use std::path::Path;

pub fn confirm_overwrite(path: &Path, interactive: bool) -> Result<bool> {
    if !interactive {
        return Err(anyhow!(t!(
            "errors.non_interactive_overwrite_requires_force",
            path = path.display()
        )));
    }
    let overwrite = Confirm::new(&t!("messages.file_exists_overwrite", path = path.display()))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;
    Ok(overwrite)
}

fn confirm_inject(path: &Path, interactive: bool) -> Result<bool> {
    if !interactive {
        return Err(anyhow!(t!(
            "errors.non_interactive_inject_requires_force",
            path = path.display()
        )));
    }
    let confirmed = Confirm::new(&t!("messages.file_exists_inject", path = path.display()))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;
    Ok(confirmed)
}

pub fn confirm_inject_twice(path: &Path, interactive: bool) -> Result<bool> {
    let first = confirm_inject(path, interactive)?;
    if !first {
        return Ok(false);
    }

    let second = Confirm::new(&t!(
        "messages.file_exists_inject_second",
        path = path.display()
    ))
    .with_default(false)
    .prompt()
    .map_err(|e| anyhow!(t!("errors.interactive_prompt_error", error = e)))?;

    Ok(second)
}
