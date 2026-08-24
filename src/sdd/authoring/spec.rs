use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::backend::feature_backend;
use crate::sdd::spec::validation::{locale_to_gherkin_lang, resolve_spec_file};
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

fn spec_file_path(root: &Path, capability: &str) -> Result<PathBuf> {
    let specs_dir = root.join(LLMANSPEC_DIR_NAME).join("specs");
    resolve_spec_file(&specs_dir, capability)
}

fn spec_lang(root: &Path) -> String {
    let raw = fs::read_to_string(root.join(LLMANSPEC_DIR_NAME).join("config.yaml")).ok();
    raw.and_then(|text| {
        text.lines().find_map(|l| {
            let l = l.trim();
            l.strip_prefix("locale:")
                .map(|v| v.trim().trim_matches('"').to_string())
        })
    })
    .map(|locale| locale_to_gherkin_lang(Some(&locale), None))
    .unwrap_or_else(|| "en".to_string())
}

pub fn run_skeleton(root: &Path, args: SpecSkeletonArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    let _ = load_required_config(&root.join(LLMANSPEC_DIR_NAME))?;

    let spec_dir = root
        .join(LLMANSPEC_DIR_NAME)
        .join("specs")
        .join(&args.capability);
    if spec_dir.exists() && !args.force {
        return Err(anyhow!(
            "spec skeleton target `{}` already exists (pass --force to overwrite)",
            spec_dir.display()
        ));
    }
    fs::create_dir_all(&spec_dir)?;

    // Allocate first req_id; printed as a hint for the next `add-requirement`.
    let first_req_id =
        crate::sdd::spec::req_registry::next_req_id(root).unwrap_or_else(|_| "r1".to_string());

    let lang = spec_lang(root);
    let kw = feature_backend::keywords_for(&lang);
    // The skeleton is valid single-track from day one (r133 headers + one
    // placeholder @human rule carrying a MUST statement).
    let body = format!(
        "# language: {lang}\n\
         # capability: {name}\n\
         # purpose: TODO: Describe this capability and its purpose.\n\
         # scope: src/\n\n\
         {feature}: {name}\n\n\
         \x20 @req:{req} @human\n\
         \x20 {scenario}: TODO-rule\n\
         \x20   System MUST ...\n",
        lang = lang,
        name = args.capability,
        feature = kw.feature,
        scenario = kw.scenario,
        req = first_req_id,
    );
    let spec_path = spec_dir.join(format!("{}.feature", args.capability));
    atomic_write_with_mode(&spec_path, body.as_bytes(), None)?;
    println!("wrote {}", spec_path.display());
    println!(
        "next-req-id: {first_req_id} (use `llman sdd spec add-requirement {name} {first_req_id} --title ... --statement ...`)",
        name = args.capability,
    );
    Ok(())
}

pub fn run_add_requirement(root: &Path, args: SpecAddRequirementArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    let _ = load_required_config(&root.join(LLMANSPEC_DIR_NAME))?;
    if args.title.trim().is_empty() {
        return Err(anyhow!("title must not be empty"));
    }
    let statement = args.statement.trim();
    if !crate::sdd::spec::validation::contains_normative_keyword(statement) {
        return Err(anyhow!(
            "statement must contain a normative keyword (MUST/SHALL or 必须/不得/禁止)"
        ));
    }

    crate::sdd::spec::req_registry::ensure_req_id_globally_free(root, &args.req_id)?;

    let spec_path = spec_file_path(root, &args.capability)?;
    let content = fs::read_to_string(&spec_path)
        .map_err(|err| anyhow!("failed to read spec: {} ({})", spec_path.display(), err))?;

    let context = format!("spec `{}`", args.capability);
    let parsed = feature_backend::FeatureBackend.parse_content(&content, &context)?;
    if parsed.rule_scenarios().any(|sc| {
        sc.req_ids
            .iter()
            .any(|rid| rid.trim() == args.req_id.trim())
    }) {
        return Err(anyhow!(
            "{context}: requirement already exists: `{}`",
            args.req_id
        ));
    }

    let lang = feature_backend::detect_language(&content);
    let kw = feature_backend::keywords_for(&lang);
    // Append the rule block at the end of the file (textual, lossless).
    let mut updated = content.clone();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&format!(
        "\n  @req:{} @human\n  {}: {}\n    {}\n",
        args.req_id.trim(),
        kw.scenario,
        args.title.trim(),
        statement
    ));
    atomic_write_with_mode(&spec_path, updated.as_bytes(), None)?;
    println!("{}", spec_path.display());
    Ok(())
}

pub fn run_add_scenario(root: &Path, args: SpecAddScenarioArgs) -> Result<()> {
    validate_sdd_id(&args.capability, "spec")?;
    validate_sdd_id(&args.req_id, "requirement")?;
    validate_sdd_id(&args.scenario_id, "scenario")?;
    let _ = load_required_config(&root.join(LLMANSPEC_DIR_NAME))?;
    if args.when_.trim().is_empty() {
        return Err(anyhow!("--when must not be empty"));
    }
    if args.then_.trim().is_empty() {
        return Err(anyhow!("--then must not be empty"));
    }

    let spec_path = spec_file_path(root, &args.capability)?;
    let content = fs::read_to_string(&spec_path)
        .map_err(|err| anyhow!("failed to read spec: {} ({})", spec_path.display(), err))?;

    let context = format!("spec `{}`", args.capability);
    let parsed = feature_backend::FeatureBackend.parse_content(&content, &context)?;

    let rule_exists = parsed.rule_scenarios().any(|sc| {
        sc.req_ids
            .iter()
            .any(|rid| rid.trim() == args.req_id.trim())
    });
    if !rule_exists {
        return Err(anyhow!(
            "{context}: unknown requirement `req_id` `{}`",
            args.req_id
        ));
    }
    if parsed
        .acceptance_scenarios()
        .any(|sc| sc.name.trim() == args.scenario_id.trim())
    {
        return Err(anyhow!(
            "{context}: scenario already exists: `{}`",
            args.scenario_id
        ));
    }

    let lang = feature_backend::detect_language(&content);
    let kw = feature_backend::keywords_for(&lang);
    let mut updated = content.clone();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&format!(
        "\n  @req:{} @executable\n  {}: {}\n",
        args.req_id.trim(),
        kw.scenario,
        args.scenario_id.trim()
    ));
    if !args.given.trim().is_empty() {
        for line in args.given.trim().lines() {
            updated.push_str(&format!("    {} {}\n", kw.given, line));
        }
    }
    for line in args.when_.trim().lines() {
        updated.push_str(&format!("    {} {}\n", kw.when, line));
    }
    for line in args.then_.trim().lines() {
        updated.push_str(&format!("    {} {}\n", kw.then, line));
    }
    atomic_write_with_mode(&spec_path, updated.as_bytes(), None)?;
    println!("{}", spec_path.display());
    Ok(())
}
