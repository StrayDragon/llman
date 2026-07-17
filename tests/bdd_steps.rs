//! Generic BDD step library for feature-as-spec (rstest-bdd).
//!
//! Gated behind `#[cfg(feature = "bdd")]` — only compiled with `cargo test --features bdd`.
//! Provides a reusable "run llman → assert output" vocabulary so that CLI-testable
//! `.feature` scenarios can be bound without writing per-scenario step functions.
//!
//! Step vocabulary:
//!   Given:
//!     - 假如 llman 二进制已构建            (reset world + assert binary exists)
//!     - 假如 已初始化 sdd 项目且 bdd 配置为 {mode}  (create a seeded TempDir project:
//!          mode="on" writes a bdd: block, "off" omits it; author a sample spec +
//!          an add-scen change delta; git init+commit; sets cwd to the project)
//!     - 假如 {env_var} 为 {value}          (accumulate env override for subprocess)
//!     - 假如今目录为 {cwd}                 (set working directory for subprocess)
//!   When:
//!     - 当 运行 llman {args}               (run llman with whitespace-split args)
//!     - 当 在非交互终端运行 llman {args}    (same, non-interactive)
//!   Then:
//!     - 那么 退出码为 {code:i32}           (exact exit code)
//!     - 那么 退出码非零                    (non-zero exit)
//!     - 那么 退出码为零                    (zero exit)
//!     - 那么 stdout 包含 {text}            (substring on stdout)
//!     - 那么 stderr 包含 {text}            (substring on stderr)
//!     - 那么 stdout 不含 {text}            (negated substring on stdout)
//!     - 那么 stderr 不含 {text}            (negated substring on stderr)
//!     - 那么 stdout 为合法 JSON            (stdout parses as JSON)
//!     - 那么 stdout 含 JSON 键 {key}       (stdout JSON has top-level key)

#![cfg(feature = "bdd")]

use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

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
    /// Owned temp project created by `已初始化 sdd 项目…` Given step. Kept here so
    /// it is not dropped (and deleted) before the scenario's When/Then run.
    fixture_dir: Option<TempDir>,
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

/// Run llman in a specific directory (for fixture setup); asserts success but
/// does NOT record output into the world (setup steps are not assertion targets).
fn run_llman_in(dir: &std::path::Path, args_raw: &str, extra_env: &[(&str, &str)]) {
    let mut cmd = Command::new(llman_bin());
    cmd.args(split_args(args_raw));
    cmd.env("LLMAN_CONFIG_DIR", "./artifacts/testing_config_home");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.current_dir(dir);
    let output = cmd.output().expect("run llman in fixture");
    assert!(
        output.status.success(),
        "fixture setup command failed: `{args_raw}` in {}\nstdout:\n{}\nstderr:\n{}",
        dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
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

/// Create a seeded sdd project in a fresh TempDir and point the world's cwd at it.
/// `mode` = "on" writes a `bdd:` block (enables feature-as-spec); "off" omits it.
/// The project gets a `sample` spec (r1 + scenario) and an `add-scen` change whose
/// delta adds r2 + a scenario, so `solidify`/`validate`/`index` have something to
/// act on. This mirrors `tests/sdd_bdd_compat_tests.rs::seed_spec_and_change`.
fn seed_bdd_project(mode: &str) {
    // reset first (same convention as `llman 二进制已构建`) so the scenario starts
    // clean; then install the fixture.
    reset_world();
    let temp = TempDir::new().expect("create fixture tempdir");
    let dir = temp.path().to_path_buf();

    // init first (generates default BDD-off config); we overwrite config.yaml to
    // the requested bdd mode AFTER all authoring commands, because some sdd
    // subcommands rewrite config.yaml on write paths.
    run_llman_in(&dir, "sdd init --lang en", &[]);

    // author sample spec: r1 + a scenario.
    run_llman_in(&dir, "sdd spec skeleton sample", &[]);
    run_llman_in(
        &dir,
        "sdd spec add-requirement sample r1 --title R1 --statement \"System MUST do X.\"",
        &[],
    );
    run_llman_in(
        &dir,
        "sdd spec add-scenario sample r1 happy --when trigger --then outcome",
        &[],
    );

    // author add-scen change: delta adds r2 + a scenario (solidify target).
    let change_dir = dir.join("llmanspec/changes/add-scen");
    std::fs::create_dir_all(&change_dir).expect("mkdir fixture change");
    std::fs::write(
        change_dir.join("proposal.md"),
        "## Why\nAdd r2 to sample.\n\n## What Changes\n- Add requirement r2.\n",
    )
    .expect("write fixture proposal");
    run_llman_in(&dir, "sdd delta skeleton add-scen sample", &[]);
    run_llman_in(
        &dir,
        "sdd delta add-req add-scen sample r2 --title R2 --statement \"System MUST support r2.\"",
        &[],
    );
    run_llman_in(
        &dir,
        "sdd delta add-scenario add-scen sample r2 \"new r2 behavior\" --when \"r2 triggered\" --then \"r2 works\"",
        &[],
    );

    // Overwrite config.yaml to the requested bdd mode AFTER authoring (authoring
    // commands rewrite config.yaml, so this must be the last config write).
    // rstest-bdd captures quoted placeholders verbatim, so `bdd 配置为 "on"` yields
    // mode = "\"on\"" — strip quotes before comparing.
    let mode_norm = mode.trim().trim_matches('"');
    let mut config = "schema: spec-driven\nlocale: en\n".to_string();
    if mode_norm == "on" {
        config.push_str("\nbdd:\n  run_command: \"cargo test --features bdd\"\n");
    }
    std::fs::write(dir.join("llmanspec/config.yaml"), config).expect("write fixture config");

    // BDD-off: drop a deliberately malformed .feature next to the spec to prove
    // validate ignores it (r4 contract). BDD-on gets a real .feature via solidify
    // in the scenario's When step, so nothing to seed here for "on".
    if mode_norm == "off" {
        std::fs::write(
            dir.join("llmanspec/specs/sample/sample.feature"),
            "# language: en\nTHIS IS NOT VALID GHERKIN {{{",
        )
        .expect("write bogus feature");
    }

    // git init+commit: staleness checks need a base ref.
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&dir)
        .output()
        .expect("git init fixture");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&dir)
        .output()
        .expect("git add fixture");
    Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@x",
            "commit",
            "-qm",
            "fixture",
        ])
        .current_dir(&dir)
        .output()
        .expect("git commit fixture");

    WORLD.with(|w| {
        let mut w = w.borrow_mut();
        let world = w.as_mut().expect("world not initialized");
        world.fixture_dir = Some(temp);
        world.cwd = Some(dir);
    });
}

fn fixture_cwd() -> PathBuf {
    WORLD.with(|w| {
        w.borrow()
            .as_ref()
            .expect("world not initialized")
            .cwd
            .clone()
            .expect("fixture cwd missing")
    })
}

#[given("已初始化 sdd 项目且 bdd 配置为 {mode}")]
fn given_seeded_sdd_project(mode: String) {
    seed_bdd_project(&mode);
}

/// BDD-on fixture where sample still has executable GWT in toon *and* a matching
/// `.feature` scenario — triggers Partitioned dual-write validate ERROR.
#[given("已初始化含可执行双写的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_dual_write(mode: String) {
    seed_bdd_project(&mode);
    let feature = fixture_cwd().join("llmanspec/specs/sample/sample.feature");
    std::fs::write(
        &feature,
        "# language: en\nFeature: sample\n  @req:r1\n  Scenario: happy\n    Given a\n    When b\n    Then c\n",
    )
    .expect("write dual-write feature");
}

/// Two capabilities share the same req_id — triggers global uniqueness ERROR.
#[given("已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_global_req_collision(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    // Second capability reuses r1 (sample already has r1).
    run_llman_in(&dir, "sdd spec skeleton other", &[]);
    // Bypass add-req global guard by writing toon directly.
    std::fs::write(
        dir.join("llmanspec/specs/other/spec.toon"),
        r#"kind: llman.sdd.spec
name: "other"
purpose: "other"
valid_scope[1]: "llmanspec/specs/other"
requirements[1]{req_id,title,statement}:
  r1,Other,"System MUST collide."
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,baseline,"","trigger","outcome",false
"#,
    )
    .expect("write colliding other spec");
}

/// Seed a project then plant an occupied custom tag for add-req guard tests.
#[given("已初始化含已占用全局 req_id 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_occupied_req(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    std::fs::write(
        dir.join("llmanspec/specs/sample/spec.toon"),
        r#"kind: llman.sdd.spec
name: "sample"
purpose: "sample"
valid_scope[1]: "llmanspec/specs/sample"
requirements[2]{req_id,title,statement}:
  r1,R1,"System MUST do X."
  occupied-id,Occupied,"System MUST be unique."
scenarios[2]{req_id,id,given,when,then,feature}:
  r1,happy,"","trigger","outcome",false
  occupied-id,baseline,"","trigger","outcome",false
"#,
    )
    .expect("write occupied sample");
}

/// BDD-on fixture whose harness `@req` points at a missing requirement id.
#[given("已初始化含无效 @req 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_bad_req(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    // Constraints-only toon so validate fails on @req link, not dual-write.
    std::fs::write(
        dir.join("llmanspec/specs/sample/spec.toon"),
        r#"kind: llman.sdd.spec
name: "sample"
purpose: "sample"
valid_scope[1]: "llmanspec/specs/sample"
requirements[1]{req_id,title,statement}:
  r1,R1,"System MUST do X."
scenarios[0]:
"#,
    )
    .expect("rewrite sample toon");
    std::fs::write(
        dir.join("llmanspec/specs/sample/sample.feature"),
        "# language: en\nFeature: sample\n  @req:r999\n  Scenario: bad link\n    Given a\n    When b\n    Then c\n",
    )
    .expect("write bad @req feature");
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

#[then("退出码非零且 stderr 包含 {text}")]
fn then_exit_nonzero_and_stderr_contains(text: String) {
    then_exit_nonzero();
    then_stderr_contains(text);
}

#[then("退出码为零且 stdout 为合法 JSON 且含 JSON 键 {key}")]
fn then_exit_zero_json_key(key: String) {
    then_exit_zero();
    then_stdout_is_json();
    then_stdout_has_json_key(key);
}

#[then("退出码为零且 stdout 为合法 JSON 且含 JSON 键 reqId 且含 JSON 键 capability")]
fn then_exit_zero_json_reqid_and_capability() {
    then_exit_zero();
    then_stdout_is_json();
    then_stdout_has_json_key("reqId".into());
    then_stdout_has_json_key("capability".into());
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

#[scenario(
    path = "llmanspec/specs/errors-exit/errors-exit.feature",
    name = "json-错误输出"
)]
#[test]
fn test_show_json_error_output() {}

// ---------------------------------------------------------------------------
// Scenario bindings — sdd-bdd-mode-compat (BDD on/off behavior contracts).
// ---------------------------------------------------------------------------

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/validate-check.feature",
    name = "BDD-on 时 validate 默认执行 BDD runner"
)]
#[test]
fn test_compat_validate_on_runs_runner() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/validate-check.feature",
    name = "BDD-on 时 validate --no-check 跳过 runner"
)]
#[test]
fn test_compat_validate_on_no_check_skips() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/validate-check.feature",
    name = "BDD-off 时 validate --check 不执行 runner"
)]
#[test]
fn test_compat_validate_off_check_noop() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/solidify-mode.feature",
    name = "BDD-on 时 solidify 产出 .feature 文件"
)]
#[test]
fn test_compat_solidify_on_generates() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/solidify-mode.feature",
    name = "BDD-off 时 solidify 为 no-op 并提示未配置"
)]
#[test]
fn test_compat_solidify_off_noop() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/index-embed.feature",
    name = "BDD-on 时 index rebuild 成功"
)]
#[test]
fn test_compat_index_on_rebuild() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/index-embed.feature",
    name = "BDD-off 时 index rebuild 成功且无 feature embed"
)]
#[test]
fn test_compat_index_off_rebuild() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/feature-ignoring.feature",
    name = "BDD-off 时 validate 忽略格式错误的 feature 文件"
)]
#[test]
fn test_compat_validate_off_ignores_feature() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature",
    name = "双写可执行 GWT 时 validate --strict 失败"
)]
#[test]
fn test_compat_partitioned_dual_write_forbidden() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature",
    name = "@req 指向缺失 requirement 时 validate --strict 失败"
)]
#[test]
fn test_compat_validate_req_link() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature",
    name = "partition-migrate --dry-run 只打印计划"
)]
#[test]
fn test_compat_partition_migrate_dry_run() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature",
    name = "global-req-collision-strict"
)]
#[test]
fn test_global_req_collision_strict() {}

#[scenario(
    path = "llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature",
    name = "global-req-collision-default"
)]
#[test]
fn test_global_req_collision_default() {}

#[scenario(
    path = "llmanspec/specs/sdd-workflow/sdd-workflow.feature",
    name = "next-req-id-json"
)]
#[test]
fn test_next_req_id_json() {}

#[scenario(
    path = "llmanspec/specs/sdd-workflow/sdd-workflow.feature",
    name = "add-req-rejects-global-collision"
)]
#[test]
fn test_add_req_rejects_global_collision() {}

#[scenario(
    path = "llmanspec/specs/sdd-workflow/sdd-workflow.feature",
    name = "resolve-req-json"
)]
#[test]
fn test_resolve_req_json() {}
