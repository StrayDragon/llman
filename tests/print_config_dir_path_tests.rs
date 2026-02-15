mod common;

use common::TestEnvironment;
use std::process::Command;

#[test]
fn print_config_dir_path_outputs_resolved_config_dir() {
    let env = TestEnvironment::new();
    let config_dir = env.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--config-dir",
            config_dir.to_str().expect("config dir"),
            "--print-config-dir-path",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run llman --print-config-dir-path");

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), config_dir.to_str().expect("config dir"));
}
