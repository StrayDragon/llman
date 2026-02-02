use llman::skills::{
    ConfigEntry, Registry, SkillCandidate, SkillsConfig, TargetMode, apply_target_links,
};
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn test_link_target_points_to_skill_dir() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let skills_root = root.join("skills");
    let skill_dir = skills_root.join("example");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: Example Skill\n---\n",
    )
    .expect("write SKILL.md");
    let target_root = root.join("targets");
    fs::create_dir_all(&target_root).expect("target root");

    let skill = SkillCandidate {
        skill_id: "example-skill".to_string(),
        skill_dir: skill_dir.clone(),
    };
    let config = SkillsConfig {
        targets: vec![ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Link,
        }],
    };
    let mut registry = Registry::default();
    registry.ensure_skill(&skill.skill_id);
    let entry = registry.skills.get(&skill.skill_id).expect("entry");

    apply_target_links(&skill, &config, entry, false, None).expect("apply links");

    let link_path = target_root.join("example-skill");
    let meta = fs::symlink_metadata(&link_path).expect("metadata");
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(&link_path).expect("read link");
    assert_eq!(target, skill_dir);
}
