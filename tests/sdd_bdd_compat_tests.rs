//! BDD on/off 兼容性测试 —— **实现细节层**。
//!
//! 行为合约（validate --check 语义、solidify 模式开关、index embed、feature 忽略）
//! 已固化在 `llmanspec/specs/sdd-bdd-mode-compat/*.feature` + `tests/bdd_steps.rs`
//! （`cargo test --features bdd`）。本文件只保留不依赖 LLM、且属于内部实现/兼容性
//! 的断言：init 结构等价、旧 tree.json 向后兼容（serde default）、13 子命令 smoke。
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
        &["sdd", "delta", "skeleton", "add-scen", "sample"],
        env,
    ));
    assert_success(&run(
        &[
            "sdd",
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
    write_config(env, bdd);
}

fn commit(env: &TestEnvironment, msg: &str) {
    let ok = |o: std::process::Output| assert_success(&o);
    ok(Command::new("git")
        .args(["add", "."])
        .current_dir(&env.work_dir)
        .output()
        .expect("git add"));
    ok(Command::new("git")
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
        .expect("git commit"));
}

// ── 实现细节：init 在两种 config 下结构等价 ──────────────────────────────────

#[test]
fn test_init_structure_identical_bdd_on_and_off() {
    for bdd in [Some(BDD_ON_BLOCK), None] {
        let env = TestEnvironment::new();
        init_project(&env, bdd);
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
    seed_spec_and_change(&env);

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

// ── 实现细节：13 子命令 smoke 兜底（两种 config 都不崩）──────────────────────

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
        &["sdd", "archive", "run", "--dry-run", "add-scen"],
        &["sdd", "project", "migrate", "--dry-run"],
        &["sdd", "project", "partition-migrate", "--dry-run"],
        &["sdd", "project", "dedupe-req-ids", "--dry-run"],
        &["sdd", "project", "upgrade-guide"],
        &["sdd", "spec", "next-req-id", "--json"],
        &["sdd", "spec", "resolve-req", "r1", "--json"],
    ];
    for bdd in [Some(BDD_ON_BLOCK), None] {
        let env = TestEnvironment::new();
        init_project(&env, bdd);
        seed_spec_and_change(&env);
        if bdd.is_some() {
            assert_success(&run(&["sdd", "solidify", "add-scen"], &env));
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
    }
}
