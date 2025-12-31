use llman::tool::command::RmEmptyDirsArgs;
use llman::tool::rm_empty_dirs::run;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_rm_empty_dirs_dry_run_and_live() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("a/b")).expect("Failed to create a/b");
    fs::create_dir_all(root.join("c/d")).expect("Failed to create c/d");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");

    let dry_run_args = RmEmptyDirsArgs {
        path: Some(root.to_path_buf()),
        yes: false,
        gitignore: None,
        verbose: false,
    };

    run(&dry_run_args).expect("Dry run failed");

    assert!(root.join("a/b").exists());
    assert!(root.join("c/d").exists());

    let live_args = RmEmptyDirsArgs {
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: None,
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(!root.join("a").exists());
    assert!(!root.join("c").exists());
    assert!(root.join("keep.txt").exists());
    assert!(root.exists());
}

#[test]
fn test_rm_empty_dirs_respects_gitignore() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("ignored_dir/inner")).expect("Failed to create ignored_dir/inner");
    fs::create_dir_all(root.join("remove_me/inner")).expect("Failed to create remove_me/inner");
    fs::write(root.join("keep.txt"), "keep").expect("Failed to create keep.txt");
    fs::write(root.join(".gitignore"), "ignored_dir/\n").expect("Failed to create .gitignore");

    let live_args = RmEmptyDirsArgs {
        path: Some(root.to_path_buf()),
        yes: true,
        gitignore: Some(root.join(".gitignore")),
        verbose: false,
    };

    run(&live_args).expect("Live run failed");

    assert!(root.join("ignored_dir").exists());
    assert!(!root.join("remove_me").exists());
    assert!(root.join("keep.txt").exists());
}
