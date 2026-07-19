//! BDD on/off 兼容性测试 —— **实现细节层**。
//!
//! 行为合约（validate --check 语义、Git-native attach/checkpoint、index embed、feature 忽略）
//! 已固化在 `llmanspec/specs/sdd-bdd-mode-compat/*.feature` + `tests/bdd_steps.rs`
//! （`cargo test --features bdd`）。本文件只保留不依赖 LLM、且属于内部实现/兼容性
//! 的断言：init 结构等价、旧 tree.json 向后兼容（serde default）、子命令 smoke、
//! feature-branch attach/checkpoint/archive docs-only。
//!
//! 维护规则见 AGENTS.md「BDD 模式兼容性测试维护规则」。

mod common;

use common::{TestEnvironment, assert_success, llman_command};
use llman::sdd::context::tree::TreeIndex;
use std::fs;
use std::process::Command;

// ── helpers ──────────────────────────────────────────────────────────────────

fn run(args: &[&str], env: &TestEnvironment) -> std::process::Output {
    let mut cmd = llman_command(&env.work_dir);
    cmd.args(args).current_dir(&env.work_dir);
    if let Some(base_ref) = common::git_head(&env.work_dir) {
        cmd.env("LLMANSPEC_BASE_REF", base_ref);
    }
    cmd.output().expect("run llman")
}

const BDD_ON_BLOCK: &str = "bdd:\n  run_command: \"cargo test --features bdd\"";

/// Seed a spec `sample` (r1 + scenario) and an `add-scen` change (delta adds r2).
fn seed_spec_and_change(env: &TestEnvironment) {
    assert_success(&run(&["sdd", "spec", "skeleton", "sample"], env));
    assert_success(&run(
        &[
            "sdd",
            "spec",
            "add-requirement",
            "sample",
            "r1",
            "--title",
            "R1",
            "--statement",
            "System MUST do X.",
        ],
        env,
    ));
    assert_success(&run(
        &[
            "sdd",
            "spec",
            "add-scenario",
            "sample",
            "r1",
            "happy",
            "--when",
            "trigger",
            "--then",
            "outcome",
        ],
        env,
    ));
    let change_dir = env.work_dir.join("llmanspec/changes/add-scen");
    fs::create_dir_all(&change_dir).expect("mkdir change");
    fs::write(
        change_dir.join("proposal.md"),
        "## Why\nAdd r2 to sample.\n\n## What Changes\n- Add requirement r2.\n",
    )
    .expect("write proposal");
    assert_success(&run(
        &["sdd", "change", "delta", "skeleton", "add-scen", "sample"],
        env,
    ));
    assert_success(&run(
        &[
            "sdd",
            "change",
            "delta",
            "add-req",
            "add-scen",
            "sample",
            "r2",
            "--title",
            "R2",
            "--statement",
            "System MUST support r2.",
        ],
        env,
    ));
    assert_success(&run(
        &[
            "sdd",
            "change",
            "delta",
            "add-scenario",
            "add-scen",
            "sample",
            "r2",
            "new r2 behavior",
            "--when",
            "r2 triggered",
            "--then",
            "r2 works",
        ],
        env,
    ));
}

fn write_config(env: &TestEnvironment, bdd: Option<&str>) {
    let mut content = "schema: spec-driven\nlocale: en\n".to_string();
    if let Some(block) = bdd {
        content.push('\n');
        content.push_str(block);
        content.push('\n');
    }
    fs::write(env.work_dir.join("llmanspec/config.yaml"), content).expect("write config.yaml");
}

fn init_project(env: &TestEnvironment, bdd: Option<&str>) {
    assert_success(&run(&["sdd", "init", "--lang", "en"], env));
    // Author TOON deltas while still BDD-off (delta is rejected under BDD-on).
    write_config(env, None);
    seed_spec_and_change(env);
    write_config(env, bdd);
    // Refresh skills so metadata.llman_sdd.bdd_mode matches final config (r95).
    assert_success(&run(&["sdd", "init", "--update"], env));
}

fn git(env: &TestEnvironment, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(&env.work_dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"))
}

fn commit(env: &TestEnvironment, msg: &str) {
    assert_success(&git(env, &["add", "."]));
    assert_success(
        &Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@x",
                "commit",
                "-qm",
                msg,
            ])
            .current_dir(&env.work_dir)
            .output()
            .expect("git commit"),
    );
}

fn seed_live_feature(env: &TestEnvironment) {
    fs::write(
        env.work_dir.join("llmanspec/specs/sample/sample.feature"),
        "# language: en\nFeature: sample\n  @req:r1\n  Scenario: harness-happy\n    Given a\n    When b\n    Then c\n",
    )
    .expect("write feature");
    fs::write(
        env.work_dir.join("llmanspec/specs/sample/spec.toon"),
        r#"kind: llman.sdd.spec
name: "sample"
purpose: "sample"
valid_scope[1]: "llmanspec/specs/sample"
requirements[1]{req_id,title,statement}:
  r1,R1,"System MUST do X."
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,happy,"","trigger","outcome",false
"#,
    )
    .expect("rewrite toon");
}

// ── 实现细节：init 在两种 config 下结构等价 ──────────────────────────────────

#[test]
fn test_init_structure_identical_bdd_on_and_off() {
    for bdd in [Some(BDD_ON_BLOCK), None] {
        let env = TestEnvironment::new();
        assert_success(&run(&["sdd", "init", "--lang", "en"], &env));
        write_config(&env, bdd);
        assert!(env.work_dir.join("llmanspec/config.yaml").exists());
        assert!(env.work_dir.join("llmanspec/AGENTS.md").exists());
        assert!(env.work_dir.join("llmanspec/specs/.gitkeep").exists());
    }
}

// ── 实现细节：旧 tree.json 向后兼容（#[serde(default)]）──────────────────────

#[test]
fn test_index_rebuild_backward_compat_old_tree_loads() {
    // 旧 tree.json 缺少 `scenarios` 字段（feat-sdd-context-bdd-aware 之前），
    // 仍必须能反序列化——scenarios 默认为空。这是内部 serde 兼容性，非用户合约。
    let env = TestEnvironment::new();
    init_project(&env, None);

    let ctx_dir = env.work_dir.join("llmanspec/.context/pageindex");
    fs::create_dir_all(&ctx_dir).expect("mkdir context");
    let old_tree = r#"{
      "version": 1,
      "spec_hash": "legacy",
      "build_timestamp": "2020-01-01T00:00:00+00:00",
      "chat_model": "",
      "docs": [
        {"spec_id": "sample", "purpose": "legacy", "reqs": []}
      ]
    }"#;
    fs::write(ctx_dir.join("tree.json"), old_tree).expect("write legacy tree");

    let tree = TreeIndex::load(&ctx_dir).expect("legacy tree.json must load");
    assert_eq!(tree.docs.len(), 1);
    assert!(tree.docs[0].scenarios.is_empty());
}

// ── 实现细节：子命令 smoke 兜底（两种 config 都不崩）──────────────────────

#[test]
fn test_all_subcommands_smoke_bdd_on_and_off() {
    let read_only: &[&[&str]] = &[
        &["sdd", "list", "--specs", "--json", "--no-interactive"],
        &["sdd", "list", "--changes", "--json", "--no-interactive"],
        &["sdd", "show", "sample", "--no-interactive"],
        &[
            "sdd",
            "show",
            "sample",
            "--output",
            "json",
            "--no-interactive",
        ],
        &[
            "sdd",
            "validate",
            "sample",
            "--strict",
            "--no-check",
            "--no-interactive",
        ],
        &[
            "sdd",
            "validate",
            "--specs",
            "--strict",
            "--no-check",
            "--no-interactive",
        ],
        &["sdd", "index", "rebuild"],
        &["sdd", "index", "check"],
        &["sdd", "graph"],
        &["sdd", "status"],
        &["sdd", "project", "migrate", "--dry-run"],
        &[
            "sdd",
            "project",
            "migrate",
            "--kind",
            "partitioned",
            "--dry-run",
        ],
        &["sdd", "project", "dedupe-req-ids", "--dry-run"],
        &["sdd", "project", "upgrade-guide"],
        &["sdd", "spec", "next-req-id", "--json"],
        &["sdd", "spec", "resolve-req", "r1", "--json"],
    ];
    for bdd in [Some(BDD_ON_BLOCK), None] {
        let env = TestEnvironment::new();
        init_project(&env, bdd);
        if bdd.is_some() {
            seed_live_feature(&env);
        }
        commit(&env, "seed");
        for args in read_only {
            let out = run(args, &env);
            if !out.status.success() {
                panic!(
                    "smoke command failed (bdd_on={}) `{}`:\n--- stdout ---\n{}\n--- stderr ---\n{}",
                    bdd.is_some(),
                    args.join(" "),
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr),
                );
            }
        }
        // solidify must not exist
        let solidify = run(&["sdd", "solidify", "add-scen"], &env);
        assert!(!solidify.status.success(), "sdd solidify must be removed");

        // BDD-on rejects change delta
        if bdd.is_some() {
            let delta = run(
                &["sdd", "change", "delta", "skeleton", "add-scen", "sample"],
                &env,
            );
            assert!(!delta.status.success(), "change delta must reject BDD-on");
        }

        // change finalize exists and parses. Both BDD-on (default branch / not
        // attached) and BDD-off paths reject with non-zero exit, but the
        // command must be recognized — i.e. no `unrecognized subcommand`.
        {
            let finalize = run(&["sdd", "change", "finalize", "add-scen"], &env);
            assert!(
                !finalize.status.success(),
                "change finalize should reject without attach/BDD-on setup"
            );
            let stderr = String::from_utf8_lossy(&finalize.stderr);
            assert!(
                !stderr.contains("unrecognized subcommand"),
                "change finalize must exist (got stderr: {stderr})"
            );
        }

        // archive dry-run: BDD-off works without git binding; BDD-on needs attach/checkpoint
        if bdd.is_none() {
            assert_success(&run(
                &["sdd", "change", "archive", "--dry-run", "add-scen"],
                &env,
            ));
        }
    }
}

#[test]
fn test_bdd_on_attach_checkpoint_archive_docs_only() {
    let env = TestEnvironment::new();
    init_project(&env, Some(BDD_ON_BLOCK));
    seed_live_feature(&env);
    commit(&env, "seed");

    // Default branch attach must fail.
    let attach_main = run(&["sdd", "change", "attach", "add-scen"], &env);
    assert!(!attach_main.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&attach_main.stdout),
        String::from_utf8_lossy(&attach_main.stderr)
    );
    assert!(
        err.to_ascii_lowercase().contains("default branch"),
        "expected default-branch rejection, got: {err}"
    );

    assert_success(&git(&env, &["checkout", "-b", "feat/add-scen"]));
    assert_success(&run(&["sdd", "change", "attach", "add-scen"], &env));
    commit(&env, "attach binding");

    // Dirty tree blocks checkpoint.
    fs::write(env.work_dir.join("dirty.txt"), "x").unwrap();
    let dirty = run(
        &["sdd", "change", "checkpoint", "add-scen", "--no-check"],
        &env,
    );
    assert!(!dirty.status.success());
    fs::remove_file(env.work_dir.join("dirty.txt")).unwrap();

    assert_success(&run(
        &["sdd", "change", "checkpoint", "add-scen", "--no-check"],
        &env,
    ));
    // Checkpoint updates proposal frontmatter — commit so archive sees a clean tree.
    commit(&env, "checkpoint");

    // Diff is read-only and non-empty after attach (may be empty if no commits since base).
    let _ = run(&["sdd", "change", "diff", "add-scen"], &env);

    // Archive moves docs only; live feature remains.
    assert_success(&run(&["sdd", "change", "archive", "add-scen"], &env));
    assert!(
        env.work_dir
            .join("llmanspec/specs/sample/sample.feature")
            .exists(),
        "live feature must remain after docs-only archive"
    );
    assert!(
        !env.work_dir.join("llmanspec/changes/add-scen").exists(),
        "active change dir must be moved"
    );
    let archived = fs::read_dir(env.work_dir.join("llmanspec/changes/archive"))
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().contains("add-scen"));
    assert!(archived, "change docs must land in archive/");
}

#[test]
fn test_bdd_on_rejects_legacy_feature_delta_on_validate() {
    let env = TestEnvironment::new();
    init_project(&env, Some(BDD_ON_BLOCK));
    seed_live_feature(&env);
    let delta_dir = env.work_dir.join("llmanspec/changes/add-scen/specs/sample");
    fs::create_dir_all(&delta_dir).unwrap();
    fs::write(
        delta_dir.join("sample.feature.delta.toon"),
        "kind: llman.sdd.feature_delta\nops[0]:\n",
    )
    .unwrap();
    commit(&env, "seed with legacy delta");
    assert_success(&git(&env, &["checkout", "-b", "feat/legacy"]));
    assert_success(&run(&["sdd", "change", "attach", "add-scen"], &env));
    let out = run(
        &[
            "sdd",
            "validate",
            "add-scen",
            "--type",
            "change",
            "--strict",
            "--no-check",
            "--no-interactive",
        ],
        &env,
    );
    assert!(!out.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        err.contains("feature_delta") || err.contains("migration blocker"),
        "expected legacy feature_delta error, got: {err}"
    );
}

#[test]
fn test_bdd_off_attach_unavailable() {
    let env = TestEnvironment::new();
    init_project(&env, None);
    commit(&env, "seed");
    let out = run(&["sdd", "change", "attach", "add-scen"], &env);
    assert!(!out.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        err.contains("BDD-on") || err.contains("bdd:"),
        "expected BDD-on requirement, got: {err}"
    );
}
