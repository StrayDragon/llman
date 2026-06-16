use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SpecSkeletonArgs {
    pub capability: String,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct SpecAddRequirementArgs {
    pub capability: String,
    pub req_id: String,
    pub title: String,
    pub statement: String,
}

#[derive(Debug, Clone)]
pub struct SpecAddScenarioArgs {
    pub capability: String,
    pub req_id: String,
    pub scenario_id: String,
    pub given: String,
    pub when_: String,
    pub then_: String,
}

pub fn run_skeleton(root: &Path, args: SpecSkeletonArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;

    let spec_path = spec_path(root, &args.capability);
    if spec_path.exists() && !args.force {
        return Err(anyhow!(
            "spec skeleton target already exists: {} (pass --force to overwrite)",
            spec_path.display()
        ));
    }
    if let Some(parent) = spec_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Standalone `.toon` file: validation proof-metadata lives inside the TOON
    // document (valid_scope / valid_commands / evidence), not a YAML frontmatter.
    let spec = MainSpecDoc {
        kind: "llman.sdd.spec".to_string(),
        name: args.capability.clone(),
        purpose: "TODO: Describe this capability and its purpose.".to_string(),
        valid_scope: vec!["src/".to_string(), "tests/".to_string()],
        valid_commands: vec!["cargo test".to_string()],
        evidence: vec!["TODO: add evidence (CI link, benchmark output, etc.)".to_string()],
        requirements: Vec::new(),
        scenarios: Vec::new(),
        feature_refs: None,
    };
    let payload = BACKEND.dump_main_spec(&spec)?;

    atomic_write_with_mode(&spec_path, payload.as_bytes(), None)?;
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_requirement(root: &Path, args: SpecAddRequirementArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;
    if args.title.trim().is_empty() {
        return Err(anyhow!("title must not be empty"));
    }
    if args.statement.trim().is_empty() {
        return Err(anyhow!("statement must not be empty"));
    }
    if !contains_shall_or_must(args.statement.trim()) {
        return Err(anyhow!("statement must contain MUST or SHALL"));
    }

    let spec_path = spec_path(root, &args.capability);
    let content = fs::read_to_string(&spec_path)
        .map_err(|err| anyhow!("failed to read spec: {} ({})", spec_path.display(), err))?;

    let context = format!("spec `{}`", args.capability);
    let mut spec = BACKEND.parse_main_spec(&content, &context)?;
    spec.kind = "llman.sdd.spec".to_string();
    spec.name = args.capability.clone();

    if spec
        .requirements
        .iter()
        .any(|row| row.req_id.trim() == args.req_id.trim())
    {
        return Err(anyhow!(
            "{context}: requirement already exists: `{}`",
            args.req_id
        ));
    }

    spec.requirements.push(RequirementEntry {
        req_id: args.req_id.trim().to_string(),
        title: args.title.trim().to_string(),
        statement: args.statement.trim().to_string(),
    });

    spec.scenarios.push(ScenarioEntry {
        req_id: args.req_id.trim().to_string(),
        id: "baseline".to_string(),
        given: "".to_string(),
        when_: "TODO: describe the trigger".to_string(),
        then_: "TODO: describe the expected result".to_string(),
    });

    let payload = BACKEND.dump_main_spec(&spec)?;
    atomic_write_with_mode(&spec_path, payload.as_bytes(), None)?;
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_scenario(root: &Path, args: SpecAddScenarioArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    validate_sdd_id(&args.scenario_id, "scenario")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;
    if args.when_.trim().is_empty() {
        return Err(anyhow!("--when must not be empty"));
    }
    if args.then_.trim().is_empty() {
        return Err(anyhow!("--then must not be empty"));
    }

    let spec_path = spec_path(root, &args.capability);
    let content = fs::read_to_string(&spec_path)
        .map_err(|err| anyhow!("failed to read spec: {} ({})", spec_path.display(), err))?;

    let context = format!("spec `{}`", args.capability);
    let mut spec = BACKEND.parse_main_spec(&content, &context)?;
    spec.kind = "llman.sdd.spec".to_string();
    spec.name = args.capability.clone();

    if !spec
        .requirements
        .iter()
        .any(|row| row.req_id.trim() == args.req_id.trim())
    {
        return Err(anyhow!(
            "{context}: unknown requirement `req_id` `{}`",
            args.req_id
        ));
    }

    if spec.scenarios.iter().any(|row| {
        row.req_id.trim() == args.req_id.trim() && row.id.trim() == args.scenario_id.trim()
    }) {
        return Err(anyhow!(
            "{context}: scenario already exists: (req_id, id) = (`{}`, `{}`)",
            args.req_id,
            args.scenario_id
        ));
    }

    spec.scenarios.push(ScenarioEntry {
        req_id: args.req_id.trim().to_string(),
        id: args.scenario_id.trim().to_string(),
        given: args.given.trim().to_string(),
        when_: args.when_.trim().to_string(),
        then_: args.then_.trim().to_string(),
    });

    let payload = BACKEND.dump_main_spec(&spec)?;
    atomic_write_with_mode(&spec_path, payload.as_bytes(), None)?;
    println!("{}", spec_path.display());
    Ok(())
}

fn spec_path(root: &Path, capability: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("specs")
        .join(capability)
        .join(SPEC_FILE)
}

fn contains_shall_or_must(text: &str) -> bool {
    text.contains("SHALL") || text.contains("MUST")
}
