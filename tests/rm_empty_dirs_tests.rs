use llman::tool::command::RmUselessDirsArgs;
use llman::tool::rm_empty_dirs::run;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
mod env_lock;

#[test]
fn test_rm_useless_dirs_dry_run_and_live() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("a/b")).expect("Failed to create a/b");
    fs::create_dir_all(root.join("c/d")).expect("Failed to create c/d");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");

    let dry_run_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: false,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    run(&dry_run_args).expect("Dry run failed");

    assert!(root.join("a/b").exists());
    assert!(root.join("c/d").exists());

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(!root.join("a").exists());
    assert!(!root.join("c").exists());
    assert!(root.join("keep.txt").exists());
    assert!(root.exists());
}

#[test]
fn test_rm_useless_dirs_respects_gitignore() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("ignored_dir/inner")).expect("Failed to create ignored_dir/inner");
    fs::create_dir_all(root.join("remove_me/inner")).expect("Failed to create remove_me/inner");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");
    fs::write(root.join(".gitignore"), "ignored_dir/\n").expect("Failed to create .gitignore");

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: Some(root.join(".gitignore")),
        prune_ignored: false,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(root.join("ignored_dir").exists());
    assert!(!root.join("remove_me").exists());
    assert!(root.join("keep.txt").exists());
}

#[test]
fn test_rm_useless_dirs_prune_ignored() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("__pycache__")).expect("Failed to create __pycache__");
    fs::write(root.join("__pycache__/a.pyc"), "pyc").expect("Failed to create pyc");
    fs::create_dir_all(root.join("logs")).expect("Failed to create logs");
    fs::write(root.join("logs/app.log"), "log").expect("Failed to create log");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");
    fs::write(root.join(".gitignore"), "__pycache__/\n*.log\n")
        .expect("Failed to create .gitignore");

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: Some(root.join(".gitignore")),
        prune_ignored: true,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(!root.join("__pycache__").exists());
    assert!(!root.join("logs").exists());
    assert!(root.join("keep.txt").exists());
}

#[test]
fn test_rm_useless_dirs_protects_node_modules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("node_modules/pkg")).expect("Failed to create node_modules");
    fs::write(root.join("node_modules/pkg/index.js"), "console.log('x');")
        .expect("Failed to create file");
    fs::create_dir_all(root.join(".cargo")).expect("Failed to create .cargo");
    fs::write(root.join(".cargo/config.toml"), "[build]\n")
        .expect("Failed to create .cargo config");
    fs::create_dir_all(root.join(".npm")).expect("Failed to create .npm");
    fs::write(root.join(".npm/_cache"), "cache").expect("Failed to create .npm file");
    fs::write(root.join(".gitignore"), "node_modules/\n").expect("Failed to create .gitignore");

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: Some(root.join(".gitignore")),
        prune_ignored: true,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(root.join("node_modules").exists());
    assert!(root.join(".cargo").exists());
    assert!(root.join(".npm").exists());
}

#[test]
fn test_rm_useless_dirs_removes_python_caches() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".pytest_cache")).expect("Failed to create .pytest_cache");
    fs::write(root.join(".pytest_cache/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".mypy_cache")).expect("Failed to create .mypy_cache");
    fs::write(root.join(".mypy_cache/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".ruff_cache")).expect("Failed to create .ruff_cache");
    fs::write(root.join(".ruff_cache/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".basedpyright")).expect("Failed to create .basedpyright");
    fs::write(root.join(".basedpyright/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".pytype")).expect("Failed to create .pytype");
    fs::write(root.join(".pytype/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".pyre")).expect("Failed to create .pyre");
    fs::write(root.join(".pyre/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".ty")).expect("Failed to create .ty");
    fs::write(root.join(".ty/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".ty_cache")).expect("Failed to create .ty_cache");
    fs::write(root.join(".ty_cache/data"), "data").expect("Failed to create data");
    fs::create_dir_all(root.join(".ty-cache")).expect("Failed to create .ty-cache");
    fs::write(root.join(".ty-cache/data"), "data").expect("Failed to create data");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(!root.join(".pytest_cache").exists());
    assert!(!root.join(".mypy_cache").exists());
    assert!(!root.join(".ruff_cache").exists());
    assert!(!root.join(".basedpyright").exists());
    assert!(!root.join(".pytype").exists());
    assert!(!root.join(".pyre").exists());
    assert!(!root.join(".ty").exists());
    assert!(!root.join(".ty_cache").exists());
    assert!(!root.join(".ty-cache").exists());
    assert!(root.join("keep.txt").exists());
}

#[test]
fn test_rm_useless_dirs_config_override_disables_protection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("node_modules/pkg")).expect("Failed to create node_modules");
    fs::write(root.join("node_modules/pkg/index.js"), "console.log('x');")
        .expect("Failed to create file");
    fs::write(root.join(".gitignore"), "node_modules/\n").expect("Failed to create .gitignore");

    let config_path = root.join("config.yaml");
    fs::write(
        &config_path,
        r#"version: "0.1"
tools:
  rm-useless-dirs:
    protected:
      mode: override
      names: []
"#,
    )
    .expect("Failed to create config");

    let live_args = RmUselessDirsArgs {
        config: Some(config_path),
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: Some(root.join(".gitignore")),
        prune_ignored: true,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(!root.join("node_modules").exists());
}

#[test]
fn test_rm_useless_dirs_config_extend_useless_list() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("custom-cache")).expect("Failed to create custom-cache");
    fs::write(root.join("custom-cache/data"), "data").expect("Failed to create data");

    let config_path = root.join("config.yaml");
    fs::write(
        &config_path,
        r#"version: "0.1"
tools:
  rm-useless-dirs:
    useless:
      mode: extend
      names: ["custom-cache"]
"#,
    )
    .expect("Failed to create config");

    let live_args = RmUselessDirsArgs {
        config: Some(config_path),
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(!root.join("custom-cache").exists());
}

#[test]
fn test_rm_useless_dirs_rejects_legacy_config_key() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let config_path = root.join("config.yaml");
    fs::write(
        &config_path,
        r#"version: "0.1"
tools:
  rm-empty-dirs:
    useless:
      mode: extend
      names: ["legacy"]
"#,
    )
    .expect("Failed to create config");

    let live_args = RmUselessDirsArgs {
        config: Some(config_path),
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    let err = run(&live_args).expect_err("Expected legacy config to fail");
    assert!(err.to_string().contains("rm-empty-dirs"));
}

#[test]
fn test_rm_useless_dirs_default_gitignore_is_relative_to_target() {
    let _guard = env_lock::lock_env();
    struct CwdGuard {
        original: PathBuf,
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    let original = std::env::current_dir().expect("cwd");
    let _guard = CwdGuard { original };

    let cwd = TempDir::new().expect("Failed to create temp cwd");
    std::env::set_current_dir(cwd.path()).expect("set cwd");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("ignored_dir/inner")).expect("Failed to create ignored_dir/inner");
    fs::create_dir_all(root.join("remove_me/inner")).expect("Failed to create remove_me/inner");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");
    fs::write(root.join(".gitignore"), "ignored_dir/\n").expect("Failed to create .gitignore");

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(root.join("ignored_dir").exists());
    assert!(!root.join("remove_me").exists());
    assert!(root.join("keep.txt").exists());
}

#[test]
fn test_rm_useless_dirs_skips_targets_in_protected_subtree() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let protected_root = root.join("some/.git/objects");
    fs::create_dir_all(protected_root.join("pack")).expect("Failed to create pack dir");

    let live_args = RmUselessDirsArgs {
        config: None,
        path: Some(protected_root),
        yes: true,
        gitignore: None,
        prune_ignored: false,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(root.join("some/.git/objects/pack").exists());
}
