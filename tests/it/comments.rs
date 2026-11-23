//! Integration tests for `llman tool comments`.

use crate::common::*;
use std::fs;

fn run_overview(env: &TestEnvironment, targets: &[&str]) -> std::process::Output {
    let mut args = vec!["tool", "comments", "overview"];
    args.extend_from_slice(targets);
    run_llman(&args, env.path(), env.path())
}

#[test]
fn overview_empty_dir_succeeds_without_sibling_headers() {
    let env = TestEnvironment::new();
    env.create_file("code.py", "# top comment\nprint(1)\n");
    fs::create_dir_all(env.path().join("empty-junk")).expect("mkdir");

    let output = run_overview(&env, &["empty-junk"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("code.py"),
        "empty dir must not pick sibling files: {stdout}"
    );
}

#[test]
fn overview_glob_zero_match_fails() {
    let env = TestEnvironment::new();
    let output = run_overview(&env, &["no-such-glob-xyzzy-*"]);
    assert!(
        !output.status.success(),
        "expected failure, got stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error"),
        "stderr should contain Error: {stderr}"
    );
}

#[test]
fn overview_dir_honors_gitignore_but_explicit_file_does_not() {
    let env = TestEnvironment::new();
    env.create_file("keep.py", "# keep me\nprint(1)\n");
    env.create_file("skip.py", "# skip me\nprint(1)\n");
    fs::write(env.path().join(".gitignore"), "skip.py\n").expect("gitignore");

    let dir_out = run_overview(&env, &["."]);
    assert_success(&dir_out);
    let dir_stdout = String::from_utf8_lossy(&dir_out.stdout);
    assert!(dir_stdout.contains("keep.py"), "{dir_stdout}");
    assert!(
        !dir_stdout.contains("skip.py"),
        "gitignore should hide skip.py: {dir_stdout}"
    );

    let file_out = run_overview(&env, &["skip.py"]);
    assert_success(&file_out);
    let file_stdout = String::from_utf8_lossy(&file_out.stdout);
    assert!(file_stdout.contains("skip.py"), "{file_stdout}");
}

#[test]
fn overview_brace_glob_selects_extensions() {
    let env = TestEnvironment::new();
    env.create_file("a.py", "# py header\nprint(1)\n");
    env.create_file("a.sh", "# sh header\necho 1\n");
    env.create_file("a.txt", "# txt header\nok\n");

    let output = run_overview(&env, &["*.{py,sh}"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a.py"), "{stdout}");
    assert!(stdout.contains("a.sh"), "{stdout}");
    assert!(!stdout.contains("a.txt"), "{stdout}");
}

#[test]
fn comments_clean_useless_help_and_hidden_alias() {
    let env = TestEnvironment::new();
    let nested = run_llman(
        &["tool", "comments", "clean-useless", "--help"],
        env.path(),
        env.path(),
    );
    assert_success(&nested);
    let nested_out = String::from_utf8_lossy(&nested.stdout);
    assert!(nested_out.contains("clean-useless"), "{nested_out}");

    let alias = run_llman(
        &["tool", "clean-useless-comments", "--help"],
        env.path(),
        env.path(),
    );
    assert_success(&alias);

    let tool_help = run_llman(&["tool", "--help"], env.path(), env.path());
    assert_success(&tool_help);
    let help = String::from_utf8_lossy(&tool_help.stdout);
    assert!(help.contains("comments"), "{help}");
    assert!(
        !help.contains("clean-useless-comments"),
        "old name must be hidden from tool help: {help}"
    );
}
