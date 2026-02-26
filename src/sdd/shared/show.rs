use crate::sdd::project::templates::TemplateStyle;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{list_changes, list_specs};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::interactive::is_interactive;
use crate::sdd::shared::match_utils::nearest_matches;
use crate::sdd::spec::parser::{Requirement, parse_change, parse_spec};
use anyhow::{Result, anyhow};
use inquire::Select;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ShowArgs {
    pub item: Option<String>,
    pub json: bool,
    pub item_type: Option<String>,
    pub no_interactive: bool,
    pub deltas_only: bool,
    pub requirements_only: bool,
    pub requirements: bool,
    pub no_scenarios: bool,
    pub requirement: Option<usize>,
    pub style: TemplateStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ItemType {
    Change,
    Spec,
}

impl ItemType {
    fn as_str(self) -> &'static str {
        match self {
            ItemType::Change => "change",
            ItemType::Spec => "spec",
        }
    }
}

impl fmt::Display for ItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ItemType::Change => t!("sdd.show.option_change"),
            ItemType::Spec => t!("sdd.show.option_spec"),
        };
        write!(f, "{label}")
    }
}

pub fn run(args: ShowArgs) -> Result<()> {
    let _style = args.style;
    let root = Path::new(".");
    let interactive = is_interactive(args.no_interactive);
    let type_override = normalize_type(args.item_type.as_deref());

    if args.item.is_none() {
        if interactive {
            let choice = Select::new(
                &t!("sdd.show.select_type"),
                vec![ItemType::Change, ItemType::Spec],
            )
            .prompt()?;
            return run_interactive_by_type(root, choice, &args);
        }
        print_non_interactive_hint();
        std::process::exit(1);
    }

    let item = args.item.as_ref().unwrap();
    show_direct(root, item, type_override, &args)
}

fn normalize_type(value: Option<&str>) -> Option<ItemType> {
    let value = value?.to_lowercase();
    match value.as_str() {
        "change" => Some(ItemType::Change),
        "spec" => Some(ItemType::Spec),
        _ => None,
    }
}

fn run_interactive_by_type(root: &Path, item_type: ItemType, args: &ShowArgs) -> Result<()> {
    match item_type {
        ItemType::Change => {
            let changes = list_changes(root)?;
            if changes.is_empty() {
                eprintln!("{}", t!("sdd.show.no_changes_found"));
                std::process::exit(1);
            }
            let picked = Select::new(&t!("sdd.show.pick_change"), changes).prompt()?;
            show_change(root, &picked, args)
        }
        ItemType::Spec => {
            let specs = list_specs(root)?;
            if specs.is_empty() {
                eprintln!("{}", t!("sdd.show.no_specs_found"));
                std::process::exit(1);
            }
            let picked = Select::new(&t!("sdd.show.pick_spec"), specs).prompt()?;
            show_spec(root, &picked, args)
        }
    }
}

fn show_direct(
    root: &Path,
    item: &str,
    type_override: Option<ItemType>,
    args: &ShowArgs,
) -> Result<()> {
    let mut changes: Vec<String> = Vec::new();
    let mut specs: Vec<String> = Vec::new();
    let mut is_change = false;
    let mut is_spec = false;

    match type_override {
        Some(ItemType::Change) => {
            changes = list_changes(root)?;
            is_change = changes.contains(&item.to_string());
        }
        Some(ItemType::Spec) => {
            specs = list_specs(root)?;
            is_spec = specs.contains(&item.to_string());
        }
        None => {
            changes = list_changes(root)?;
            specs = list_specs(root)?;
            is_change = changes.contains(&item.to_string());
            is_spec = specs.contains(&item.to_string());
        }
    }

    let resolved_type = type_override.or(if is_change {
        Some(ItemType::Change)
    } else if is_spec {
        Some(ItemType::Spec)
    } else {
        None
    });

    if resolved_type.is_none() {
        eprintln!("{}", t!("sdd.show.unknown_item", item = item));
        let mut candidates = Vec::new();
        if changes.is_empty() && specs.is_empty() {
            candidates.extend(list_changes(root)?);
            candidates.extend(list_specs(root)?);
        } else {
            candidates.extend(changes);
            candidates.extend(specs);
        }
        let suggestions = nearest_matches(item, &candidates, 5);
        if !suggestions.is_empty() {
            eprintln!(
                "{}",
                t!("sdd.show.did_you_mean", items = suggestions.join(", "))
            );
        }
        std::process::exit(1);
    }

    if type_override.is_none() && is_change && is_spec {
        eprintln!("{}", t!("sdd.show.ambiguous_item", item = item));
        eprintln!("{}", t!("sdd.show.ambiguous_hint"));
        std::process::exit(1);
    }

    let resolved_type = resolved_type.expect("resolved type");
    warn_irrelevant_flags(resolved_type, args);

    match resolved_type {
        ItemType::Change => show_change(root, item, args),
        ItemType::Spec => show_spec(root, item, args),
    }
}

fn show_change(root: &Path, change_id: &str, args: &ShowArgs) -> Result<()> {
    validate_sdd_id(change_id, "change")?;
    let change_dir = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_id);
    let proposal_path = change_dir.join("proposal.md");
    if !proposal_path.exists() {
        return Err(anyhow!(t!("sdd.show.change_not_found", id = change_id)));
    }

    if args.json {
        let content = fs::read_to_string(&proposal_path)?;
        let change = parse_change(&content, change_id, &change_dir)?;
        let title = extract_title(&content, change_id);
        let deltas = change.deltas;
        if args.requirements_only {
            eprintln!("{}", t!("sdd.show.requirements_only_deprecated"));
        }
        let output = serde_json::json!({
            "id": change_id,
            "title": title,
            "deltaCount": deltas.len(),
            "deltas": deltas
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let content = fs::read_to_string(&proposal_path)?;
    print!("{content}");
    Ok(())
}

fn show_spec(root: &Path, spec_id: &str, args: &ShowArgs) -> Result<()> {
    validate_sdd_id(spec_id, "spec")?;
    let spec_path = root
        .join(LLMANSPEC_DIR_NAME)
        .join("specs")
        .join(spec_id)
        .join("spec.md");
    if !spec_path.exists() {
        return Err(anyhow!(t!("sdd.show.spec_not_found", id = spec_id)));
    }

    if args.json {
        if args.requirements && args.requirement.is_some() {
            return Err(anyhow!(t!("sdd.show.requirements_conflict")));
        }
        let content = fs::read_to_string(&spec_path)?;
        let spec = parse_spec(&content, spec_id)?;
        let requirements = filter_requirements(&spec.requirements, args)?;
        let output = serde_json::json!({
            "id": spec_id,
            "title": spec.name,
            "overview": spec.overview,
            "requirementCount": requirements.len(),
            "requirements": requirements,
            "metadata": spec.metadata
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let content = fs::read_to_string(&spec_path)?;
    print!("{content}");
    Ok(())
}

fn filter_requirements(requirements: &[Requirement], args: &ShowArgs) -> Result<Vec<Requirement>> {
    let requirement_index = match args.requirement {
        Some(index) => {
            if index == 0 || index > requirements.len() {
                return Err(anyhow!(t!(
                    "sdd.show.requirement_not_found",
                    id = index,
                    count = requirements.len()
                )));
            }
            Some(index - 1)
        }
        None => None,
    };

    let include_scenarios = !args.requirements && !args.no_scenarios;
    let selected: Vec<Requirement> = if let Some(index) = requirement_index {
        vec![requirements[index].clone()]
    } else {
        requirements.to_vec()
    };

    Ok(selected
        .into_iter()
        .map(|req| Requirement {
            text: req.text,
            scenarios: if include_scenarios {
                req.scenarios
            } else {
                Vec::new()
            },
        })
        .collect())
}

fn warn_irrelevant_flags(item_type: ItemType, args: &ShowArgs) {
    let mut ignored = Vec::new();
    match item_type {
        ItemType::Change => {
            if args.requirements {
                ignored.push("--requirements");
            }
            if args.no_scenarios {
                ignored.push("--no-scenarios");
            }
            if args.requirement.is_some() {
                ignored.push("--requirement");
            }
        }
        ItemType::Spec => {
            if args.deltas_only {
                ignored.push("--deltas-only");
            }
            if args.requirements_only {
                ignored.push("--requirements-only");
            }
        }
    }

    if !ignored.is_empty() {
        eprintln!(
            "{}",
            t!(
                "sdd.show.ignore_flags",
                item_type = item_type.as_str(),
                flags = ignored.join(", ")
            )
        );
    }
}

fn extract_title(content: &str, fallback: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(title) = trimmed.strip_prefix("# ") {
            let cleaned = title.trim();
            if let Some(stripped) = cleaned.strip_prefix("Change: ") {
                return stripped.trim().to_string();
            }
            return cleaned.to_string();
        }
    }
    fallback.to_string()
}

fn print_non_interactive_hint() {
    eprintln!("{}", t!("sdd.show.non_interactive.line1"));
    eprintln!("{}", t!("sdd.show.non_interactive.line2"));
    eprintln!("{}", t!("sdd.show.non_interactive.line3"));
    eprintln!("{}", t!("sdd.show.non_interactive.line4"));
    eprintln!("{}", t!("sdd.show.non_interactive.line5"));
}
