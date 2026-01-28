use anyhow::{Result, anyhow};
use inquire::Confirm;
use std::io::IsTerminal;

pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}

pub fn confirm_non_repo(interactive: bool) -> Result<bool> {
    if !interactive {
        return Err(anyhow!(t!("skills.non_repo_non_interactive")));
    }
    let prompt = t!("skills.non_repo_confirm");
    let confirmed = Confirm::new(&prompt)
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!(t!("skills.non_repo_prompt_failed", error = e)))?;
    Ok(confirmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_non_repo_non_interactive() {
        let result = confirm_non_repo(false);
        assert!(result.is_err());
    }
}
