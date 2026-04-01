use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::{SpecStyle, load_required_config};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::backend::yaml_overlay::{
    YamlWriteBackMode, update_main_spec_markdown_with_overlay_or_fallback,
};
use crate::sdd::spec::backend::{DumpOptions, backend_for_style};
use crate::sdd::spec::fence::render_code_fence;
use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
use crate::sdd::spec::ison::{compose_with_frontmatter, split_frontmatter};
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

pub fn run_skeleton(root: &Path, args: SpecSkeletonArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    if args.pretty_ison && config.spec_style != SpecStyle::Ison {
        return Err(anyhow!(t!(
            "sdd.style.pretty_ison_requires_ison",
            style = config.spec_style.as_str()
        )));
    }

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
    let backend = backend_for_style(config.spec_style);
    let spec = MainSpecDoc {
        kind: "llman.sdd.spec".to_string(),
        name: args.capability.clone(),
        purpose: "TODO: Describe this capability and its purpose.".to_string(),
        requirements: Vec::new(),
        scenarios: Vec::new(),
    };
    let payload = backend.dump_main_spec(
        &spec,
        DumpOptions {
            pretty_ison: args.pretty_ison,
        },
    )?;
    let body = render_code_fence(config.spec_style.as_str(), &payload);
    let rebuilt = compose_with_frontmatter(Some(&frontmatter), &body);

    atomic_write_with_mode(&spec_path, rebuilt.as_bytes(), None)?;
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_requirement(root: &Path, args: SpecAddRequirementArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    if args.pretty_ison && config.spec_style != SpecStyle::Ison {
        return Err(anyhow!(t!(
            "sdd.style.pretty_ison_requires_ison",
            style = config.spec_style.as_str()
        )));
    }
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
    let backend = backend_for_style(config.spec_style);
    let old_doc = backend.parse_main_spec(&body, &context)?;
    let mut spec = old_doc.clone();

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

    let payload = backend.dump_main_spec(
        &spec,
        DumpOptions {
            pretty_ison: args.pretty_ison,
        },
    )?;

    if config.spec_style == SpecStyle::Yaml {
        let writeback = update_main_spec_markdown_with_overlay_or_fallback(
            &content, &old_doc, &spec, &payload,
        )?;
        if writeback.mode == YamlWriteBackMode::FencedRewrite {
            eprintln!(
                "{}",
                t!(
                    "sdd.yaml.lossless_fallback",
                    path = display_llmanspec_path(&spec_path)
                )
            );
        }
        atomic_write_with_mode(&spec_path, writeback.content.as_bytes(), None)?;
    } else {
        let body = render_code_fence(config.spec_style.as_str(), &payload);
        let rebuilt = compose_with_frontmatter(Some(&frontmatter_yaml), &body);
        atomic_write_with_mode(&spec_path, rebuilt.as_bytes(), None)?;
    }
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_scenario(root: &Path, args: SpecAddScenarioArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    validate_sdd_id(&args.scenario_id, "scenario")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    if args.pretty_ison && config.spec_style != SpecStyle::Ison {
        return Err(anyhow!(t!(
            "sdd.style.pretty_ison_requires_ison",
            style = config.spec_style.as_str()
        )));
    }
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
    let backend = backend_for_style(config.spec_style);
    let old_doc = backend.parse_main_spec(&body, &context)?;
    let mut spec = old_doc.clone();

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

    let payload = backend.dump_main_spec(
        &spec,
        DumpOptions {
            pretty_ison: args.pretty_ison,
        },
    )?;
    if config.spec_style == SpecStyle::Yaml {
        let writeback = update_main_spec_markdown_with_overlay_or_fallback(
            &content, &old_doc, &spec, &payload,
        )?;
        if writeback.mode == YamlWriteBackMode::FencedRewrite {
            eprintln!(
                "{}",
                t!(
                    "sdd.yaml.lossless_fallback",
                    path = display_llmanspec_path(&spec_path)
                )
            );
        }
        atomic_write_with_mode(&spec_path, writeback.content.as_bytes(), None)?;
    } else {
        let body = render_code_fence(config.spec_style.as_str(), &payload);
        let rebuilt = compose_with_frontmatter(Some(&frontmatter_yaml), &body);
        atomic_write_with_mode(&spec_path, rebuilt.as_bytes(), None)?;
    }
    println!("{}", spec_path.display());
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

fn display_llmanspec_path(path: &Path) -> String {
    let display = path.display().to_string();
    if let Some(idx) = display.find(LLMANSPEC_DIR_NAME) {
        return display[idx..].to_string();
    }
    display
}
