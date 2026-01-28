use llman::skills::{
    ConfigEntry, ConflictOption, ConflictResolver, Registry, SkillsConfig, SkillsPaths, TargetMode,
    sync_sources,
};
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct DefaultResolver;

impl ConflictResolver for DefaultResolver {
    fn resolve(
        &mut self,
        _skill_id: &str,
        _options: &[ConflictOption],
        default_index: usize,
    ) -> anyhow::Result<usize> {
        Ok(default_index)
    }
}

#[cfg(unix)]
#[test]
fn test_sync_imports_and_keeps_source() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_root = root.join("config");
    fs::create_dir_all(&config_root).expect("config root");
    let home_root = root.join("home");
    fs::create_dir_all(&home_root).expect("home root");
    unsafe {
        std::env::set_var("LLMAN_CONFIG_DIR", &config_root);
        std::env::set_var("HOME", &home_root);
        std::env::set_var("CLAUDE_HOME", home_root.join("claude"));
        std::env::set_var("CODEX_HOME", home_root.join("codex"));
    }

    let source_root = root.join("source");
    let skill_dir = source_root.join("example");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: Example Skill\n---\n",
    )
    .expect("write SKILL.md");

    let paths = SkillsPaths::resolve().expect("paths");
    let config = SkillsConfig {
        sources: vec![ConfigEntry {
            id: "source".to_string(),
            agent: "agent".to_string(),
            scope: "user".to_string(),
            path: source_root.clone(),
            enabled: true,
            mode: TargetMode::Link,
        }],
        targets: Vec::new(),
        repo_root: None,
    };

    let mut registry = Registry::default();
    let mut resolver = DefaultResolver;
    sync_sources(&config, &paths, &mut registry, &mut resolver).expect("sync");

    let entry = registry
        .skills
        .get("example-skill")
        .expect("registry entry");
    let hash = entry.current_hash.as_ref().expect("current hash");
    let version_dir = paths
        .store_dir
        .join("example-skill")
        .join("versions")
        .join(hash);
    assert!(version_dir.exists());

    let source_link = source_root.join("example");
    let meta = fs::symlink_metadata(&source_link).expect("metadata");
    assert!(!meta.file_type().is_symlink());

    unsafe {
        std::env::remove_var("LLMAN_CONFIG_DIR");
        std::env::remove_var("HOME");
        std::env::remove_var("CLAUDE_HOME");
        std::env::remove_var("CODEX_HOME");
    }
}
