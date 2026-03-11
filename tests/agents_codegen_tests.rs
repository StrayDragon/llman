#![cfg(unix)]

mod common;

use common::{assert_success, run_llman};
use std::fs;
use tempfile::TempDir;

#[test]
fn agents_gen_code_renders_agent_py_with_prompt_and_includes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");
    let out_dir = root.join("out");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    fs::write(
        skills_root.join("foo/SKILL.md"),
        "---\nname: foo\n---\n\nSYSTEM PROMPT BODY\n\n## Requirements\n\n- must\n",
    )
    .expect("write SKILL.md");
    fs::write(
        config_dir.join("agents/foo/agent.toml"),
        "version = 1\nid = \"foo\"\nincludes = [\"a\", \"b\"]\n\n[[skills]]\nid = \"a\"\npath = \"/tmp/a\"\n",
    )
    .expect("write manifest");

    let output = run_llman(
        &[
            "agents",
            "gen-code",
            "foo",
            "--framework",
            "pydantic-ai",
            "--out",
            out_dir.to_str().unwrap(),
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    let agent_py = out_dir.join("agent.py");
    let content = fs::read_to_string(&agent_py).expect("read agent.py");
    assert!(content.contains("SYSTEM PROMPT BODY"));
    assert!(content.contains("# - a"));
    assert!(content.contains("# - b"));
    assert!(content.contains("# - id=a path=/tmp/a"));
}

#[test]
fn agents_gen_code_fails_when_output_exists_without_force() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");
    let out_dir = root.join("out");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    fs::write(
        skills_root.join("foo/SKILL.md"),
        "---\nname: foo\n---\n\nprompt\n\n## Requirements\n\n- must\n",
    )
    .expect("write SKILL.md");

    fs::create_dir_all(&out_dir).expect("create out dir");
    fs::write(out_dir.join("agent.py"), "existing").expect("write existing");

    let output = run_llman(
        &[
            "agents",
            "gen-code",
            "foo",
            "--framework",
            "crewai",
            "--out",
            out_dir.to_str().unwrap(),
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert!(
        !output.status.success(),
        "expected failure, got success:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_llman(
        &[
            "agents",
            "gen-code",
            "foo",
            "--framework",
            "crewai",
            "--out",
            out_dir.to_str().unwrap(),
            "--force",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);
    let content = fs::read_to_string(out_dir.join("agent.py")).expect("read overwritten");
    assert!(content.contains("SYSTEM_PROMPT"));
}

#[test]
fn agents_gen_code_fails_when_manifest_missing() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let skills_root = root.join("skills");
    let out_dir = root.join("out");

    let output = run_llman(
        &[
            "agents",
            "new",
            "foo",
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert_success(&output);

    fs::remove_file(config_dir.join("agents/foo/agent.toml")).expect("remove manifest");

    let output = run_llman(
        &[
            "agents",
            "gen-code",
            "foo",
            "--framework",
            "pydantic-ai",
            "--out",
            out_dir.to_str().unwrap(),
            "--skills-dir",
            skills_root.to_str().unwrap(),
        ],
        root,
        &config_dir,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Missing agent manifest"));
}
