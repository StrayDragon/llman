use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::{load_config, write_config};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::backend::{BACKEND, SpecBackend};
use crate::sdd::spec::fence::render_code_fence;
use crate::sdd::spec::frontmatter::compose_with_frontmatter;
use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
use anyhow::{Result, anyhow};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImportArgs {
    pub source: Option<PathBuf>,
    pub scope: Option<String>,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
struct ParsedScenario {
    id: String,
    given: String,
    when_: String,
    then_: String,
}

#[derive(Debug, Clone)]
struct ParsedRequirement {
    title: String,
    statement: String,
    scenarios: Vec<ParsedScenario>,
}

#[derive(Debug, Clone)]
struct ParsedSpec {
    name: String,
    purpose: String,
    requirements: Vec<ParsedRequirement>,
}

#[derive(Debug)]
struct MigrationResult {
    name: String,
    status: MigrationStatus,
    req_count: usize,
    scenario_count: usize,
    errors: Vec<String>,
    reason: String,
}

#[derive(Debug, PartialEq)]
enum MigrationStatus {
    Ok,
    Partial,
    Error,
    Skip,
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::Partial => write!(f, "partial"),
            Self::Error => write!(f, "error"),
            Self::Skip => write!(f, "skip"),
        }
    }
}

pub fn run(root: &Path, args: ImportArgs) -> Result<()> {
    let source = args
        .source
        .unwrap_or_else(|| root.join("openspec").join("specs"));
    let target = root.join(LLMANSPEC_DIR_NAME).join("specs");

    if !source.exists() {
        return Err(anyhow!(
            "source directory does not exist: {}",
            source.display()
        ));
    }
    if !target.parent().is_some_and(|p| p.exists()) {
        return Err(anyhow!(
            "target parent directory does not exist: {} (run `llman sdd init` first)",
            target.display()
        ));
    }

    let mut spec_dirs: Vec<PathBuf> = fs::read_dir(&source)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join("spec.md").exists())
        .collect();
    spec_dirs.sort();

    if let Some(ref scope) = args.scope {
        let pattern = glob::Pattern::new(scope)
            .map_err(|e| anyhow!("invalid --scope glob pattern `{scope}`: {e}"))?;
        spec_dirs.retain(|d| {
            d.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| pattern.matches(n))
        });
    }

    if args.dry_run {
        println!("Mode: DRY RUN (no files will be written)");
    }
    println!("Source: {}", source.display());
    println!("Target: {}", target.display());

    let openspec_config = source.parent().map(|p| p.join("config.yaml"));
    if let Some(ref cfg_path) = openspec_config
        && cfg_path.exists()
    {
        match migrate_config(root, cfg_path, args.dry_run) {
            Ok(true) => println!("Config: merged context from openspec/config.yaml"),
            Ok(false) => println!("Config: no context to merge"),
            Err(e) => println!("Config: merge failed ({e}), skipping"),
        }
    }

    println!("Found {} specs to migrate\n", spec_dirs.len());

    let mut ok_count = 0usize;
    let mut partial_count = 0usize;
    let mut error_count = 0usize;
    let mut skip_count = 0usize;
    let mut total_reqs = 0usize;
    let mut total_scenarios = 0usize;
    let mut failed: Vec<MigrationResult> = Vec::new();
    let mut partial: Vec<MigrationResult> = Vec::new();

    for (i, spec_dir) in spec_dirs.iter().enumerate() {
        let dir_name = spec_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        print!("[{:3}/{}] {}... ", i + 1, spec_dirs.len(), dir_name);

        let spec_file = spec_dir.join("spec.md");
        let text = match fs::read_to_string(&spec_file) {
            Ok(t) => t,
            Err(e) => {
                let r = MigrationResult {
                    name: dir_name.to_string(),
                    status: MigrationStatus::Error,
                    req_count: 0,
                    scenario_count: 0,
                    errors: vec![],
                    reason: format!("read failed: {e}"),
                };
                println!("{} ({})", r.status, r.reason);
                error_count += 1;
                failed.push(r);
                continue;
            }
        };

        let mut parsed = parse_openspec_md(&text);
        if parsed.name.is_empty() {
            parsed.name = dir_name.to_string();
        }

        let target_dir = target.join(dir_name);
        let result = migrate_spec(&parsed, &target_dir, args.dry_run, args.force);

        println!(
            "{} (reqs={}, scenarios={})",
            result.status, result.req_count, result.scenario_count
        );
        if !result.errors.is_empty() {
            for err in &result.errors {
                println!("         ! {err}");
            }
        }

        total_reqs += result.req_count;
        total_scenarios += result.scenario_count;

        match result.status {
            MigrationStatus::Ok => ok_count += 1,
            MigrationStatus::Partial => {
                partial_count += 1;
                partial.push(result);
            }
            MigrationStatus::Error => {
                error_count += 1;
                failed.push(result);
            }
            MigrationStatus::Skip => skip_count += 1,
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Migration summary:");
    println!("  OK:      {ok_count}");
    println!("  Partial: {partial_count}");
    println!("  Error:   {error_count}");
    println!("  Skip:    {skip_count}");
    println!("\n  Total requirements: {total_reqs}");
    println!("  Total scenarios:    {total_scenarios}");

    if !failed.is_empty() {
        println!("\nFailed specs:");
        for r in &failed {
            println!("  - {}: {}", r.name, r.reason);
        }
    }

    if !partial.is_empty() {
        println!("\nPartially migrated specs:");
        for r in &partial {
            println!("  - {}: {} error(s)", r.name, r.errors.len());
            for err in &r.errors {
                println!("      {err}");
            }
        }
    }

    if !args.dry_run && (ok_count > 0 || partial_count > 0) {
        println!("\nNext steps:");
        println!("  1. Run: llman sdd validate --all --strict --no-interactive");
        println!("  2. Fix any validation errors in partial specs");
        println!("  3. Review and commit the migrated specs");
    }

    Ok(())
}

fn migrate_spec(
    spec: &ParsedSpec,
    target_dir: &Path,
    dry_run: bool,
    force: bool,
) -> MigrationResult {
    let target_file = target_dir.join("spec.md");
    if target_file.exists() && !force {
        return MigrationResult {
            name: spec.name.clone(),
            status: MigrationStatus::Skip,
            req_count: 0,
            scenario_count: 0,
            errors: vec![],
            reason: "already exists".into(),
        };
    }

    let total_scenarios: usize = spec.requirements.iter().map(|r| r.scenarios.len()).sum();

    if dry_run {
        return MigrationResult {
            name: spec.name.clone(),
            status: MigrationStatus::Ok,
            req_count: spec.requirements.len(),
            scenario_count: total_scenarios,
            errors: vec![],
            reason: String::new(),
        };
    }

    let mut errors: Vec<String> = Vec::new();
    let mut requirements = Vec::new();
    let mut scenarios = Vec::new();
    let mut written_scenarios = 0usize;

    for (i, req) in spec.requirements.iter().enumerate() {
        let req_id = format!("r{}", i + 1);
        let title = truncate(&req.title, 80);
        let mut statement = truncate(&req.statement, 500);

        if !contains_shall_or_must(&statement) {
            statement = format!("System MUST {statement}");
        }

        if statement.starts_with("- ") {
            errors.push(format!(
                "req {req_id} ({title}): statement starts with '- ', may cause issues"
            ));
        }

        requirements.push(RequirementEntry {
            req_id: req_id.clone(),
            title,
            statement,
        });

        if req.scenarios.is_empty() {
            scenarios.push(ScenarioEntry {
                req_id: req_id.clone(),
                id: "baseline".into(),
                given: String::new(),
                when_: "TODO: describe the trigger".into(),
                then_: "TODO: describe the expected result".into(),
            });
            written_scenarios += 1;
        } else {
            for sc in &req.scenarios {
                scenarios.push(ScenarioEntry {
                    req_id: req_id.clone(),
                    id: sc.id.clone(),
                    given: sc.given.clone(),
                    when_: if sc.when_.is_empty() {
                        "condition is met".into()
                    } else {
                        sc.when_.clone()
                    },
                    then_: if sc.then_.is_empty() {
                        "expected behavior occurs".into()
                    } else {
                        sc.then_.clone()
                    },
                });
                written_scenarios += 1;
            }
        }
    }

    let doc = MainSpecDoc {
        kind: "llman.sdd.spec".into(),
        name: spec.name.clone(),
        purpose: if spec.purpose.is_empty() {
            "TBD".into()
        } else {
            truncate(&spec.purpose, 200)
        },
        requirements,
        scenarios,
    };

    let payload = match BACKEND.dump_main_spec(&doc) {
        Ok(p) => p,
        Err(e) => {
            return MigrationResult {
                name: spec.name.clone(),
                status: MigrationStatus::Error,
                req_count: spec.requirements.len(),
                scenario_count: 0,
                errors: vec![],
                reason: format!("TOON serialization failed: {e}"),
            };
        }
    };

    let frontmatter = build_frontmatter(&spec.name);
    let body = render_code_fence("toon", &payload);
    let content = compose_with_frontmatter(Some(&frontmatter), &body);

    if let Some(parent) = target_file.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return MigrationResult {
            name: spec.name.clone(),
            status: MigrationStatus::Error,
            req_count: spec.requirements.len(),
            scenario_count: 0,
            errors: vec![],
            reason: format!("mkdir failed: {e}"),
        };
    }

    if let Err(e) = atomic_write_with_mode(&target_file, content.as_bytes(), None) {
        return MigrationResult {
            name: spec.name.clone(),
            status: MigrationStatus::Error,
            req_count: spec.requirements.len(),
            scenario_count: 0,
            errors: vec![],
            reason: format!("write failed: {e}"),
        };
    }

    let status = if errors.is_empty() {
        MigrationStatus::Ok
    } else {
        MigrationStatus::Partial
    };

    MigrationResult {
        name: spec.name.clone(),
        status,
        req_count: spec.requirements.len(),
        scenario_count: written_scenarios,
        errors,
        reason: String::new(),
    }
}

fn migrate_config(root: &Path, openspec_config_path: &Path, dry_run: bool) -> Result<bool> {
    let content = fs::read_to_string(openspec_config_path)?;
    let openspec_val: serde_yaml::Value = serde_yaml::from_str(&content)?;
    let openspec_map = openspec_val
        .as_mapping()
        .ok_or_else(|| anyhow!("not a mapping"))?;

    let context = openspec_map
        .get(serde_yaml::Value::String("context".into()))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let rules: Option<std::collections::BTreeMap<String, Vec<String>>> = openspec_map
        .get(serde_yaml::Value::String("rules".into()))
        .and_then(|v| serde_yaml::from_value(v.clone()).ok());

    if context.is_none() && rules.is_none() {
        return Ok(false);
    }

    if dry_run {
        return Ok(true);
    }

    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let mut config = load_config(&llmanspec_dir)?
        .ok_or_else(|| anyhow!("llmanspec config not found (run `llman sdd init` first)"))?;

    if config.context.is_none() {
        config.context = context;
    }
    if config.rules.is_none() {
        config.rules = rules;
    }

    write_config(&llmanspec_dir, &config)?;
    Ok(true)
}

fn parse_openspec_md(text: &str) -> ParsedSpec {
    let mut spec = ParsedSpec {
        name: String::new(),
        purpose: String::new(),
        requirements: Vec::new(),
    };

    let name_re = Regex::new(r"^#\s+(\S+)\s+Specification").expect("regex");
    if let Some(caps) = name_re.captures(text) {
        spec.name = caps[1].to_string();
    }

    let purpose_re = Regex::new(r"(?s)##\s+Purpose\s*\n(.*?)(?:\n##\s|\z)").expect("regex");
    if let Some(caps) = purpose_re.captures(text) {
        let raw = caps[1].trim();
        let bold_re = Regex::new(r"\*\*.*?\*\*\s*").expect("regex");
        let cleaned = bold_re.replace_all(raw, "");
        let lines: Vec<&str> = cleaned
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        spec.purpose = if lines.is_empty() {
            "TBD".into()
        } else {
            truncate(&lines.join(" "), 300)
        };
    }

    let req_header_re = Regex::new(r"(?m)^###\s+Requirement:\s*(.+)$").expect("regex");
    let req_positions: Vec<(usize, String)> = req_header_re
        .captures_iter(text)
        .map(|c| {
            let m = c.get(0).unwrap();
            let title = c[1].trim().to_string();
            (m.end(), title)
        })
        .collect();

    let section_end_re = Regex::new(r"(?m)^##\s").expect("regex");

    for (idx, (body_start, title)) in req_positions.iter().enumerate() {
        let body_end = if let Some(next) = req_positions.get(idx + 1) {
            text[..next.0].rfind("\n###").unwrap_or(next.0)
        } else if let Some(m) = section_end_re.find(&text[*body_start..]) {
            body_start + m.start()
        } else {
            text.len()
        };

        let body = text[*body_start..body_end].trim();

        let before_scenarios: &str = body.split("\n#### Scenario:").next().unwrap_or("");
        let mut stmt_lines = Vec::new();
        for line in before_scenarios.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
                break;
            }
            stmt_lines.push(trimmed);
        }
        let statement = if stmt_lines.is_empty() {
            title.clone()
        } else {
            stmt_lines.join(" ")
        };

        let scenarios = parse_scenarios(body);

        spec.requirements.push(ParsedRequirement {
            title: title.clone(),
            statement,
            scenarios,
        });
    }

    spec
}

fn parse_scenarios(req_body: &str) -> Vec<ParsedScenario> {
    let header_re = Regex::new(r"(?m)^####\s+Scenario:\s*(.+)$").expect("regex");
    let positions: Vec<(usize, String)> = header_re
        .captures_iter(req_body)
        .map(|c| {
            let m = c.get(0).unwrap();
            let name = c[1].trim().to_string();
            (m.end(), name)
        })
        .collect();

    let mut scenarios = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for (idx, (body_start, sc_name)) in positions.iter().enumerate() {
        let body_end = if let Some(next) = positions.get(idx + 1) {
            req_body[..next.0].rfind("\n####").unwrap_or(next.0)
        } else {
            req_body.len()
        };

        let sc_body = req_body[*body_start..body_end].trim();
        let (given, when_, then_) = extract_gherkin(sc_body);
        let mut sc_id = slugify(sc_name, 60);
        if seen_ids.contains(&sc_id) {
            sc_id = format!("{}-{}", sc_id, seen_ids.len());
        }
        seen_ids.insert(sc_id.clone());
        scenarios.push(ParsedScenario {
            id: sc_id,
            given,
            when_,
            then_,
        });
    }

    scenarios
}

fn extract_gherkin(body: &str) -> (String, String, String) {
    let given_re =
        Regex::new(r"(?si)\*\*(?:GIVEN|Given)\*\*\s*(.*?)(?:\n\s*-\s*\*\*|\z)").expect("regex");
    let when_re =
        Regex::new(r"(?si)\*\*(?:WHEN|When)\*\*\s*(.*?)(?:\n\s*-\s*\*\*|\z)").expect("regex");
    let then_re =
        Regex::new(r"(?si)\*\*(?:THEN|Then)\*\*\s*(.*?)(?:\n\s*-\s*\*\*|\z)").expect("regex");

    let given = given_re
        .captures(body)
        .map(|c| clean_gherkin(&c[1]))
        .unwrap_or_default();
    let mut when_ = when_re
        .captures(body)
        .map(|c| clean_gherkin(&c[1]))
        .unwrap_or_default();
    let mut then_ = then_re
        .captures(body)
        .map(|c| clean_gherkin(&c[1]))
        .unwrap_or_default();

    if when_.is_empty() && then_.is_empty() {
        let lines: Vec<&str> = body
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        let combined = lines.join(" ");
        when_ = truncate(&combined, 200);
        if when_.is_empty() {
            when_ = "condition is met".into();
        }
        then_ = "expected behavior occurs".into();
    }

    if when_.is_empty() {
        when_ = "condition is met".into();
    }
    if then_.is_empty() {
        then_ = "expected behavior occurs".into();
    }

    (given, when_, then_)
}

fn clean_gherkin(text: &str) -> String {
    let and_re = Regex::new(r"\*\*(?:AND|and|And)\*\*\s*").expect("regex");
    let bullet_start = Regex::new(r"^\s*-\s*").expect("regex");
    let mut lines = Vec::new();
    for line in text.trim().lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- **") {
            break;
        }
        let cleaned = bullet_start.replace(trimmed, "");
        let cleaned = and_re.replace_all(&cleaned, "");
        let cleaned = cleaned.trim().to_string();
        if !cleaned.is_empty() {
            lines.push(cleaned);
        }
    }
    let result = lines.join(" ");
    let ws_re = Regex::new(r"\s+").expect("regex");
    ws_re.replace_all(result.trim(), " ").to_string()
}

fn slugify(name: &str, max_len: usize) -> String {
    let lower = name.to_lowercase();
    let slug_re = Regex::new(r"[^a-z0-9\u{4e00}-\u{9fff}]+").expect("regex");
    let slug = slug_re.replace_all(&lower, "-");
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        return "default".into();
    }
    truncate(slug, max_len)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len).collect()
    }
}

fn contains_shall_or_must(text: &str) -> bool {
    text.contains("SHALL") || text.contains("MUST")
}

fn build_frontmatter(spec_name: &str) -> String {
    [
        "llman_spec_valid_scope:",
        "  - src/",
        "  - tests/",
        "llman_spec_valid_commands:",
        &format!("  - llman sdd validate {spec_name} --type spec --strict --no-interactive"),
        "llman_spec_evidence:",
        "  - migrated from openspec",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_openspec_md() {
        let md = r#"# my-feature Specification
## Purpose
Define how my-feature works.
## Requirements
### Requirement: Basic operation
The CLI MUST support basic operation with standard input.
#### Scenario: Happy path
- **WHEN** the user runs the command
- **THEN** the system produces expected output
### Requirement: Error handling
The CLI MUST return an error for invalid input.
#### Scenario: Invalid input
- **GIVEN** the user provides invalid data
- **WHEN** the command is executed
- **THEN** the system returns a descriptive error
"#;

        let parsed = parse_openspec_md(md);
        assert_eq!(parsed.name, "my-feature");
        assert!(parsed.purpose.contains("my-feature works"));
        assert_eq!(parsed.requirements.len(), 2);
        assert_eq!(parsed.requirements[0].title, "Basic operation");
        assert_eq!(parsed.requirements[0].scenarios.len(), 1);
        assert_eq!(parsed.requirements[0].scenarios[0].id, "happy-path");
        assert_eq!(
            parsed.requirements[1].scenarios[0].given,
            "the user provides invalid data"
        );
    }

    #[test]
    fn slugify_names() {
        assert_eq!(slugify("Happy path", 60), "happy-path");
        assert_eq!(
            slugify("CLI override provided", 60),
            "cli-override-provided"
        );
        assert_eq!(slugify("", 60), "default");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello", 3), "hel");
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn extract_gherkin_full() {
        let body = r#"- **GIVEN** precondition exists
- **WHEN** user triggers action
- **THEN** system responds correctly"#;
        let (g, w, t) = extract_gherkin(body);
        assert_eq!(g, "precondition exists");
        assert_eq!(w, "user triggers action");
        assert_eq!(t, "system responds correctly");
    }

    #[test]
    fn extract_gherkin_missing_given() {
        let body = r#"- **WHEN** user triggers action
- **THEN** system responds correctly"#;
        let (g, w, t) = extract_gherkin(body);
        assert!(g.is_empty());
        assert_eq!(w, "user triggers action");
        assert_eq!(t, "system responds correctly");
    }

    #[test]
    fn build_frontmatter_contains_spec_name() {
        let fm = build_frontmatter("config-paths");
        assert!(fm.contains("config-paths"));
        assert!(fm.contains("migrated from openspec"));
    }
}
