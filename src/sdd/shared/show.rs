use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::shared::discovery::{
    list_changes, list_specs, resolve_change_dir, resolve_change_rel_path,
};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::shared::interactive::is_interactive;
use crate::sdd::shared::json::print_json;
use crate::sdd::shared::match_utils::nearest_matches;
use crate::sdd::shared::types::{ItemType, normalize_type};
use crate::sdd::spec::backend::feature_backend;
use crate::sdd::spec::backend::feature_backend::compute_rule_morphology;
use crate::sdd::spec::parser::parse_change;
use crate::sdd::spec::validation::determine_stage;
use anyhow::{Result, anyhow};
use inquire::Select;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct ShowArgs {
    pub(crate) item: Option<String>,
    pub(crate) json: bool,
    pub(crate) compact_json: bool,
    pub(crate) item_type: Option<String>,
    pub(crate) no_interactive: bool,
    pub(crate) deltas_only: bool,
    pub(crate) requirements_only: bool,
    pub(crate) requirements: bool,
    pub(crate) no_scenarios: bool,
    pub(crate) requirement: Option<usize>,
    pub(crate) meta_only: bool,
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

pub(crate) fn run(args: ShowArgs) -> Result<()> {
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
        return Err(anyhow!(non_interactive_hint_message()));
    }

    let Some(item) = args.item.as_deref() else {
        return Err(anyhow!(non_interactive_hint_message()));
    };
    show_direct(root, item, type_override, &args)
}

fn run_interactive_by_type(root: &Path, item_type: ItemType, args: &ShowArgs) -> Result<()> {
    match item_type {
        ItemType::Change => {
            let changes = list_changes(root)?;
            if changes.is_empty() {
                return Err(anyhow!(t!("sdd.show.no_changes_found")));
            }
            let picked = Select::new(&t!("sdd.show.pick_change"), changes).prompt()?;
            show_change(root, &picked, false, args)
        }
        ItemType::Spec => {
            let specs = list_specs(root)?;
            if specs.is_empty() {
                return Err(anyhow!(t!("sdd.show.no_specs_found")));
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
    let mut is_change = false;
    let mut is_spec = false;
    let mut resolved_change_id: Option<String> = None;
    // Whether the change id was resolved via a prefix match (for the r112 hint
    // + JSON `matchedViaPrefix` field). Only meaningful when is_change.
    let mut matched_via_prefix = false;

    match type_override {
        Some(ItemType::Change) => {
            // Use prefix-aware resolution
            let resolved = crate::sdd::shared::discovery::resolve_change_id(root, item)?;
            matched_via_prefix = resolved.via_prefix;
            resolved_change_id = Some(resolved.id);
            is_change = true;
        }
        Some(ItemType::Spec) => {
            let specs = list_specs(root)?;
            is_spec = specs.contains(&item.to_string());
        }
        None => {
            // Per cli spec r112: exact match takes priority over prefix.
            // Check exact spec match first so a spec id (e.g. `cli`) is not
            // hijacked by a change whose id starts with it (e.g. `cli-xxx`).
            let specs = list_specs(root)?;
            if specs.contains(&item.to_string()) {
                is_spec = true;
            } else if let Ok(resolved) =
                crate::sdd::shared::discovery::resolve_change_id(root, item)
            {
                matched_via_prefix = resolved.via_prefix;
                resolved_change_id = Some(resolved.id);
                is_change = true;
            }
        }
    }

    let resolved_type = type_override.or(if is_change {
        Some(ItemType::Change)
    } else if is_spec {
        Some(ItemType::Spec)
    } else {
        None
    });

    let Some(resolved_type) = resolved_type else {
        let mut candidates = Vec::new();
        candidates.extend(list_changes(root)?);
        candidates.extend(list_specs(root)?);
        let suggestions = nearest_matches(item, &candidates, 5);

        let mut msg = t!("sdd.show.unknown_item", item = item).to_string();
        if !suggestions.is_empty() {
            msg.push('\n');
            msg.push_str(&t!("sdd.show.did_you_mean", items = suggestions.join(", ")));
        }
        return Err(anyhow!(msg));
    };

    if type_override.is_none() && is_change && is_spec {
        return Err(anyhow!(
            "{}\n{}",
            t!("sdd.show.ambiguous_item", item = item),
            t!("sdd.show.ambiguous_hint")
        ));
    }
    warn_irrelevant_flags(resolved_type, args);

    match resolved_type {
        ItemType::Change => {
            let change_id = resolved_change_id.as_deref().unwrap_or(item);
            // r112: emit the "'input' -> 'resolved' (prefix match)" hint to stderr
            // for human output when the change id was resolved via a prefix.
            if matched_via_prefix && !args.json {
                eprintln!(
                    "{}",
                    t!("sdd.prefix_match_hint", input = item, resolved = change_id)
                );
            }
            show_change(root, change_id, matched_via_prefix, args)
        }
        ItemType::Spec => show_spec(root, item, args),
    }
}

fn show_change(
    root: &Path,
    change_id: &str,
    matched_via_prefix: bool,
    args: &ShowArgs,
) -> Result<()> {
    validate_sdd_id(change_id, "change")?;
    let change_dir = resolve_change_dir(root, change_id)
        .map_err(|_| anyhow!(t!("sdd.show.change_not_found", id = change_id)))?;
    let rel_path =
        resolve_change_rel_path(root, change_id).unwrap_or_else(|_| change_id.to_string());
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
        let stage = determine_stage(&change_dir);
        let artifacts = list_change_artifacts(&change_dir);
        let landing = crate::sdd::change::specs_landing::evaluate_specs_landing(root, &change_dir);
        let ready_to_implement = landing.ready_to_implement;
        // Unified Git-native flow: always surface the attach binding (no longer
        // BDD-on only). stage=full comes from Git-native attach.
        let attached = crate::sdd::spec::validation::has_attach_binding(&change_dir);
        let output = serde_json::json!({
            "id": change_id,
            "path": rel_path,
            "title": title,
            "stage": stage.as_str(),
            "artifacts": artifacts,
            "readyToImplement": ready_to_implement,
            "specsLanded": landing.specs_landed,
            "skipSpecsLanding": landing.skip_specs_landing,
            "attached": attached,
            "deltaCount": deltas.len(),
            "deltas": deltas,
            // r112: surface whether the change id came from a prefix match.
            "matchedViaPrefix": matched_via_prefix
        });
        print_json(&output, args.compact_json)?;
        return Ok(());
    }

    let content = fs::read_to_string(&proposal_path)?;
    let stage = determine_stage(&change_dir);
    println!("{}", t!("sdd.show.change_stage", stage = stage.as_str()));
    println!("path: {rel_path}");
    print!("{content}");
    Ok(())
}

/// Enumerate the artifacts actually present in a change directory.
///
/// Mirrors the existence checks used by `determine_stage` so the reported
/// `artifacts` list is consistent with the inferred `stage` (e.g. an empty
/// `specs/` directory does not count as a present artifact).
fn list_change_artifacts(change_dir: &Path) -> Vec<&'static str> {
    let mut artifacts = Vec::new();
    if change_dir.join("proposal.md").exists() {
        artifacts.push("proposal.md");
    }
    let has_specs = match fs::read_dir(change_dir.join("specs")) {
        Ok(entries) => entries.flatten().any(|e| {
            e.file_type().map(|t| t.is_dir()).unwrap_or(false) && e.path().join(SPEC_FILE).exists()
        }),
        Err(_) => false,
    };
    if has_specs {
        artifacts.push("specs");
    }
    if change_dir.join("design.md").exists() {
        artifacts.push("design.md");
    }
    if change_dir.join("tasks.md").exists() {
        artifacts.push("tasks.md");
    }
    artifacts
}

fn show_spec(root: &Path, spec_id: &str, args: &ShowArgs) -> Result<()> {
    validate_sdd_id(spec_id, "spec")?;
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec_dir)?;

    // Single-track (r131): the capability's `.feature` is the spec.
    let specs_root = root.join(LLMANSPEC_DIR_NAME).join("specs");
    let spec_path =
        crate::sdd::spec::validation::resolve_spec_file(&specs_root, spec_id).map_err(|err| {
            anyhow!(t!(
                "sdd.show.spec_not_found_with_reason",
                id = spec_id,
                reason = err.to_string()
            ))
        })?;
    let content = fs::read_to_string(&spec_path)?;

    let parsed = crate::sdd::spec::backend::FEATURE_BACKEND
        .parse_content(&content, &format!("spec `{spec_id}`"))?;
    let morphology = Some(compute_rule_morphology(&parsed));

    // T0 presenters (feat-sdd-review-workflow-suite): JSON and text rendering
    // are pure functions over the parsed spec so the review suite can reuse
    // the exact same shapes. Output is byte-frozen (r134 keys / r39 morph).

    if args.json {
        let output = render_spec_json(spec_id, &parsed, morphology, args)?;
        print_json(&output, args.compact_json)?;
        return Ok(());
    }

    println!("{}", render_spec_text(&content, morphology));
    Ok(())
}

/// Pure JSON renderer. Key sets are behavior-frozen: meta branch
/// `{id,featureId,title,purpose,overview,requirementCount,morphology}` and
/// full branch adds `requirements`. `requirements`/`--requirement` conflict
/// is only observable in JSON mode (preserved here).
fn render_spec_json(
    spec_id: &str,
    parsed: &feature_backend::ParsedFeatureSpec,
    morphology: Option<feature_backend::RuleMorphology>,
    args: &ShowArgs,
) -> Result<serde_json::Value> {
    if args.requirements && args.requirement.is_some() {
        return Err(anyhow!(t!("sdd.show.requirements_conflict")));
    }
    if args.meta_only {
        return Ok(serde_json::json!({
            "id": spec_id,
            "featureId": parsed.name,
            "title": parsed.feature_title,
            "purpose": parsed.purpose,
            "overview": parsed.purpose,
            "requirementCount": parsed.rule_scenarios().count(),
            "morphology": morphology,
        }));
    }

    let requirements_json = requirements_json_from_parsed(parsed, args)?;
    let rule_count = parsed.rule_scenarios().count();
    Ok(serde_json::json!({
        "id": spec_id,
        "title": parsed.feature_title,
        "purpose": parsed.purpose,
        "overview": parsed.purpose,
        "requirementCount": rule_count,
        "requirements": requirements_json,
        "morphology": morphology,
    }))
}

/// Pure text renderer (byte-frozen): header, full feature source, morphology line.
fn render_spec_text(content: &str, morphology: Option<feature_backend::RuleMorphology>) -> String {
    let mut out = String::new();
    out.push_str("## Spec\n");
    out.push_str(content);
    out.push_str("\n## Morphology\n");
    if let Some(m) = morphology {
        out.push_str(&format!(
            "ruleCount={} enforced={} manual={} pending={} acceptanceCount={}",
            m.rule_count,
            m.rule_enforced_count,
            m.rule_manual_count,
            m.rule_pending_count,
            m.acceptance_count
        ));
    }
    out
}

/// Build the requirements JSON array straight from the rich parse (r132).
fn requirements_json_from_parsed(
    parsed: &feature_backend::ParsedFeatureSpec,
    args: &ShowArgs,
) -> Result<Vec<serde_json::Value>> {
    let include_scenarios = !args.requirements && !args.no_scenarios;
    let mut out = Vec::new();
    for rule in parsed.rule_scenarios() {
        let mut scenarios = Vec::new();
        if include_scenarios {
            for acc in parsed.acceptance_scenarios() {
                if acc.req_ids.iter().any(|r| rule.req_ids.contains(r)) {
                    scenarios.push(serde_json::json!({
                        "id": acc.name,
                        "rawText": format!(
                            "GIVEN: {}\nWHEN: {}\nTHEN: {}",
                            acc.given.join("\n"),
                            acc.when_.join("\n"),
                            acc.then_.join("\n"),
                        ),
                        "source": "acceptance",
                        "reqIds": acc.req_ids,
                    }));
                }
            }
        }
        out.push(serde_json::json!({
            "reqId": rule.req_ids.first().cloned().unwrap_or_default(),
            "title": rule.name,
            "text": feature_backend::rule_statement(rule),
            "scenarios": scenarios,
        }));
    }
    Ok(out)
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
            if args.meta_only {
                ignored.push("--meta-only");
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

fn non_interactive_hint_message() -> String {
    super::interactive::non_interactive_hint_message(
        t!("sdd.show.non_interactive.line1").to_string(),
        &[
            t!("sdd.show.non_interactive.line2").to_string(),
            t!("sdd.show.non_interactive.line3").to_string(),
            t!("sdd.show.non_interactive.line4").to_string(),
        ],
    )
}

#[cfg(test)]
mod t0_freeze_tests {
    use super::*;

    const SAMPLE: &str = "# language: en\n# capability: demo\n# purpose: demo\n# scope: src/\n\nFeature: demo\n\n  @req:r1 @human\n  Scenario: rule-one\n    System MUST behave.\n";

    fn parse_sample() -> feature_backend::ParsedFeatureSpec {
        crate::sdd::spec::backend::FEATURE_BACKEND
            .parse_content(SAMPLE, "test")
            .expect("sample parses")
    }

    #[test]
    fn text_renderer_shape_frozen() {
        let parsed = parse_sample();
        let morph = Some(compute_rule_morphology(&parsed));
        let out = render_spec_text(SAMPLE, morph);
        assert!(out.starts_with("## Spec\n"));
        assert!(out.contains("\n## Morphology\n"));
        assert!(
            out.contains("ruleCount=1 enforced=0 manual=0 pending=1 acceptanceCount=0"),
            "got: {out}"
        );
    }

    #[test]
    fn json_meta_keyset_frozen() {
        let parsed = parse_sample();
        let morph = Some(compute_rule_morphology(&parsed));
        let args = ShowArgs {
            item: None,
            item_type: None,
            json: true,
            compact_json: false,
            meta_only: true,
            requirements: false,
            requirements_only: false,
            requirement: None,
            no_scenarios: false,
            deltas_only: false,
            no_interactive: false,
        };
        let v = render_spec_json("demo", &parsed, morph, &args).unwrap();
        let obj = v.as_object().unwrap();
        let mut keys: Vec<_> = obj.keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "featureId",
                "id",
                "morphology",
                "overview",
                "purpose",
                "requirementCount",
                "title"
            ]
        );
    }

    #[test]
    fn json_full_keyset_frozen() {
        let parsed = parse_sample();
        let morph = Some(compute_rule_morphology(&parsed));
        let args = ShowArgs {
            item: None,
            item_type: None,
            json: true,
            compact_json: false,
            meta_only: false,
            requirements: false,
            requirements_only: false,
            requirement: None,
            no_scenarios: false,
            deltas_only: false,
            no_interactive: false,
        };
        let v = render_spec_json("demo", &parsed, morph, &args).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj.contains_key("requirements"));
        assert!(obj.contains_key("morphology"));
        assert_eq!(obj["requirementCount"], 1);
    }
}
