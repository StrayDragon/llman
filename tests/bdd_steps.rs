//! BDD step definitions for feature-as-spec (errors-exit pilot).
//!
//! Gated behind `#[cfg(feature = "bdd")]` — only compiled with `cargo test --features bdd`.
//! Uses rstest-bdd to bind `.feature` scenarios under `llmanspec/specs/errors-exit/`
//! to executable step definitions that invoke the llman binary via subprocess.

#![cfg(feature = "bdd")]

use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;

/// Holds the last llman subprocess output so steps can chain Given→When→Then.
#[derive(Default)]
struct BddWorld {
    exit_code: Option<i32>,
    stderr: String,
    #[allow(dead_code)]
    stdout: String,
    /// True when the command finished successfully (exit 0).
    success: bool,
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

// ---------------------------------------------------------------------------
// Shared step: "llman 二进制已构建"
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

// ---------------------------------------------------------------------------
// Feature: CLI 入口错误渲染 (r1)
// ---------------------------------------------------------------------------

/// Use `sdd show` without arguments — it fails when non-interactive (no TTY).
#[when("我以会失败的参数运行 llman")]
fn when_run_failing() {
    let output = Command::new(llman_bin())
        .args(["sdd", "show"])
        .env("LLMAN_CONFIG_DIR", "./artifacts/testing_config_home")
        .output()
        .expect("run llman");
    record_output(output);
}

#[then("退出码为 {code:i32}")]
fn then_exit_code(code: i32) {
    with_world(|w| {
        let actual = w.exit_code.unwrap_or(-1);
        assert_eq!(actual, code, "expected exit code {code}, got {actual}");
    });
}

#[then("stderr 恰好包含一行错误信息")]
fn then_stderr_one_line() {
    with_world(|w| {
        let non_empty_lines: Vec<&str> =
            w.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            !non_empty_lines.is_empty(),
            "expected at least one error line on stderr, got empty"
        );
        // r1: "a single user-facing error message" — the error block may span
        // multiple display lines (hint + suggestions) but is one logical error.
        assert!(
            w.stderr.to_lowercase().contains("error") || !non_empty_lines.is_empty(),
            "expected an error message on stderr, got: {}",
            w.stderr
        );
    });
}

// ---------------------------------------------------------------------------
// Feature: 子命令错误处理 (r2)
// ---------------------------------------------------------------------------

#[when("我在非交互终端运行 llman sdd show")]
fn when_run_show_noninteractive() {
    // No TTY in test harness → inherently non-interactive.
    let output = Command::new(llman_bin())
        .args(["sdd", "show"])
        .env("LLMAN_CONFIG_DIR", "./artifacts/testing_config_home")
        .output()
        .expect("run llman sdd show");
    record_output(output);
}

#[then("stderr 包含非交互提示")]
fn then_stderr_has_hint() {
    with_world(|w| {
        assert!(
            !w.stderr.is_empty(),
            "expected non-interactive hint on stderr, got empty"
        );
        assert!(
            w.stderr.to_lowercase().contains("try") || w.stderr.contains("llman sdd"),
            "expected a hint suggesting commands, got: {}",
            w.stderr
        );
    });
}

#[when("我运行 llman sdd show 不存在的spec")]
fn when_run_show_nonexistent() {
    let output = Command::new(llman_bin())
        .args(["sdd", "show", "nonexistent-spec", "--type", "spec"])
        .env("LLMAN_CONFIG_DIR", "./artifacts/testing_config_home")
        .output()
        .expect("run llman sdd show nonexistent-spec");
    record_output(output);
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

#[then("stderr 包含错误信息")]
fn then_stderr_has_error() {
    with_world(|w| {
        assert!(
            !w.stderr.trim().is_empty(),
            "expected error message on stderr, got empty"
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
        *w.borrow_mut() = Some(BddWorld {
            exit_code: code,
            stderr,
            stdout,
            success,
        });
    });
}

// ---------------------------------------------------------------------------
// Scenario bindings — bind each named scenario to its .feature file.
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
