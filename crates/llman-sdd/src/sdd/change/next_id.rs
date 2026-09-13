//! `llman sdd change next-id` (r29): read-only preview of the next free
//! change id number. Prints the whole-tree max and the next unused number —
//! the same value `change new --from` injects as `llman_sdd_unique_id`.
//! Creates nothing and writes nothing.

use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::change_id::{harvest_unique_numbers, unique_group_regex};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::json::print_json;
use anyhow::Result;
use std::path::Path;

pub(crate) fn run(root: &Path, json: bool) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    let pattern_re = unique_group_regex(
        config
            .change_id
            .as_ref()
            .and_then(|change_id| change_id.pattern.as_deref()),
    )?;
    let harvest = harvest_unique_numbers(&llmanspec_dir, pattern_re.as_ref())?;

    if json {
        print_json(
            &serde_json::json!({
                "maxNumber": harvest.max_number,
                "nextNumber": harvest.next_number,
                "warnings": harvest.warnings,
            }),
            false,
        )?;
        return Ok(());
    }

    match harvest.max_number {
        Some(max) => println!("max number in tree: {max}"),
        None => println!("no numbered change dirs found in tree"),
    }
    println!("next free number: {}", harvest.next_number);
    for warning in &harvest.warnings {
        eprintln!("warning: {warning}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::project::init;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn prints_whole_tree_max_without_creating_anything() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        init::run(root, None, false).expect("sdd init");
        fs::create_dir_all(root.join("llmanspec/delayed-changes/tools/c2620-tool-x")).unwrap();
        fs::create_dir_all(root.join("llmanspec/changes/c10-active")).unwrap();

        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        let config = load_required_config(&llmanspec_dir).unwrap();
        let pattern_re = unique_group_regex(
            config
                .change_id
                .as_ref()
                .and_then(|change_id| change_id.pattern.as_deref()),
        )
        .unwrap();
        let harvest = harvest_unique_numbers(&llmanspec_dir, pattern_re.as_ref()).unwrap();
        assert_eq!(harvest.max_number, Some(2620));
        assert_eq!(harvest.next_number, 2621);
        assert_eq!(harvest.warnings, Vec::<String>::new());

        // Only the fixture dirs exist; run() only reads.
        assert!(llmanspec_dir.join("changes/c10-active").exists());
        assert!(!llmanspec_dir.join("changes/c2621-anything").exists());
    }
}
