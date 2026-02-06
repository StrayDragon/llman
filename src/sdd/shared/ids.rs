use anyhow::{Result, anyhow};

pub fn validate_sdd_id(id: &str, kind: &'static str) -> Result<()> {
    let trimmed = id.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return Err(anyhow!(t!("sdd.shared.invalid_id", kind = kind, id = id)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_like_ids() {
        assert!(validate_sdd_id("../oops", "change").is_err());
        assert!(validate_sdd_id("a/b", "change").is_err());
        assert!(validate_sdd_id(r"a\\b", "change").is_err());
        assert!(validate_sdd_id(".", "change").is_err());
        assert!(validate_sdd_id("..", "change").is_err());
        assert!(validate_sdd_id("   ", "change").is_err());
    }

    #[test]
    fn accepts_simple_identifier() {
        assert!(validate_sdd_id("fix-sdd-command-safety-and-flags", "change").is_ok());
        assert!(validate_sdd_id("sdd-workflow", "spec").is_ok());
    }
}
