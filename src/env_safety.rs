use anyhow::{Result, bail};

pub fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }

    true
}

pub fn is_dangerous_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    matches!(upper.as_str(), "LD_PRELOAD" | "LD_LIBRARY_PATH" | "PATH")
        || upper.starts_with("DYLD_")
}

pub fn find_dangerous_env_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut dangerous: Vec<&str> = keys
        .into_iter()
        .filter(|key| is_dangerous_env_key(key))
        .collect();
    dangerous.sort_unstable();
    dangerous.dedup();
    dangerous
}

pub fn reject_dangerous_env_keys<'a, I>(keys: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let dangerous = find_dangerous_env_keys(keys);
    if dangerous.is_empty() {
        return Ok(());
    }
    bail!(
        "Refused dangerous environment variable key(s): {}",
        dangerous.join(", ")
    )
}

pub fn validate_user_git_ref(reference: &str) -> Result<(), String> {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return Err("git ref must not be empty".to_string());
    }
    if trimmed.starts_with('-') {
        return Err(format!(
            "git ref must not start with '-': {trimmed} (refuses option injection)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_keys_are_case_insensitive() {
        assert!(is_dangerous_env_key("PATH"));
        assert!(is_dangerous_env_key("path"));
        assert!(is_dangerous_env_key("Ld_Preload"));
        assert!(is_dangerous_env_key("DYLD_LIBRARY_PATH"));
        assert!(is_dangerous_env_key("dyld_insert_libraries"));
        assert!(!is_dangerous_env_key("API_KEY"));
        assert!(!is_dangerous_env_key("MY_PATH"));
    }

    #[test]
    fn reject_dangerous_env_keys_lists_hits() {
        let err = reject_dangerous_env_keys(["FOO", "LD_PRELOAD", "PATH"]).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("LD_PRELOAD"));
        assert!(msg.contains("PATH"));
        assert!(!msg.contains("FOO"));
    }

    #[test]
    fn validate_user_git_ref_rejects_option_like() {
        assert!(validate_user_git_ref("origin/main").is_ok());
        assert!(validate_user_git_ref("-c").is_err());
        assert!(validate_user_git_ref("--all").is_err());
        assert!(validate_user_git_ref("").is_err());
        assert!(validate_user_git_ref("   ").is_err());
    }
}
