//! External subcommand delegation (cli.feature r56): `llman <name>` resolves
//! `llman-<name>` on PATH and forwards argv/env/stdio/cwd/exit-code to the
//! plugin process. Fixtures are executable shell scripts in a TempDir that
//! is prepended to the child PATH.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn write_plugin(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, body).expect("write plugin script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod +x plugin");
}

fn echo_plugin_body() -> &'static str {
    "#!/bin/sh\nprintf 'args=%s\\n' \"$*\"\nprintf 'config=%s\\n' \"$LLMAN_CONFIG_DIR\"\n"
}

fn run_llman_with_plugin_path(plugin_dir: &Path, args: &[&str]) -> Output {
    run_llman_with_home(plugin_dir, None, args)
}

/// Run `llman` with the plugin dir prepended to PATH and package-manager env
/// vars pointed away from the real machine: discovery must not pick up real
/// `~/.bun/bin` etc., so plugin assertions stay deterministic. `home=None`
/// creates a fresh empty home.
fn run_llman_with_home(plugin_dir: &Path, home: Option<&Path>, args: &[&str]) -> Output {
    let isolated_home;
    let home = match home {
        Some(path) => path,
        None => {
            isolated_home = TempDir::new().expect("isolated home tempdir");
            isolated_home.path()
        }
    };
    let path_value = format!(
        "{}:{}",
        plugin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    run_llman_with_path_env(&path_value, home, args)
}

/// Full control over the child's PATH and HOME: plugins are spawned by
/// absolute path, so a bare-dir PATH keeps a test hermetic against whatever
/// else is installed on the real machine.
fn run_llman_with_path_env(path_value: &str, home: &Path, args: &[&str]) -> Output {
    Command::new(llman_bin())
        .args(args)
        .env("PATH", path_value)
        .env("HOME", home)
        .env_remove("BUN_INSTALL")
        .env_remove("PNPM_HOME")
        .output()
        .expect("run llman with plugin PATH")
}

#[test]
fn external_delegation_forwards_argv_verbatim() {
    let plugins = TempDir::new().expect("plugin tempdir");
    write_plugin(plugins.path(), "llman-fake-echo", echo_plugin_body());

    let output = run_llman_with_plugin_path(plugins.path(), &["fake-echo", "--flag", "value"]);

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("args=--flag value"),
        "plugin argv not forwarded verbatim; stdout:\n{stdout}"
    );
}

#[test]
fn external_delegation_injects_resolved_config_dir_env() {
    let plugins = TempDir::new().expect("plugin tempdir");
    let config = TempDir::new().expect("config tempdir");
    write_plugin(plugins.path(), "llman-fake-echo", echo_plugin_body());

    let config_arg = format!("--config-dir={}", config.path().display());
    let output = run_llman_with_plugin_path(plugins.path(), &[&config_arg, "fake-echo"]);

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = format!("config={}", config.path().display());
    assert!(
        stdout.contains(&expected),
        "child LLMAN_CONFIG_DIR not normalized to resolved config dir; stdout:\n{stdout}"
    );
}

#[test]
fn external_delegation_propagates_plugin_exit_code() {
    let plugins = TempDir::new().expect("plugin tempdir");
    write_plugin(plugins.path(), "llman-fake-exit", "#!/bin/sh\nexit 3\n");

    let output = run_llman_with_plugin_path(plugins.path(), &["fake-exit"]);

    assert_eq!(
        output.status.code(),
        Some(3),
        "plugin exit code must propagate"
    );
}

#[test]
fn external_delegation_maps_signal_death_to_128_plus_signum() {
    let plugins = TempDir::new().expect("plugin tempdir");
    write_plugin(
        plugins.path(),
        "llman-fake-signal",
        "#!/bin/sh\nkill -TERM $$\n",
    );

    let output = run_llman_with_plugin_path(plugins.path(), &["fake-signal"]);

    assert_eq!(
        output.status.code(),
        Some(128 + 15),
        "signal death (SIGTERM=15) must map to exit 143; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn external_not_found_reports_unrecognized_and_plugin_hint() {
    let plugins = TempDir::new().expect("plugin tempdir");
    write_plugin(plugins.path(), "llman-real-plugin", "#!/bin/sh\nexit 0\n");

    let output = run_llman_with_plugin_path(plugins.path(), &["no-such-cmd"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized"),
        "stderr must report unrecognized command; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("llman-real-plugin"),
        "stderr must list discovered plugins; stderr:\n{stderr}"
    );
}

#[test]
fn external_not_found_without_plugins_degrades_gracefully() {
    let plugins = TempDir::new().expect("plugin tempdir");

    let output = run_llman_with_plugin_path(plugins.path(), &["no-such-cmd"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unrecognized"));
    assert!(
        !stderr.contains("discovered llman-* commands"),
        "empty discovery must omit the hint section; stderr:\n{stderr}"
    );
}

#[test]
fn sdd_subcommand_delegates_to_llman_sdd_plugin() {
    // The SDD workflow moved to the standalone llman-sdd v2 project, so `sdd`
    // is NOT a built-in anymore: `llman sdd <args>` must reach `llman-sdd`
    // via external discovery (argv verbatim, exit code propagated).
    let plugins = TempDir::new().expect("plugin tempdir");
    write_plugin(
        plugins.path(),
        "llman-sdd",
        "#!/bin/sh\nprintf 'sdd-args=%s\\n' \"$*\"\nexit 7\n",
    );

    let output = run_llman_with_plugin_path(plugins.path(), &["sdd", "list", "--specs", "--json"]);

    assert_eq!(
        output.status.code(),
        Some(7),
        "plugin exit code must propagate; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sdd-args=list --specs --json"),
        "argv not forwarded to llman-sdd; stdout:\n{stdout}"
    );
}

#[test]
fn sdd_plugin_resolves_from_bun_bin_when_dir_is_off_path() {
    // Dev installs via `bun link` live under <home>/.bun/bin, which is not
    // guaranteed to be on PATH; discovery must fall back to it. PATH holds
    // only a bare empty dir so nothing else on the machine can answer.
    let home = TempDir::new().expect("home tempdir");
    let bun_bin = home.path().join(".bun/bin");
    fs::create_dir_all(&bun_bin).expect("create .bun/bin");
    write_plugin(&bun_bin, "llman-sdd", "#!/bin/sh\nprintf 'bun-linked\\n'\n");

    let empty_path_dir = TempDir::new().expect("plugin tempdir");
    let output = run_llman_with_path_env(
        empty_path_dir.path().to_str().expect("utf-8 tempdir path"),
        home.path(),
        &["sdd"],
    );

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bun-linked"),
        "off-PATH ~/.bun/bin plugin not resolved; stdout:\n{stdout}"
    );
}

#[test]
fn sdd_not_found_suggests_migration_install() {
    let plugins = TempDir::new().expect("plugin tempdir");

    let output = run_llman_with_plugin_path(plugins.path(), &["sdd"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("sdd functionality has moved"),
        "sdd not-found must include migration hint; stderr:\n{stderr}"
    );
}

#[test]
fn non_sdd_not_found_omits_migration_hint() {
    let plugins = TempDir::new().expect("plugin tempdir");

    let output = run_llman_with_plugin_path(plugins.path(), &["no-such-cmd"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("sdd functionality has moved"),
        "migration hint must not leak into other commands' errors; stderr:\n{stderr}"
    );
}
