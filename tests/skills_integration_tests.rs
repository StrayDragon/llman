#![cfg(unix)]

mod common;

use common::{assert_success, run_llman};
use llman::skills::catalog::types::{ConfigEntry, SkillCandidate, SkillsConfig, TargetMode};
use llman::skills::targets::sync::apply_target_links;
use std::collections::HashMap;
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
    let desired_by_target = HashMap::new();

    apply_target_links(&skill, &config, &desired_by_target, false, None).expect("apply links");

    let link_path = target_root.join("example-skill");
    let meta = fs::symlink_metadata(&link_path).expect("metadata");
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(&link_path).expect("read link");
    assert_eq!(target, skill_dir);
}

#[cfg(unix)]
#[test]
fn test_skills_cli_non_interactive_links_and_realtime_state() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let skills_root = work_dir.join("skills-root");
    let skill_dir = skills_root.join("example");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# example skill").expect("write SKILL.md");

    let target_root = work_dir.join("targets");
    fs::create_dir_all(&target_root).expect("target root");

    fs::create_dir_all(&skills_root).expect("skills root");
    let config = format!(
        r#"version = 2

[[target]]
id = "claude_user"
agent = "claude"
scope = "user"
path = "{}"
mode = "link"
enabled = true
"#,
        target_root.display()
    );
    fs::write(skills_root.join("config.toml"), config).expect("write config");

    let output = run_llman(
        &["skills", "--skills-dir", skills_root.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&output);

    let link_path = target_root.join("example");
    let meta = fs::symlink_metadata(&link_path).expect("metadata");
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(&link_path).expect("read link");
    assert_eq!(target, skill_dir);

    let registry_path = skills_root.join("registry.json");
    assert!(!registry_path.exists());
}

#[cfg(unix)]
#[test]
fn test_skills_cli_warns_when_legacy_registry_exists() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let skills_root = work_dir.join("skills-root");
    let skill_dir = skills_root.join("example");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# example skill").expect("write SKILL.md");

    let target_root = work_dir.join("targets");
    fs::create_dir_all(&target_root).expect("target root");

    fs::create_dir_all(&skills_root).expect("skills root");
    let config = format!(
        r#"version = 2

[[target]]
id = "claude_user"
agent = "claude"
scope = "user"
path = "{}"
mode = "link"
enabled = true
"#,
        target_root.display()
    );
    fs::write(skills_root.join("config.toml"), config).expect("write config");
    fs::write(skills_root.join("registry.json"), "{}").expect("write legacy registry");

    let output = run_llman(
        &["skills", "--skills-dir", skills_root.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&output);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Legacy skills registry detected at")
            && stderr.contains("it is ignored in realtime mode"),
        "stderr did not contain legacy registry warning:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_skills_cli_non_interactive_supports_project_target_id_without_registry() {
    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let skills_root = work_dir.join("skills-root");
    let skill_dir = skills_root.join("example");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# example skill").expect("write SKILL.md");

    let target_root = work_dir.join("targets");
    fs::create_dir_all(&target_root).expect("target root");

    fs::create_dir_all(&skills_root).expect("skills root");
    let config = format!(
        r#"version = 2

[[target]]
id = "claude_project"
agent = "claude"
scope = "project"
path = "{}"
mode = "link"
enabled = true
"#,
        target_root.display()
    );
    fs::write(skills_root.join("config.toml"), config).expect("write config");

    let output = run_llman(
        &["skills", "--skills-dir", skills_root.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&output);

    let link_path = target_root.join("example");
    let meta = fs::symlink_metadata(&link_path).expect("metadata");
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(&link_path).expect("read link");
    assert!(!target.is_absolute(), "expected compact relative symlink");
    let expected =
        llman::path_utils::relative_path_from_dir(&target_root, &skill_dir).expect("relative path");
    assert_eq!(target, expected);

    let registry_path = skills_root.join("registry.json");
    assert!(!registry_path.exists());
}

#[cfg(unix)]
#[test]
fn test_skills_cli_non_interactive_keeps_existing_link_when_enabled_false() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().expect("temp dir");
    let work_dir = temp.path();
    let skills_root = work_dir.join("skills-root");
    let skill_dir = skills_root.join("example");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# example skill").expect("write SKILL.md");

    let target_root = work_dir.join("targets");
    fs::create_dir_all(&target_root).expect("target root");
    let link_path = target_root.join("example");
    unix_fs::symlink(&skill_dir, &link_path).expect("pre-link skill");

    fs::create_dir_all(&skills_root).expect("skills root");
    let config = format!(
        r#"version = 2

[[target]]
id = "claude_user"
agent = "claude"
scope = "user"
path = "{}"
mode = "link"
enabled = false
"#,
        target_root.display()
    );
    fs::write(skills_root.join("config.toml"), config).expect("write config");

    let output = run_llman(
        &["skills", "--skills-dir", skills_root.to_str().unwrap()],
        work_dir,
        work_dir,
    );
    assert_success(&output);

    let meta = fs::symlink_metadata(&link_path).expect("metadata");
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(&link_path).expect("read link");
    assert_eq!(target, skill_dir);

    let registry_path = skills_root.join("registry.json");
    assert!(!registry_path.exists());
}
