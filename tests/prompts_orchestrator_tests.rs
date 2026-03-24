mod common;

use common::{assert_success, prepare_work_and_config_dirs, run_llman};
use tempfile::TempDir;

#[test]
fn test_prompts_no_interactive_prints_delegation_guidance() {
    let temp = TempDir::new().expect("temp dir");
    let (work_dir, config_dir) = prepare_work_and_config_dirs(temp.path());

    let output = run_llman(&["prompts", "--no-interactive"], &work_dir, &config_dir);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("llman x cursor prompts"));
    assert!(stdout.contains("llman x codex prompts"));
    assert!(stdout.contains("llman x claude-code prompts"));
}

#[test]
fn test_prompt_alias_no_interactive_prints_delegation_guidance() {
    let temp = TempDir::new().expect("temp dir");
    let (work_dir, config_dir) = prepare_work_and_config_dirs(temp.path());

    let output = run_llman(&["prompt", "--no-interactive"], &work_dir, &config_dir);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("llman x cursor prompts"));
    assert!(stdout.contains("llman x codex prompts"));
    assert!(stdout.contains("llman x claude-code prompts"));
}
