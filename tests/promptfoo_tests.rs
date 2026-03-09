use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn run_llman(args: &[&str], work_dir: &Path, config_dir: &Path, envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(llman_bin());
    cmd.args(["--config-dir", config_dir.to_str().expect("config dir")])
        .args(args)
        .current_dir(work_dir);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("run llman")
}

#[test]
fn promptfoo_check_errors_when_runner_missing() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).expect("create config dir");

    let out = run_llman(
        &["x", "promptfoo", "check"],
        root,
        &config_dir,
        &[("LLMAN_PROMPTFOO_CMD", "llman-test-missing-promptfoo-cmd")],
    );

    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Promptfoo runner not found"),
        "unexpected stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("LLMAN_PROMPTFOO_CMD"),
        "unexpected stderr:\n{stderr}"
    );
}

#[test]
fn promptfoo_eval_dry_run_prints_resolved_argv_json() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).expect("create config dir");

    let promptfoo_dir = root.join("promptfoo");
    fs::create_dir_all(&promptfoo_dir).expect("create promptfoo dir");
    let cfg_path = promptfoo_dir.join("promptfooconfig.yaml");
    fs::write(&cfg_path, "description: dry-run\nproviders: [promptfoo:manual-input]\nprompts: ['hi']\ntests: [{vars:{}}]\n").expect("write cfg");
    let results_path = promptfoo_dir.join("results.json");

    let out = run_llman(
        &[
            "x",
            "promptfoo",
            "eval",
            "--dry-run",
            "--config",
            cfg_path.to_str().unwrap(),
            "--output",
            results_path.to_str().unwrap(),
        ],
        root,
        &config_dir,
        &[("LLMAN_PROMPTFOO_CMD", "npx promptfoo@latest")],
    );

    assert!(
        out.status.success(),
        "dry-run should succeed.\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("parse stdout json");
    assert_eq!(json.get("dry_run").and_then(Value::as_bool), Some(true));

    let argv = json
        .get("argv")
        .and_then(Value::as_array)
        .expect("argv array")
        .iter()
        .map(|v| v.as_str().expect("argv string").to_string())
        .collect::<Vec<_>>();

    assert_eq!(argv.get(0).map(String::as_str), Some("npx"));
    assert_eq!(argv.get(1).map(String::as_str), Some("promptfoo@latest"));
    assert_eq!(argv.get(2).map(String::as_str), Some("eval"));

    let config_i = argv
        .iter()
        .position(|s| s == "--config")
        .expect("find --config");
    assert_eq!(
        argv.get(config_i + 1).map(String::as_str),
        cfg_path.to_str()
    );

    let output_i = argv
        .iter()
        .position(|s| s == "--output")
        .expect("find --output");
    assert_eq!(
        argv.get(output_i + 1).map(String::as_str),
        results_path.to_str()
    );
}
