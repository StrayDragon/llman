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
//!     - 假如 项目中存在技能目录 {name}     (plant `.agents/skills/<name>/SKILL.md`)
//!     - 假如 项目 extra_skills 包含 {name} (rewrite config.yaml `extra_skills`)
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
//!     - 那么 相对路径 {rel} 存在           (path under fixture cwd)
//!     - 那么 相对路径 {rel} 不存在         (path under fixture cwd absent)
//!     - 那么 相对路径 {rel} 内容包含 {text} (substring on file content)

#![cfg(feature = "bdd")]

use rstest_bdd_macros::{given, scenarios, then, when};
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
/// The project gets a `sample` spec (r1 + non-executable scenario note) and an
/// `add-scen` change whose delta adds r2, plus a live `.feature` when BDD-on so
/// index/validate have harness content. This mirrors
/// `tests/sdd_bdd_compat_tests.rs::seed_spec_and_change`.
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

    // Single-track (r131): seed `sample` as one `.feature` with a @human rule
    // plus an executable acceptance scenario for harness content.
    write_single_track_spec(&dir, "sample", &[("r1", "R1")]);
    let sample_feature = dir.join("llmanspec/specs/sample/sample.feature");
    let mut body = std::fs::read_to_string(&sample_feature).expect("read seeded feature");
    body.push_str(
        "\n  @req:r1 @executable\n  Scenario: harness-happy\n    Given a precondition\n    When an action\n    Then an outcome\n",
    );
    std::fs::write(&sample_feature, body).expect("append acceptance scenario");

    // author add-scen change: proposal only (delta specs are removed, r115).
    let change_dir = dir.join("llmanspec/changes/add-scen");
    std::fs::create_dir_all(&change_dir).expect("mkdir fixture change");
    std::fs::write(
        change_dir.join("proposal.md"),
        "## Why\nAdd r2 to sample.\n\n## What Changes\n- Add requirement r2.\n",
    )
    .expect("write fixture proposal");
    std::fs::write(change_dir.join("design.md"), "# Design\n").expect("write fixture design");
    std::fs::write(change_dir.join("tasks.md"), "- [x] t1\n").expect("write fixture tasks");

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

    // Regenerated skills must match final bdd mode (r95 metadata gate).
    run_llman_in(&dir, "sdd init --update", &[]);

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

#[given("已初始化含多个 capability 且无占位符计数 run_command 的 sdd 项目")]
fn given_multi_cap_counter_run_command() {
    reset_world();
    let temp = TempDir::new().expect("create fixture tempdir");
    let dir = temp.path().to_path_buf();

    run_llman_in(&dir, "sdd init --lang en", &[]);

    for (name, req) in [("sample", "r1"), ("other", "r2")] {
        write_single_track_spec(&dir, name, &[(req, name)]);
    }

    // Project-wide runner with no {feature_*} placeholders; each spawn appends one line.
    let config = "schema: spec-driven\nlocale: en\n\nbdd:\n  run_command: \"printf 'x\\n' >> .bdd-run-count\"\n";
    std::fs::write(dir.join("llmanspec/config.yaml"), config).expect("write counter config");
    run_llman_in(&dir, "sdd init --update", &[]);

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

/// Write a minimal valid single-track spec for `name` with the given rules.
fn write_single_track_spec(dir: &std::path::Path, name: &str, reqs: &[(&str, &str)]) {
    let spec_dir = dir.join(format!("llmanspec/specs/{name}"));
    std::fs::create_dir_all(&spec_dir).expect("mkdir spec dir");
    let mut body = format!(
        "# language: en\n# capability: {name}\n# purpose: {name}\n# scope: llmanspec/specs/{name}\n\nFeature: {name}\n"
    );
    for (id, title) in reqs {
        body.push_str(&format!(
            "\n  @req:{id} @human\n  Scenario: {title}\n    System MUST cover {title}.\n"
        ));
    }
    std::fs::write(spec_dir.join(format!("{name}.feature")), body).expect("write feature");
}

#[given("已初始化 sdd 项目且 bdd 配置为 {mode}")]
fn given_seeded_sdd_project(mode: String) {
    seed_bdd_project(&mode);
}

/// Two capabilities share the same req_id — triggers global uniqueness ERROR.
#[given("已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_global_req_collision(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    // Second capability reuses r1 (sample already has r1).
    write_single_track_spec(&dir, "other", &[("r1", "Other")]);
}

/// Seed a project then plant an occupied custom tag for add-req guard tests.
#[given("已初始化含已占用全局 req_id 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_occupied_req(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    write_single_track_spec(&dir, "sample", &[("r1", "R1"), ("occupied-id", "Occupied")]);
}

/// Seed a project then corrupt an extra change proposal (unknown depends_on
/// ref) so `validate --all` reports a change-level ERROR — review CRITICAL
/// exit-code fixture.
#[given("已初始化含损坏 proposal 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_corrupted_proposal(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    let changes_dir = dir.join("llmanspec/changes/broken");
    std::fs::create_dir_all(&changes_dir).expect("create change dir");
    std::fs::write(
        changes_dir.join("proposal.md"),
        "---\ndepends_on: [nonexistent-change]\n---\n\n## Why\nx\n\n## What Changes\n- y\n",
    )
    .expect("write corrupted proposal");
}

/// Seed a project with one active change `c123-fix-bug` plus an archived
/// change — the r112 prefix-match resolution fixture (Given for
/// prefix-match-baseline / prefix-match-hint).
#[given("存在 active change 和 archived change 且含 c123-fix-bug")]
fn given_active_and_archived_changes_with_c123() {
    reset_world();
    let temp = TempDir::new().expect("create fixture tempdir");
    let dir = temp.path().to_path_buf();
    run_llman_in(&dir, "sdd init --lang en", &[]);
    write_single_track_spec(&dir, "sample", &[("r1", "R1")]);

    // Active change c123-fix-bug (proposal + tasks + design).
    let active = dir.join("llmanspec/changes/c123-fix-bug");
    std::fs::create_dir_all(&active).expect("mkdir active change");
    std::fs::write(
        active.join("proposal.md"),
        "---\ndepends_on: []\n---\n\n## Why\nFix bug c123.\n\n## What Changes\n- Fix it.\n",
    )
    .expect("write active proposal");
    std::fs::write(active.join("design.md"), "# Design\n").expect("write design");
    std::fs::write(active.join("tasks.md"), "- [x] t1\n").expect("write tasks");

    // Archived change under changes/archive/.
    let archived = dir.join("llmanspec/changes/archive/c9-other");
    std::fs::create_dir_all(&archived).expect("mkdir archived change");
    std::fs::write(
        archived.join("proposal.md"),
        "---\ndepends_on: []\n---\n\n## Why\nOld change.\n\n## What Changes\n- Done.\n",
    )
    .expect("write archived proposal");
    std::fs::write(archived.join("design.md"), "# Design\n").expect("write archived design");
    std::fs::write(archived.join("tasks.md"), "- [x] t1\n").expect("write archived tasks");

    WORLD.with(|w| {
        let mut w = w.borrow_mut();
        let world = w.as_mut().expect("world not initialized");
        world.fixture_dir = Some(temp);
        world.cwd = Some(dir);
    });
}

/// BDD fixture with a leftover legacy `spec.toon` next to the single-track
/// feature — triggers the r131 migration-pointer ERROR.
#[given("已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_legacy_toon(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    std::fs::write(
        dir.join("llmanspec/specs/sample/spec.toon"),
        concat!(
            "kind: llman.sdd.spec\n",
            "name: \"sample\"\n",
            "purpose: \"sample legacy\"\n",
            "valid_scope[1]: \"llmanspec/specs/sample\"\n",
            "requirements[1]{req_id,title,statement}:\n",
            "  r1,R1,\"System MUST do X.\"\n",
            "scenarios[0]:\n",
        ),
    )
    .expect("write legacy toon");
}

/// BDD fixture with a toon-ONLY `legacy` capability (no `.feature` in the
/// dir) — migrate creates `legacy.feature` from spec.toon alone.
#[given("已初始化含仅遗留 spec.toon 的 legacy capability 且 bdd 配置为 {mode}")]
fn given_sdd_project_toon_only_capability(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    let spec_dir = dir.join("llmanspec/specs/legacy");
    std::fs::create_dir_all(&spec_dir).expect("mkdir legacy capability");
    std::fs::write(
        spec_dir.join("spec.toon"),
        concat!(
            "kind: llman.sdd.spec\n",
            "name: \"legacy\"\n",
            "purpose: \"legacy notes\"\n",
            "valid_scope[1]: \"llmanspec/specs/legacy\"\n",
            "requirements[1]{req_id,title,statement}:\n",
            "  r1,R1,\"System MUST do X.\"\n",
            "scenarios[0]:\n",
        ),
    )
    .expect("write toon-only spec.toon");
}

/// BDD fixture: `sample3` has a legacy spec.toon (GWT rows: two paired, one
/// unpaired, one contentless) plus a live legacy multi-file `.feature` with an
/// @executable scenario — migrate must leave the .feature untouched and
/// convert the GWT toon rows into @human note scenarios (r136).
#[given("已初始化含遗留 spec.toon 与既有 .feature 的 sample3 capability 且 bdd 配置为 {mode}")]
fn given_sdd_project_toon_with_existing_features(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    let spec_dir = dir.join("llmanspec/specs/sample3");
    std::fs::create_dir_all(&spec_dir).expect("mkdir sample3 capability");
    std::fs::write(
        spec_dir.join("spec.toon"),
        concat!(
            "kind: llman.sdd.spec\n",
            "name: \"sample3\"\n",
            "purpose: \"sample3 legacy\"\n",
            "valid_scope[1]: \"llmanspec/specs/sample3\"\n",
            "requirements[1]{req_id,title,statement}:\n",
            "  r1,R1,\"System MUST do X.\"\n",
            "scenarios[4]{req_id,id,given,when,then,feature}:\n",
            "  r1,acc-1,\"precondition ready\",\"run llman sdd validate sample3\",\"exit code is zero\",true\n",
            "  r1,acc-2,\"\",\"run llman sdd status\",\"status shows sample3\",true\n",
            "  r404,orphan,\"\",\"a trigger\",\"an outcome\",true\n",
            "  r1,note,\"\",\"\",\"\",false\n",
        ),
    )
    .expect("write sample3 spec.toon");
    std::fs::write(
        spec_dir.join("legacy-acc.feature"),
        concat!(
            "# language: en\n",
            "Feature: sample3 legacy acceptance\n",
            "  @req:r1 @executable\n",
            "  Scenario: legacy-acc\n",
            "    Given seeded\n",
            "    When noop\n",
            "    Then ok\n",
        ),
    )
    .expect("write sample3 legacy .feature");
}

/// BDD-on fixture whose acceptance `@req` points at a missing rule id.
#[given("已初始化含无效 @req 的 sdd 项目且 bdd 配置为 {mode}")]
fn given_sdd_project_bad_req(mode: String) {
    seed_bdd_project(&mode);
    let dir = fixture_cwd();
    let path = dir.join("llmanspec/specs/sample/sample.feature");
    let body = std::fs::read_to_string(&path).expect("read sample feature");
    let updated = body.replace("@req:r1 @executable", "@req:r999 @executable");
    std::fs::write(&path, updated).expect("write dangling @req feature");
}

#[given("项目中存在技能目录 {name}")]
fn given_skill_dir(name: String) {
    let dir = fixture_cwd();
    let skill_dir = dir
        .join(".agents/skills")
        .join(name.trim().trim_matches('"'));
    std::fs::create_dir_all(&skill_dir).expect("mkdir planted skill");
    std::fs::write(skill_dir.join("SKILL.md"), "planted\n").expect("write planted skill");
}

/// Plant a global config.yaml with one of three `skills` shapes into a fresh
/// temp dir, then point `LLMAN_CONFIG_DIR` at it. Used by config-schemas r125
/// executable scenarios (multi-repo / legacy-dir / missing-path). The temp dir
/// is owned by the world so it survives until the scenario's When/Then.
#[given("全局 config.yaml 含 {kind} skills 配置")]
fn given_global_skills_config(kind: String) {
    reset_world();
    let temp = TempDir::new().expect("create skills-config tempdir");
    let dir = temp.path().to_path_buf();
    let skills_yaml = match kind.trim() {
        "multi-repo" => {
            "skills:\n  repo:\n    - name: Team\n      path: /tmp/team-skills\n    - path: /tmp/personal-skills\n"
                .to_string()
        }
        "legacy-dir" => "skills:\n  dir: /tmp/skills\n".to_string(),
        "missing-path" => {
            // One present dir so resolve still succeeds; one missing to trigger warn+filter.
            let present = dir.join("present-skills");
            std::fs::create_dir_all(&present).expect("create present skills dir");
            let missing = dir.join("missing-skills");
            format!(
                "skills:\n  repo:\n    - name: gone\n      path: {}\n    - name: ok\n      path: {}\n",
                missing.display(),
                present.display()
            )
        }
        other => panic!("unknown skills config kind: {other}"),
    };
    let config = format!("version: \"0.1\"\ntools: {{}}\n{skills_yaml}");
    std::fs::write(dir.join("config.yaml"), config).expect("write global config");

    WORLD.with(|w| {
        let mut guard = w.borrow_mut();
        let world = guard.as_mut().expect("world not initialized");
        world.fixture_dir = Some(temp);
        world.env_overrides.insert(
            "LLMAN_CONFIG_DIR".to_string(),
            dir.to_string_lossy().to_string(),
        );
    });
}

#[given("项目 extra_skills 包含 {name}")]
fn given_extra_skills(name: String) {
    let dir = fixture_cwd();
    let skill = name.trim().trim_matches('"');
    let config_path = dir.join("llmanspec/config.yaml");
    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
    // Preserve an existing `bdd:` block so skill bdd_mode stays consistent (r95).
    let bdd_tail = existing
        .find("\nbdd:")
        .map(|i| existing[i + 1..].to_string())
        .or_else(|| {
            if existing.starts_with("bdd:") {
                Some(existing.clone())
            } else {
                None
            }
        });
    let mut config = format!("schema: spec-driven\nlocale: en\nextra_skills:\n  - {skill}\n");
    if let Some(bdd) = bdd_tail {
        // Keep only the bdd section if present at start of remainder.
        if let Some(rest) = bdd.strip_prefix("bdd:") {
            config.push_str("\nbdd:");
            config.push_str(rest);
        } else if bdd.starts_with("bdd:") {
            config.push('\n');
            config.push_str(&bdd);
        }
    }
    std::fs::write(&config_path, config).expect("write extra_skills config");
    // Refresh managed skills so optional skill is installed and bdd_mode matches.
    run_llman_in(&dir, "sdd init --update", &[]);
}

/// Seed a project with a change directory carrying proposal+design+tasks, and
/// optionally a Git-native attach binding in proposal frontmatter. Used to
/// exercise `determine_stage` under BDD-on (r93): `attached = "yes"` writes
/// non-empty `branch` + `base_sha`; any other value omits them.
///
/// `{change}` is the change id (used as the branch name when attached). The
/// fixture must be combined with `已初始化 sdd 项目且 bdd 配置为 {mode}` first to
/// establish config + git base ref.
#[given("变更 {change} 含 proposal design tasks 且 attach 状态为 {attached}")]
fn given_change_with_artifacts_and_attach(change: String, attached: String) {
    let dir = fixture_cwd();
    let change_dir = dir.join("llmanspec/changes").join(&change);
    std::fs::create_dir_all(&change_dir).expect("mkdir attach-stage fixture change");
    let attach_flag = attached.trim().trim_matches('"');
    let frontmatter = match attach_flag {
        "yes" | "true" | "attached" | "on" => {
            format!(
                "---\ndepends_on: []\nbranch: feat/{change}\nbase_sha: 0000000000000000000000000000000000000000\n---\n"
            )
        }
        "skip" => {
            format!(
                "---\ndepends_on: []\nbranch: feat/{change}\nbase_sha: 0000000000000000000000000000000000000000\nskip_specs_landing: true\n---\n"
            )
        }
        _ => "---\ndepends_on: []\n---\n".to_string(),
    };
    // `parse_change` (used by `show`) requires both `## Why` and `## What Changes`.
    let proposal = format!(
        "{frontmatter}\n## Why\nr93 stage fixture.\n\n## What Changes\n- Probe determine_stage.\n"
    );
    std::fs::write(change_dir.join("proposal.md"), proposal).expect("write fixture proposal");
    std::fs::write(change_dir.join("design.md"), "# Design\nr93 fixture.\n").expect("write design");
    std::fs::write(change_dir.join("tasks.md"), "- [ ] t1\n").expect("write tasks");
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

#[when("用前缀 c123 运行 llman {args}")]
fn when_run_llman_prefix_c123(args: String) {
    run_llman(&args);
}

#[when("用前缀运行 llman {args}")]
fn when_run_llman_any_prefix(args: String) {
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
            "expected zero exit code, got failure (exit {:?})\nstdout:\n{}\nstderr:\n{}",
            w.exit_code, w.stdout, w.stderr
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

#[then("对应的完整 change 被找到且输出正确")]
fn then_prefix_resolved_correctly() {
    // The prefix-resolved change appears in the human-readable output and the
    // run succeeded (exact match or prefix resolution found exactly one change).
    with_world(|w| {
        assert!(
            w.success,
            "prefix run should succeed, exit {:?}",
            w.exit_code
        );
        let combined = format!("{}\n{}", w.stdout, w.stderr);
        assert!(
            combined.contains("c123-fix-bug"),
            "expected output to mention the resolved change, got: {combined}"
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

#[then("stdout 为合法 JSON 且含 JSON 键 {key}")]
fn then_stdout_is_json_with_key(key: String) {
    with_world(|w| {
        let v: serde_json::Value = serde_json::from_str(&w.stdout)
            .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{}", w.stdout));
        assert!(
            v.get(&key).is_some(),
            "JSON key `{key}` missing\n{}",
            w.stdout
        );
    });
}

#[then("stdout 的 JSON 键 {key} 为数字")]
fn then_stdout_json_key_is_number(key: String) {
    with_world(|w| {
        let v: serde_json::Value = serde_json::from_str(&w.stdout)
            .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{}", w.stdout));
        let mut cur = &v;
        for part in key.split('.') {
            cur = cur
                .get(part)
                .unwrap_or_else(|| panic!("JSON key `{key}` missing\n{}", w.stdout));
        }
        assert!(
            cur.is_number(),
            "JSON key `{key}` is not a number\n{}",
            w.stdout
        );
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

/// Assert a top-level stdout JSON key equals a string value.
/// rstest-bdd captures quoted placeholders verbatim, so `{value}` may arrive as
/// `"full"` — surrounding quotes are stripped before comparison.
#[then("stdout 的 JSON 键 {key} 为 {value}")]
fn then_stdout_json_key_equals(key: String, value: String) {
    with_world(|w| {
        let parsed: serde_json::Value = serde_json::from_str(&w.stdout)
            .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{}", w.stdout));
        let actual = parsed
            .get(&key)
            .unwrap_or_else(|| panic!("stdout JSON missing key {key:?}; got: {parsed}"));
        // Normalize both sides to JSON string form for comparison.
        let actual_str = match actual {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let expected = value.trim().trim_matches('"').to_string();
        assert!(
            actual_str == expected,
            "expected stdout JSON {key:?} = {expected:?}, got {actual_str:?}"
        );
    });
}

#[then("相对路径 {rel} 存在")]
fn then_rel_path_exists(rel: String) {
    let path = fixture_cwd().join(rel.trim().trim_matches('"'));
    assert!(path.exists(), "expected path to exist: {}", path.display());
}

#[then("相对路径 {rel} 行数为 {n:usize}")]
fn then_rel_path_line_count(rel: String, n: usize) {
    let path = fixture_cwd().join(rel.trim().trim_matches('"'));
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let lines = content.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(
        lines,
        n,
        "expected {n} non-empty lines in {}, got {lines}: {content:?}",
        path.display()
    );
}

#[then("相对路径 {rel} 不存在")]
fn then_rel_path_absent(rel: String) {
    let path = fixture_cwd().join(rel.trim().trim_matches('"'));
    assert!(
        !path.exists(),
        "expected path to be absent: {}",
        path.display()
    );
}

#[then("相对路径 {rel} 内容包含 {text}")]
fn then_rel_path_contains(rel: String, text: String) {
    let path = fixture_cwd().join(rel.trim().trim_matches('"'));
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let needle = text.trim().trim_matches('"');
    assert!(
        content.contains(needle),
        "expected {} to contain {:?}, got: {:?}",
        path.display(),
        needle,
        content
    );
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
// Scenario discovery — compile-time directory binding (Git-native BDD-on).
// Tag full-mode / CLI-drivable scenarios with `@executable`. Documentation-only
// features under llmanspec/specs remain untagged and are not expanded into tests.
// ---------------------------------------------------------------------------

scenarios!("llmanspec/specs", tags = "@executable");
