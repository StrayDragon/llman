use crate::sdd::project::templates::TemplateStyle;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::ison::{compose_with_frontmatter, split_frontmatter};
use crate::sdd::spec::ison_table::render_ison_fence;
use crate::sdd::spec::ison_v1::{CanonicalSpec, RequirementRow, ScenarioRow, SpecMeta};
use crate::sdd::spec::ison_v1::{SPEC_KIND, dump_spec_payload, parse_spec_body};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SpecSkeletonArgs {
    pub capability: String,
    pub force: bool,
    pub pretty_ison: bool,
}

#[derive(Debug, Clone)]
pub struct SpecAddRequirementArgs {
    pub capability: String,
    pub req_id: String,
    pub title: String,
    pub statement: String,
    pub pretty_ison: bool,
}

#[derive(Debug, Clone)]
pub struct SpecAddScenarioArgs {
    pub capability: String,
    pub req_id: String,
    pub scenario_id: String,
    pub given: String,
    pub when_: String,
    pub then_: String,
    pub pretty_ison: bool,
}

pub fn run_skeleton(root: &Path, args: SpecSkeletonArgs, style: TemplateStyle) -> Result<()> {
    ensure_new_style(style)?;
    validate_sdd_id(&args.capability, "spec")?;

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

    let frontmatter = default_spec_frontmatter_yaml();
    let spec = CanonicalSpec {
        meta: SpecMeta {
            kind: SPEC_KIND.to_string(),
            name: args.capability.clone(),
            purpose: "TODO: Describe this capability and its purpose.".to_string(),
        },
        requirements: Vec::new(),
        scenarios: Vec::new(),
    };
    let payload = dump_spec_payload(&spec, args.pretty_ison);
    let body = render_ison_fence(&payload);
    let rebuilt = compose_with_frontmatter(Some(&frontmatter), &body);

    fs::write(&spec_path, rebuilt)?;
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_requirement(
    root: &Path,
    args: SpecAddRequirementArgs,
    style: TemplateStyle,
) -> Result<()> {
    ensure_new_style(style)?;
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
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
    let (frontmatter_yaml, body) = split_frontmatter(&content);
    let Some(frontmatter_yaml) = frontmatter_yaml else {
        return Err(anyhow!(
            "spec is missing YAML frontmatter: {} (run `llman sdd spec skeleton {}` to initialize)",
            spec_path.display(),
            args.capability
        ));
    };

    let context = format!("spec `{}`", args.capability);
    let mut spec = parse_spec_body(&body, &context)?;

    spec.meta.kind = SPEC_KIND.to_string();
    spec.meta.name = args.capability.clone();

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

    spec.requirements.push(RequirementRow {
        req_id: args.req_id.trim().to_string(),
        title: args.title.trim().to_string(),
        statement: args.statement.trim().to_string(),
    });

    spec.scenarios.push(ScenarioRow {
        req_id: args.req_id.trim().to_string(),
        id: "baseline".to_string(),
        given: "".to_string(),
        when: "TODO: describe the trigger".to_string(),
        then: "TODO: describe the expected result".to_string(),
    });

    let payload = dump_spec_payload(&spec, args.pretty_ison);
    let body = render_ison_fence(&payload);
    let rebuilt = compose_with_frontmatter(Some(&frontmatter_yaml), &body);
    fs::write(&spec_path, rebuilt)?;
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_scenario(
    root: &Path,
    args: SpecAddScenarioArgs,
    style: TemplateStyle,
) -> Result<()> {
    ensure_new_style(style)?;
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    validate_sdd_id(&args.scenario_id, "scenario")?;
    if args.when_.trim().is_empty() {
        return Err(anyhow!("--when must not be empty"));
    }
    if args.then_.trim().is_empty() {
        return Err(anyhow!("--then must not be empty"));
    }

    let spec_path = spec_path(root, &args.capability);
    let content = fs::read_to_string(&spec_path)
        .map_err(|err| anyhow!("failed to read spec: {} ({})", spec_path.display(), err))?;
    let (frontmatter_yaml, body) = split_frontmatter(&content);
    let Some(frontmatter_yaml) = frontmatter_yaml else {
        return Err(anyhow!(
            "spec is missing YAML frontmatter: {} (run `llman sdd spec skeleton {}` to initialize)",
            spec_path.display(),
            args.capability
        ));
    };

    let context = format!("spec `{}`", args.capability);
    let mut spec = parse_spec_body(&body, &context)?;

    spec.meta.kind = SPEC_KIND.to_string();
    spec.meta.name = args.capability.clone();

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

    spec.scenarios.push(ScenarioRow {
        req_id: args.req_id.trim().to_string(),
        id: args.scenario_id.trim().to_string(),
        given: args.given.trim().to_string(),
        when: args.when_.trim().to_string(),
        then: args.then_.trim().to_string(),
    });

    let payload = dump_spec_payload(&spec, args.pretty_ison);
    let body = render_ison_fence(&payload);
    let rebuilt = compose_with_frontmatter(Some(&frontmatter_yaml), &body);
    fs::write(&spec_path, rebuilt)?;
    println!("{}", spec_path.display());
    Ok(())
}

fn ensure_new_style(style: TemplateStyle) -> Result<()> {
    if style != TemplateStyle::New {
        return Err(anyhow!(
            "`llman sdd-legacy` does not support canonical table/object ISON authoring helpers; use `llman sdd spec ...`"
        ));
    }
    Ok(())
}

fn spec_path(root: &Path, capability: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("specs")
        .join(capability)
        .join("spec.md")
}

fn default_spec_frontmatter_yaml() -> String {
    [
        "llman_spec_valid_scope:",
        "  - src/",
        "  - tests/",
        "llman_spec_valid_commands:",
        "  - cargo test",
        "llman_spec_evidence:",
        "  - \"TODO: add evidence (CI link, benchmark output, etc.)\"",
    ]
    .join("\n")
}

fn contains_shall_or_must(text: &str) -> bool {
    text.contains("SHALL") || text.contains("MUST")
}
