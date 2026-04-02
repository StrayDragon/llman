use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::{SpecStyle, load_required_config};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::backend::yaml_overlay::{
    YamlWriteBackMode, update_delta_spec_markdown_with_overlay_or_fallback,
};
use crate::sdd::spec::backend::{DumpOptions, backend_for_style};
use crate::sdd::spec::fence::render_code_fence;
use crate::sdd::spec::ir::{DeltaOpEntry, DeltaSpecDoc, ScenarioEntry};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DeltaSkeletonArgs {
    pub change_id: String,
    pub capability: String,
    pub force: bool,
    pub pretty_ison: bool,
}

#[derive(Debug, Clone)]
pub struct DeltaAddOpArgs {
    pub change_id: String,
    pub capability: String,
    pub op: String,
    pub req_id: String,
    pub title: Option<String>,
    pub statement: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub name: Option<String>,
    pub pretty_ison: bool,
}

#[derive(Debug, Clone)]
pub struct DeltaAddScenarioArgs {
    pub change_id: String,
    pub capability: String,
    pub req_id: String,
    pub scenario_id: String,
    pub given: String,
    pub when_: String,
    pub then_: String,
    pub pretty_ison: bool,
}

pub fn run_skeleton(root: &Path, args: DeltaSkeletonArgs) -> Result<()> {
    validate_sdd_id(&args.change_id, "change")?;
    validate_sdd_id(&args.capability, "spec")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec_dir)?;
    if args.pretty_ison && config.spec_style != SpecStyle::Ison {
        return Err(anyhow!(t!(
            "sdd.style.pretty_ison_requires_ison",
            style = config.spec_style.as_str()
        )));
    }

    let delta_path = delta_path(root, &args.change_id, &args.capability);
    if delta_path.exists() && !args.force {
        return Err(anyhow!(
            "delta spec skeleton target already exists: {} (pass --force to overwrite)",
            delta_path.display()
        ));
    }
    if let Some(parent) = delta_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let backend = backend_for_style(config.spec_style);
    let delta = DeltaSpecDoc {
        kind: "llman.sdd.delta".to_string(),
        ops: Vec::new(),
        op_scenarios: Vec::new(),
    };
    let payload = backend.dump_delta_spec(
        &delta,
        DumpOptions {
            pretty_ison: args.pretty_ison,
        },
    )?;
    let rebuilt = render_code_fence(config.spec_style.as_str(), &payload);
    atomic_write_with_mode(&delta_path, rebuilt.as_bytes(), None)?;
    println!("{}", delta_path.display());
    Ok(())
}

pub fn run_add_op(root: &Path, args: DeltaAddOpArgs) -> Result<()> {
    validate_sdd_id(&args.change_id, "change")?;
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

    let delta_path = delta_path(root, &args.change_id, &args.capability);
    let content = fs::read_to_string(&delta_path).map_err(|err| {
        anyhow!(
            "failed to read delta spec: {} ({})",
            delta_path.display(),
            err
        )
    })?;

    let context = format!(
        "delta spec `{}` for change `{}`",
        args.capability, args.change_id
    );
    let backend = backend_for_style(config.spec_style);
    let old_doc = backend.parse_delta_spec(&content, &context)?;
    let mut delta = old_doc.clone();
    delta.kind = "llman.sdd.delta".to_string();

    let req_id = args.req_id.trim().to_string();
    if delta.ops.iter().any(|row| row.req_id.trim() == req_id) {
        return Err(anyhow!(
            "{context}: op already exists for `req_id` `{}`",
            req_id
        ));
    }

    let op = args.op.trim().to_ascii_lowercase();
    let row = build_op_row(&context, &op, &req_id, &args)?;
    delta.ops.push(row);

    let payload = backend.dump_delta_spec(
        &delta,
        DumpOptions {
            pretty_ison: args.pretty_ison,
        },
    )?;
    if config.spec_style == SpecStyle::Yaml {
        let writeback = update_delta_spec_markdown_with_overlay_or_fallback(
            &content, &old_doc, &delta, &payload,
        )?;
        if writeback.mode == YamlWriteBackMode::FencedRewrite {
            eprintln!(
                "{}",
                t!(
                    "sdd.yaml.lossless_fallback",
                    path = display_llmanspec_path(&delta_path)
                )
            );
        }
        atomic_write_with_mode(&delta_path, writeback.content.as_bytes(), None)?;
    } else {
        let rebuilt = render_code_fence(config.spec_style.as_str(), &payload);
        atomic_write_with_mode(&delta_path, rebuilt.as_bytes(), None)?;
    }
    println!("{}", delta_path.display());
    Ok(())
}

pub fn run_add_scenario(root: &Path, args: DeltaAddScenarioArgs) -> Result<()> {
    validate_sdd_id(&args.change_id, "change")?;
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

    let delta_path = delta_path(root, &args.change_id, &args.capability);
    let content = fs::read_to_string(&delta_path).map_err(|err| {
        anyhow!(
            "failed to read delta spec: {} ({})",
            delta_path.display(),
            err
        )
    })?;

    let context = format!(
        "delta spec `{}` for change `{}`",
        args.capability, args.change_id
    );
    let backend = backend_for_style(config.spec_style);
    let old_doc = backend.parse_delta_spec(&content, &context)?;
    let mut delta = old_doc.clone();
    delta.kind = "llman.sdd.delta".to_string();

    let req_id = args.req_id.trim();
    let op = delta
        .ops
        .iter()
        .find(|row| row.req_id.trim() == req_id)
        .ok_or_else(|| anyhow!("{context}: unknown op `req_id` `{}`", args.req_id))?
        .op
        .trim()
        .to_ascii_lowercase();
    if op != "add_requirement" && op != "modify_requirement" {
        return Err(anyhow!(
            "{context}: op scenarios are only allowed for add/modify ops; found `{}` for `req_id` `{}`",
            op,
            args.req_id
        ));
    }

    if delta
        .op_scenarios
        .iter()
        .any(|row| row.req_id.trim() == req_id && row.id.trim() == args.scenario_id.trim())
    {
        return Err(anyhow!(
            "{context}: scenario already exists: (req_id, id) = (`{}`, `{}`)",
            args.req_id,
            args.scenario_id
        ));
    }

    delta.op_scenarios.push(ScenarioEntry {
        req_id: args.req_id.trim().to_string(),
        id: args.scenario_id.trim().to_string(),
        given: args.given.trim().to_string(),
        when_: args.when_.trim().to_string(),
        then_: args.then_.trim().to_string(),
    });

    let payload = backend.dump_delta_spec(
        &delta,
        DumpOptions {
            pretty_ison: args.pretty_ison,
        },
    )?;
    if config.spec_style == SpecStyle::Yaml {
        let writeback = update_delta_spec_markdown_with_overlay_or_fallback(
            &content, &old_doc, &delta, &payload,
        )?;
        if writeback.mode == YamlWriteBackMode::FencedRewrite {
            eprintln!(
                "{}",
                t!(
                    "sdd.yaml.lossless_fallback",
                    path = display_llmanspec_path(&delta_path)
                )
            );
        }
        atomic_write_with_mode(&delta_path, writeback.content.as_bytes(), None)?;
    } else {
        let rebuilt = render_code_fence(config.spec_style.as_str(), &payload);
        atomic_write_with_mode(&delta_path, rebuilt.as_bytes(), None)?;
    }
    println!("{}", delta_path.display());
    Ok(())
}

fn delta_path(root: &Path, change_id: &str, capability: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_id)
        .join("specs")
        .join(capability)
        .join("spec.md")
}

fn build_op_row(
    context: &str,
    op: &str,
    req_id: &str,
    args: &DeltaAddOpArgs,
) -> Result<DeltaOpEntry> {
    match op {
        "add_requirement" | "modify_requirement" => {
            let title = args
                .title
                .as_deref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow!("{context}: `--title` is required for op `{}`", op))?;
            let statement = args
                .statement
                .as_deref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow!("{context}: `--statement` is required for op `{}`", op))?;
            if !contains_shall_or_must(statement) {
                return Err(anyhow!(
                    "{context}: `--statement` must contain MUST or SHALL"
                ));
            }
            Ok(DeltaOpEntry {
                op: op.to_string(),
                req_id: req_id.to_string(),
                title: Some(title.to_string()),
                statement: Some(statement.to_string()),
                from: None,
                to: None,
                name: None,
            })
        }
        "remove_requirement" => {
            let name = args
                .name
                .as_deref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string());
            Ok(DeltaOpEntry {
                op: op.to_string(),
                req_id: req_id.to_string(),
                title: None,
                statement: None,
                from: None,
                to: None,
                name,
            })
        }
        "rename_requirement" => {
            let from = args
                .from
                .as_deref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow!("{context}: `--from` is required for op `{}`", op))?;
            let to = args
                .to
                .as_deref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow!("{context}: `--to` is required for op `{}`", op))?;
            Ok(DeltaOpEntry {
                op: op.to_string(),
                req_id: req_id.to_string(),
                title: None,
                statement: None,
                from: Some(from.to_string()),
                to: Some(to.to_string()),
                name: None,
            })
        }
        _ => Err(anyhow!(
            "{context}: unsupported op `{}` (expected add_requirement/modify_requirement/remove_requirement/rename_requirement)",
            op
        )),
    }
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
