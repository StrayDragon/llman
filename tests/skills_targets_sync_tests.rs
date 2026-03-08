use llman::skills::catalog::types::{ConfigEntry, SkillCandidate, TargetMode};
use llman::skills::targets::sync::apply_target_diff;
use std::collections::HashSet;
use std::fs;
use tempfile::Builder;

#[cfg(unix)]
#[test]
fn test_tmp_project_is_cleaned_up() {
    let tmp_path = {
        let temp = Builder::new()
            .prefix("llman-sync-it-")
            .tempdir_in("/tmp")
            .expect("temp dir in /tmp");
        let tmp_path = temp.path().to_path_buf();
        fs::write(tmp_path.join("touched.txt"), "ok").expect("write file");
        tmp_path
    };

    assert!(
        !tmp_path.exists(),
        "expected temp project to be cleaned up: {}",
        tmp_path.display()
    );
}

#[cfg(unix)]
#[test]
fn test_copy_mode_does_not_write_vendored_metadata_file() {
    let temp = Builder::new()
        .prefix("llman-sync-it-")
        .tempdir_in("/tmp")
        .expect("temp dir in /tmp");

    let skills_root = temp.path().join("skills");
    let skill_dir = skills_root.join("skill");
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

    let target_root = temp.path().join("targets");
    let skill = SkillCandidate {
        skill_id: "skill".to_string(),
        skill_dir: skill_dir.clone(),
    };
    let target = ConfigEntry {
        id: "codex_repo".to_string(),
        agent: "codex".to_string(),
        scope: "repo".to_string(),
        path: target_root.clone(),
        enabled: true,
        mode: TargetMode::Copy,
    };

    let mut desired = HashSet::new();
    desired.insert("skill".to_string());
    apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
        .expect("apply diff");

    let target_metadata_path = target_root.join(".llman-vendored.json");
    assert!(
        !target_metadata_path.exists(),
        "vendored metadata file should not be created"
    );

    let entry_path = target_root.join("skill");
    assert!(entry_path.join("SKILL.md").exists());
    assert!(
        !entry_path.join(".llman-vendored.json").exists(),
        "per-skill vendored metadata file should not be created"
    );
}
