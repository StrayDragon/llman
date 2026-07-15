//! Generic BDD step library for feature-as-spec (rstest-bdd).
//!
//! Gated behind `#[cfg(feature = "bdd")]` — only compiled with `cargo test --features bdd`.
//! Provides a reusable "run llman → assert output" vocabulary so that CLI-testable
//! `.feature` scenarios can be bound without writing per-scenario step functions.
//!
//! Step vocabulary:
//!   Given:
//!     - 假如 llman 二进制已构建          (reset world + assert binary exists)
//!     - 假如 {env_var} 为 {value}        (accumulate env override for subprocess)
//!     - 假如今目录为 {cwd}               (set working directory for subprocess)
//!   When:
//!     - 当 运行 llman {args}             (run llman with whitespace-split args)
//!     - 当 在非交互终端运行 llman {args}  (same, non-interactive)
//!   Then:
//!     - 那么 退出码为 {code:i32}         (exact exit code)
//!     - 那么 退出码非零                  (non-zero exit)
//!     - 那么 退出码为零                  (zero exit)
//!     - 那么 stdout 包含 {text}          (substring on stdout)
//!     - 那么 stderr 包含 {text}          (substring on stderr)
//!     - 那么 stdout 不含 {text}          (negated substring on stdout)
//!     - 那么 stderr 不含 {text}          (negated substring on stderr)
//!     - 那么 stdout 为合法 JSON          (stdout parses as JSON)
//!     - 那么 stdout 含 JSON 键 {key}     (stdout JSON has top-level key)

#![cfg(feature = "bdd")]

use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Holds the last llman subprocess output so steps can chain Given→When→Then.
#[derive(Default)]
struct BddWorld {
    exit_code: Option<i32>,
    stderr: String,
    stdout: String,
    /// True when the command finished successfully (exit 0).
    success: bool,
    /// Env overrides accumulated by Given steps; merged into the subprocess.
    env_overrides: HashMap<String, String>,
    /// Optional working directory override for the subprocess.
    cwd: Option<PathBuf>,
}

// Each scenario runs in a single thread, so thread-local storage avoids the
// parallel-test contention that a global Mutex would cause.
thread_local! {
    static WORLD: RefCell<Option<BddWorld>> = const { RefCell::new(None) };
}

fn reset_world() {
    WORLD.with(|w| *w.borrow_mut() = Some(BddWorld::default()));
}

fn with_world<F, R>(f: F) -> R
where
    F: FnOnce(&BddWorld) -> R,
{
    WORLD.with(|w| {
        let w = w.borrow();
        let w = w.as_ref().expect("world not initialized");
        f(w)
    })
}

fn llman_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_llman"))
}

fn split_args(raw: &str) -> Vec<String> {
    // Whitespace split with quote awareness: keep quoted segments together.
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for ch in raw.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn run_llman(args_raw: &str) {
    let (env_overrides, cwd) = WORLD.with(|w| {
        let w = w.borrow();
        let w = w.as_ref().expect("world not initialized");
        (w.env_overrides.clone(), w.cwd.clone())
    });

    let mut cmd = Command::new(llman_bin());
    cmd.args(split_args(args_raw));
    cmd.env("LLMAN_CONFIG_DIR", "./artifacts/testing_config_home");
    for (k, v) in &env_overrides {
        cmd.env(k, v);
    }
    if let Some(dir) = &cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().expect("run llman");
    record_output(output);
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given("llman 二进制已构建")]
fn given_binary_built() {
    reset_world();
    assert!(
        llman_bin().exists(),
        "llman binary not found at {}",
        llman_bin().display()
    );
}

#[given("{env_var} 为 {value}")]
fn given_env_var(env_var: String, value: String) {
    WORLD.with(|w| {
        let mut w = w.borrow_mut();
        let world = w.as_mut().expect("world not initialized");
        world.env_overrides.insert(env_var, value);
    });
}

#[given("今目录为 {cwd}")]
fn given_cwd(cwd: PathBuf) {
    WORLD.with(|w| {
        let mut w = w.borrow_mut();
        let world = w.as_mut().expect("world not initialized");
        world.cwd = Some(cwd);
    });
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when("运行 llman {args}")]
fn when_run_llman(args: String) {
    run_llman(&args);
}

#[when("在非交互终端运行 llman {args}")]
fn when_run_llman_noninteractive(args: String) {
    // No TTY in test harness → inherently non-interactive.
    run_llman(&args);
}

// ---------------------------------------------------------------------------
// Then steps — exit codes
// ---------------------------------------------------------------------------

#[then("退出码为 {code:i32}")]
fn then_exit_code(code: i32) {
    with_world(|w| {
        let actual = w.exit_code.unwrap_or(-1);
        assert_eq!(actual, code, "expected exit code {code}, got {actual}");
    });
}

#[then("退出码非零")]
fn then_exit_nonzero() {
    with_world(|w| {
        assert!(
            !w.success,
            "expected non-zero exit code, got success (exit {:?})",
            w.exit_code
        );
    });
}

#[then("退出码为零")]
fn then_exit_zero() {
    with_world(|w| {
        assert!(
            w.success,
            "expected zero exit code, got failure (exit {:?})",
            w.exit_code
        );
    });
}

// ---------------------------------------------------------------------------
// Then steps — output substring assertions
// ---------------------------------------------------------------------------

#[then("stdout 包含 {text}")]
fn then_stdout_contains(text: String) {
    with_world(|w| {
        assert!(
            w.stdout.contains(&text),
            "expected stdout to contain {:?}, got: {}",
            text,
            w.stdout
        );
    });
}

#[then("stderr 包含 {text}")]
fn then_stderr_contains(text: String) {
    with_world(|w| {
        assert!(
            w.stderr.contains(&text),
            "expected stderr to contain {:?}, got: {}",
            text,
            w.stderr
        );
    });
}

#[then("stdout 不含 {text}")]
fn then_stdout_not_contains(text: String) {
    with_world(|w| {
        assert!(
            !w.stdout.contains(&text),
            "expected stdout to NOT contain {:?}, got: {}",
            text,
            w.stdout
        );
    });
}

#[then("stderr 不含 {text}")]
fn then_stderr_not_contains(text: String) {
    with_world(|w| {
        assert!(
            !w.stderr.contains(&text),
            "expected stderr to NOT contain {:?}, got: {}",
            text,
            w.stderr
        );
    });
}

// ---------------------------------------------------------------------------
// Then steps — JSON structure assertions
// ---------------------------------------------------------------------------

#[then("stdout 为合法 JSON")]
fn then_stdout_is_json() {
    with_world(|w| {
        serde_json::from_str::<serde_json::Value>(&w.stdout)
            .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{}", w.stdout));
    });
}

#[then("stdout 含 JSON 键 {key}")]
fn then_stdout_has_json_key(key: String) {
    with_world(|w| {
        let value: serde_json::Value = serde_json::from_str(&w.stdout)
            .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{}", w.stdout));
        let obj = value.as_object().unwrap_or_else(|| {
            panic!("stdout JSON is not an object, cannot check key {key:?}");
        });
        assert!(
            obj.contains_key(&key),
            "expected stdout JSON to contain key {key:?}, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    });
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn record_output(output: std::process::Output) {
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();
    WORLD.with(|w| {
        let mut w = w.borrow_mut();
        let world = w.as_mut().expect("world not initialized");
        world.exit_code = code;
        world.stderr = stderr;
        world.stdout = stdout;
        world.success = success;
    });
}

// ---------------------------------------------------------------------------
// Scenario bindings — errors-exit pilot (rewritten with generic steps).
// ---------------------------------------------------------------------------

#[scenario(
    path = "llmanspec/specs/errors-exit/error-rendering.feature",
    name = "子命令返回错误时打印单行错误并以退出码 1 退出"
)]
#[test]
fn test_error_rendering() {}

#[scenario(
    path = "llmanspec/specs/errors-exit/subcommand-error-handling.feature",
    name = "非交互终端下 sdd show 无参数时以退出码 1 退出"
)]
#[test]
fn test_show_noninteractive_exit() {}

#[scenario(
    path = "llmanspec/specs/errors-exit/subcommand-error-handling.feature",
    name = "查看不存在的 spec 时正常报错而非 panic"
)]
#[test]
fn test_show_nonexistent_spec() {}
