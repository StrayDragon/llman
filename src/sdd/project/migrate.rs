//! One-shot, idempotent conversion: legacy `spec.toon` → single-track
//! `<capability>.feature` (r136).
//!
//! - Existing `*.feature` files in the capability directory are live harness
//!   assets; they are NEVER read, rewritten, or deleted. The report counts
//!   them (`left`) and points at the r131 manual merge.
//! - `requirements[]` become `@req:<id> @human` rule scenarios with the
//!   statement preserved verbatim as the scenario description (lossless).
//! - `scenarios[]` rows with GWT content (any non-empty given/when/then) whose
//!   req_id pairs with a defined requirement become `@req:<id> @human` note
//!   scenarios; step keywords render in the project's Gherkin language
//!   (config `bdd.default_language` > config `locale` mapping > any existing
//!   `.feature` `# language:` header > English). Legacy keyword prefixes in
//!   cell content are stripped before re-prefixing. Rows without GWT content
//!   are counted `dropped_notes`; unpaired rows are counted `dropped_unpaired`.
//!   The historical `feature` column no longer branches and is optional — rows
//!   from legacy 5-column sections (`{req_id,id,given,when,then}`) convert the
//!   same way. Each note scenario carries a mechanically synthesized normative
//!   description bullet (`MUST hold: Given …; When …; Then …`) so migrated
//!   output passes `validate --strict`'s locked-rule keyword gate without any
//!   information loss (the GWT steps render verbatim below it).
//! - Row-level reconciliation: every data row under a table section is either
//!   accepted or recorded as malformed (with its 1-based line number). Any
//!   malformed row aborts that capability BEFORE anything is written —
//!   spec.toon is kept, the failure is listed, and the command exits non-zero.
//!   Nothing is dropped silently.
//! - A capability already having `<capability>.feature` is skipped (spec.toon
//!   kept; merge manually and re-run). `spec.toon` is deleted after a
//!   successful write; re-running is a no-op.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::{LLMANSPEC_DIR_NAME, SPEC_FILE};
use crate::sdd::spec::backend::FEATURE_BACKEND;
use crate::sdd::spec::backend::feature_backend::{self, GherkinKw, detect_language, keywords_for};
use crate::sdd::spec::ir::MainSpecDoc;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MigrateArgs {
    pub dry_run: bool,
    #[allow(dead_code)]
    pub force: bool,
    /// Skip the confirmation prompt and apply (for agents/scripts).
    pub yes: bool,
    /// Treat the terminal as non-interactive even when stdin is a TTY.
    pub no_interactive: bool,
}

/// One capability's migration plan.
struct Plan {
    dir: PathBuf,
    capability: String,
    toon: Option<MainSpecDoc>,
    /// Row-level accounting from the legacy reader (drives the
    /// reconcile-before-write gate).
    stats: LegacyParseStats,
}

/// A legacy table row that could not be parsed against its section's declared
/// columns. Migration refuses to proceed while any of these exist: silently
/// dropping rows would corrupt the spec without a trace.
#[derive(Debug)]
struct MalformedRow {
    line: usize,
    section: &'static str,
    reason: String,
}

impl MalformedRow {
    fn describe(&self) -> String {
        format!("line {}: [{}] {}", self.line, self.section, self.reason)
    }
}

/// Row-level parse accounting for the legacy reader. Invariant enforced by the
/// reader: `requirement_rows_seen == requirements.len() + malformed(requirements)`
/// and likewise for scenarios, so "rows seen vs rows accepted" always
/// reconciles through the malformed list.
#[derive(Debug, Default)]
struct LegacyParseStats {
    requirement_rows_seen: usize,
    scenario_rows_seen: usize,
    malformed: Vec<MalformedRow>,
}

/// Per-capability apply outcome for reporting.
enum Outcome {
    Converted(String),
    Skipped(String),
}

pub fn run(args: MigrateArgs) -> Result<()> {
    run_at(Path::new("."), args)
}

pub fn run_at(root: &Path, args: MigrateArgs) -> Result<()> {
    let specs_root = root.join(LLMANSPEC_DIR_NAME).join("specs");
    if !specs_root.is_dir() {
        println!("No specs directory found; nothing to migrate.");
        return Ok(());
    }

    // Phase 1: scan legacy dirs.
    let mut plans: Vec<Plan> = Vec::new();
    for dir in collect_capability_dirs(&specs_root)? {
        let capability = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if !dir.join(SPEC_FILE).exists() {
            continue; // already single-track → idempotent no-op
        }
        let raw = fs::read_to_string(dir.join(SPEC_FILE))
            .with_context(|| format!("read {}", dir.join(SPEC_FILE).display()))?;
        let (doc, stats) =
            parse_legacy_toon(&raw).with_context(|| format!("parse legacy `{capability}`"))?;
        plans.push(Plan {
            dir,
            capability,
            toon: Some(doc),
            stats,
        });
    }

    if plans.is_empty() {
        println!("Nothing to migrate; all capabilities are already single-track.");
        return Ok(());
    }
    println!("Legacy capabilities to migrate: {}", plans.len());
    let lang = detect_gherkin_lang(root, &specs_root);

    if args.dry_run {
        let mut skips = 0usize;
        let mut problems: Vec<String> = Vec::new();
        for p in &plans {
            if p.dir.join(format!("{}.feature", p.capability)).exists() {
                skips += 1;
                println!(
                    "  {} → SKIP ({}.feature already exists; merge spec.toon manually, then re-run)",
                    p.dir.display(),
                    p.capability
                );
                continue;
            }
            let doc = p.toon.as_ref();
            let (converted, notes, unpaired, _) =
                doc.map(classify_note_rows).unwrap_or((0, 0, 0, Vec::new()));
            let left = crate::sdd::spec::validation::discover_features(&p.dir).len();
            let malformed = &p.stats.malformed;
            let malformed_note = if malformed.is_empty() {
                String::new()
            } else {
                format!(", MALFORMED {}", malformed.len())
            };
            println!(
                "  {} → {}.feature (rules {}, toon scenarios → converted {converted}, notes {notes}, unpaired {unpaired}{malformed_note}, left {left} legacy .feature; lang {lang})",
                p.dir.display(),
                p.capability,
                doc.map(|d| d.requirements.len()).unwrap_or(0),
            );
            if !malformed.is_empty() {
                problems.push(format!(
                    "{}: {} malformed legacy row(s) — {}",
                    p.dir.display(),
                    malformed.len(),
                    malformed
                        .iter()
                        .map(MalformedRow::describe)
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
        if !problems.is_empty() {
            eprintln!("\nMalformed legacy rows detected (dry-run):");
            for p in &problems {
                eprintln!("  - {p}");
            }
            return Err(anyhow!(
                "dry-run found malformed legacy rows; fix or remove them in spec.toon before migrating"
            ));
        }
        println!("\n(dry-run: no files written; {skips} capability(s) would be skipped)");
        return Ok(());
    }

    if !args.yes {
        let interactive = crate::sdd::shared::interactive::is_interactive(args.no_interactive);
        if !interactive {
            return Err(anyhow!(
                "non-interactive terminal: re-run with --yes to apply, or --dry-run to preview"
            ));
        }
        let confirmed = inquire::Confirm::new(&format!(
            "Migrate {} capability(s) to the single-track format?",
            plans.len()
        ))
        .with_default(false)
        .prompt()
        .map_err(|e| anyhow!("confirmation prompt failed: {e}"))?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Phase 2: apply.
    let mut converted_count = 0usize;
    let mut skipped_count = 0usize;
    let mut failures = Vec::new();
    for p in &plans {
        match migrate_capability(p, &lang) {
            Ok(Outcome::Converted(summary)) => {
                println!("  ✔ {}: {summary}", p.capability);
                converted_count += 1;
            }
            Ok(Outcome::Skipped(summary)) => {
                println!("  ⚠ {}: {summary}", p.capability);
                skipped_count += 1;
            }
            Err(e) => failures.push(format!("{}: {e:#}", p.capability)),
        }
    }

    if !failures.is_empty() {
        eprintln!("Failures ({}):", failures.len());
        for f in &failures {
            eprintln!("  - {f}");
        }
        return Err(anyhow!(
            "migration completed with {} failure(s): {}",
            failures.len(),
            failures.join("; ")
        ));
    }

    println!("\nMigration complete: {converted_count} converted, {skipped_count} skipped.");
    if skipped_count > 0 {
        println!("Skipped capabilities keep their spec.toon — merge manually, then re-run.");
    }
    println!("Legacy .feature files are left untouched — merge them manually per r131.");
    Ok(())
}

fn migrate_capability(plan: &Plan, lang: &str) -> Result<Outcome> {
    let Some(toon_doc) = &plan.toon else {
        return Ok(Outcome::Skipped("skipped".to_string()));
    };
    let feature_path = plan.dir.join(format!("{}.feature", plan.capability));
    if feature_path.exists() {
        // Main feature already present: writing would either clobber a live
        // harness asset or fork the single-track authority. Keep spec.toon and
        // let the human merge, then re-run.
        return Ok(Outcome::Skipped(format!(
            "skipped — {}.feature already exists; merge spec.toon manually, then re-run",
            plan.capability
        )));
    }
    // Existing .feature files are live harness assets: count only, never touch.
    let left = crate::sdd::spec::validation::discover_features(&plan.dir).len();

    // Reconcile-before-write: any malformed legacy row means the conversion
    // would silently lose spec content. Abort the capability (spec.toon kept,
    // nothing written) and surface the offending line numbers.
    if !plan.stats.malformed.is_empty() {
        anyhow::bail!(
            "{} malformed legacy row(s); nothing written, spec.toon kept — {}",
            plan.stats.malformed.len(),
            plan.stats
                .malformed
                .iter()
                .map(MalformedRow::describe)
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    // Legacy toon `scenarios[]` rows: GWT-bearing rows paired with a defined
    // requirement become `@req:<id> @human` note scenarios (documented
    // constraints, NOT executable acceptance — the executable scenarios live
    // in the untouched legacy .feature files). Rows without GWT content are
    // note noise (`dropped_notes`); GWT rows with an undefined req_id would
    // dangle under `validate --strict` (`dropped_unpaired`).
    let (converted_from_toon, dropped_notes, dropped_unpaired, note_rows) =
        classify_note_rows(toon_doc);

    // Rules come from legacy toon requirements (statement verbatim).
    let doc = MainSpecDoc {
        kind: "llman.sdd.spec".to_string(),
        name: toon_doc.name.trim().to_string(),
        purpose: toon_doc.purpose.clone(),
        valid_scope: toon_doc.valid_scope.clone(),
        requirements: toon_doc.requirements.clone(),
        scenarios: Vec::new(),
    };

    let kw = keywords_for(lang);
    let mut payload = FEATURE_BACKEND.dump_main_spec_lang(&doc, lang)?;
    if !payload.ends_with('\n') {
        payload.push('\n');
    }
    for sc in &note_rows {
        payload.push_str(&render_note_scenario(sc, &kw, lang));
    }
    atomic_write_with_mode(&feature_path, payload.as_bytes(), None)?;

    fs::remove_file(plan.dir.join(SPEC_FILE))
        .with_context(|| format!("remove legacy {}", plan.dir.join(SPEC_FILE).display()))?;

    // r136: the report MUST carry the conversion/accounting split, the
    // left-untouched count, and the rule three-tier initial values.
    let tiers = FEATURE_BACKEND
        .parse_content(&payload, &format!("migrated `{}`", plan.capability))
        .map(|p| feature_backend::compute_rule_morphology(&p));
    let left_note = if left > 0 {
        format!("left {left} legacy .feature file(s) untouched — merge per r131")
    } else {
        format!("left {left} legacy .feature file(s)")
    };
    let base = format!(
        "wrote {} (converted_from_toon {converted_from_toon}; dropped_notes {dropped_notes}; dropped_unpaired {dropped_unpaired}; {left_note}); removed legacy spec.toon",
        feature_path.display(),
    );
    match tiers {
        Ok(t) => Ok(Outcome::Converted(format!(
            "{base}; rules {} enforced {} manual {} pending {}",
            t.rule_count, t.rule_enforced_count, t.rule_manual_count, t.rule_pending_count,
        ))),
        Err(e) => Ok(Outcome::Converted(format!(
            "{base}; tier report unavailable: {e}"
        ))),
    }
}

/// A GWT-bearing toon row rendered as a `@req:<id> @human` note scenario with
/// steps in the target Gherkin language. A mechanically synthesized normative
/// description bullet (`MUST hold: Given …; When …; Then …`) is emitted above
/// the steps so `validate --strict`'s locked-rule keyword gate accepts the
/// scenario (`rule_statement` prefers the description). No information is
/// lost: the bullet restates exactly the step content below it.
fn render_note_scenario(
    sc: &crate::sdd::spec::ir::ScenarioEntry,
    kw: &GherkinKw,
    lang: &str,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  @req:{} @human", sc.req_id);
    let _ = writeln!(out, "  {}: {}", kw.scenario, sc.id);
    let given = flatten_cell(&sc.given);
    let when = flatten_cell(&sc.when_);
    let then = flatten_cell(&sc.then_);
    // Synthesized normative statement: zh projects get 必须, everything else
    // MUST (both accepted by validate's normative-keyword gate).
    if !(given.is_empty() && when.is_empty() && then.is_empty()) {
        let statement = if lang.trim().to_lowercase().starts_with("zh") {
            let mut s = String::from("必须成立：");
            if !given.is_empty() {
                s.push_str(&format!("{} {given}；", kw.given));
            }
            if !when.is_empty() {
                s.push_str(&format!("{} {when}；", kw.when));
            }
            if !then.is_empty() {
                s.push_str(&format!("{} {then}", kw.then));
            }
            s
        } else {
            let mut parts = Vec::new();
            if !given.is_empty() {
                parts.push(format!("Given {given}"));
            }
            if !when.is_empty() {
                parts.push(format!("When {when}"));
            }
            if !then.is_empty() {
                parts.push(format!("Then {then}"));
            }
            format!("MUST hold: {}.", parts.join("; "))
        };
        let _ = writeln!(out, "    - {statement}");
    }
    for (kw_str, value) in [
        (kw.given, given.as_str()),
        (kw.when, when.as_str()),
        (kw.then, then.as_str()),
    ] {
        if !value.is_empty() {
            let _ = writeln!(out, "    {kw_str} {value}");
        }
    }
    out
}

/// Collapse a multi-line cell value to one physical line and strip any legacy
/// keyword prefix before re-prefixing with the target keyword.
fn flatten_cell(field: &str) -> String {
    field
        .split('\n')
        .map(|l| strip_step_keyword(l.trim()))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Shared accounting + row selection for legacy GWT rows: returns the
/// converted/notes/unpaired split and the rows selected as note scenarios.
/// Used identically by dry-run reporting and the apply phase so both paths can
/// never disagree about what would be / was converted.
fn classify_note_rows(
    toon_doc: &MainSpecDoc,
) -> (
    usize,
    usize,
    usize,
    Vec<&crate::sdd::spec::ir::ScenarioEntry>,
) {
    let defined: std::collections::HashSet<&str> = toon_doc
        .requirements
        .iter()
        .map(|r| r.req_id.as_str())
        .collect();
    let mut converted = 0usize;
    let mut notes = 0usize;
    let mut unpaired = 0usize;
    let mut note_rows = Vec::new();
    for sc in &toon_doc.scenarios {
        if !has_gwt_content(sc) {
            notes += 1;
        } else if defined.contains(sc.req_id.as_str()) {
            note_rows.push(sc);
            converted += 1;
        } else {
            unpaired += 1;
        }
    }
    (converted, notes, unpaired, note_rows)
}

/// Whether a toon scenario row carries any GWT content at all.
fn has_gwt_content(sc: &crate::sdd::spec::ir::ScenarioEntry) -> bool {
    !sc.given.trim().is_empty() || !sc.when_.trim().is_empty() || !sc.then_.trim().is_empty()
}

const EN_STEP_KEYWORDS: &[&str] = &["Given", "When", "Then", "And", "But"];
const ZH_STEP_KEYWORDS: &[&str] = &["假如", "当", "那么", "而且", "并且", "但是"];

/// Strip a leading Gherkin step keyword (English or Chinese, followed by
/// whitespace) so content can be re-prefixed in the target language. Requiring
/// trailing whitespace avoids mangling prose that merely starts with the
/// characters (e.g. `当初`).
fn strip_step_keyword(cell: &str) -> &str {
    let all: Vec<&str> = EN_STEP_KEYWORDS
        .iter()
        .chain(ZH_STEP_KEYWORDS)
        .copied()
        .collect();
    for kw in all {
        if let Some(rest) = cell.strip_prefix(kw)
            && rest.starts_with(char::is_whitespace)
        {
            return rest.trim_start();
        }
    }
    cell
}

/// Project Gherkin language for rendered output (r136): config
/// `bdd.default_language` > config `locale` mapping > any existing `.feature`
/// `# language:` header > English.
fn detect_gherkin_lang(root: &Path, specs_root: &Path) -> String {
    if let Ok(Some(cfg)) = crate::sdd::project::config::load_config(&root.join(LLMANSPEC_DIR_NAME))
    {
        if let Some(bdd) = cfg.bdd.as_ref()
            && let Some(dl) = bdd.default_language.as_deref()
            && !dl.trim().is_empty()
        {
            return dl.trim().to_string();
        }
        if !cfg.locale.trim().is_empty() {
            return crate::sdd::spec::validation::locale_to_gherkin_lang(
                Some(cfg.locale.trim()),
                None,
            );
        }
    }
    if let Ok(dirs) = collect_capability_dirs(specs_root) {
        for dir in dirs {
            for f in crate::sdd::spec::validation::discover_features(&dir) {
                if let Ok(raw) = fs::read_to_string(&f) {
                    let sniffed = detect_language(&raw);
                    if !sniffed.trim().is_empty() {
                        return sniffed.trim().to_string();
                    }
                }
            }
        }
    }
    "en".to_string()
}

fn collect_capability_dirs(specs_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let entries =
        fs::read_dir(specs_root).with_context(|| format!("read {}", specs_root.display()))?;
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

/// Minimal reader for the LEGACY strict-TOON spec shape (migration-only).
/// Handles exactly the keys the old writer emitted; anything else fails loudly.
/// Returns row-level stats so migrate can reconcile "rows seen vs rows
/// accepted": every data row under a table section is either parsed into the
/// doc or recorded as malformed (with its 1-based line number) — never dropped
/// silently.
///
/// Escaping: quoted fields accept both historical styles — `""` doubling and
/// backslash escapes (`\"` → literal `"`, `\\` → `\`). Hand-written and
/// third-party legacy files commonly used `\"`; earlier versions of this
/// reader treated each `"` as a quote toggle and shredded such rows.
fn parse_legacy_toon(content: &str) -> Result<(MainSpecDoc, LegacyParseStats)> {
    fn split_row(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                // Backslash escape inside quotes: keep the pair verbatim in
                // the cell and consume the next char so an escaped quote or
                // comma never toggles quoting / splits cells. Decoding happens
                // once, later, in `unquote`.
                '\\' if in_quotes => {
                    cur.push(c);
                    if let Some(&n) = chars.peek() {
                        cur.push(n);
                        chars.next();
                    }
                }
                '"' if in_quotes && chars.peek() == Some(&'"') => {
                    cur.push('"');
                    chars.next();
                }
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => out.push(std::mem::take(&mut cur)),
                _ => cur.push(c),
            }
        }
        out.push(cur);
        out
    }

    fn unquote(v: impl AsRef<str>) -> String {
        let t = v.as_ref().trim();
        let inner = if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
            &t[1..t.len() - 1]
        } else {
            t
        };
        decode_escapes(inner)
    }

    /// Decode legacy backslash escapes left-to-right, exactly once. Only the
    /// two escapes legacy writers emitted are mapped; any other `\x` sequence
    /// is preserved verbatim.
    fn decode_escapes(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.peek() {
                    Some('"') => {
                        out.push('"');
                        chars.next();
                    }
                    Some('\\') => {
                        out.push('\\');
                        chars.next();
                    }
                    _ => out.push(c),
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn push_requirement_row(
        cells: &[String],
        line_no: usize,
        stats: &mut LegacyParseStats,
        requirements: &mut Vec<crate::sdd::spec::ir::RequirementEntry>,
    ) {
        stats.requirement_rows_seen += 1;
        if cells.len() < 3 {
            stats.malformed.push(MalformedRow {
                line: line_no,
                section: "requirements",
                reason: format!(
                    "expected at least 3 cells (req_id,title,statement), got {}",
                    cells.len()
                ),
            });
            return;
        }
        requirements.push(crate::sdd::spec::ir::RequirementEntry {
            req_id: unquote(cells[0].clone()).trim().to_string(),
            title: unquote(cells[1].clone()).trim().to_string(),
            statement: unquote(cells[2..].join(",")),
        });
    }

    fn push_scenario_row(
        cells: &[String],
        line_no: usize,
        declared_cols: Option<&[String]>,
        stats: &mut LegacyParseStats,
        scenarios: &mut Vec<crate::sdd::spec::ir::ScenarioEntry>,
    ) {
        stats.scenario_rows_seen += 1;
        // Resolve the column layout: names declared in the section header win;
        // a header without `{...}` falls back to the historical 5-column GWT
        // shape (`feature` absent → defaults true; the flag never branched).
        const REQUIRED: [&str; 5] = ["req_id", "id", "given", "when", "then"];
        let schema: Vec<String> = declared_cols
            .map(<[String]>::to_vec)
            .unwrap_or_else(|| REQUIRED.iter().map(|s| (*s).to_string()).collect());
        for name in REQUIRED {
            if !schema.iter().any(|c| c.as_str() == name) {
                stats.malformed.push(MalformedRow {
                    line: line_no,
                    section: "scenarios",
                    reason: format!("declared columns {schema:?} missing required column `{name}`"),
                });
                return;
            }
        }
        if cells.len() != schema.len() {
            stats.malformed.push(MalformedRow {
                line: line_no,
                section: "scenarios",
                reason: format!(
                    "expected {} cell(s) per declared columns {schema:?}, got {}",
                    schema.len(),
                    cells.len()
                ),
            });
            return;
        }
        let idx = |name: &str| {
            schema
                .iter()
                .position(|c| c.as_str() == name)
                .expect("required column presence checked above")
        };
        let feature_idx = schema.iter().position(|c| c.as_str() == "feature");
        scenarios.push(crate::sdd::spec::ir::ScenarioEntry {
            req_id: unquote(cells[idx("req_id")].clone()).trim().to_string(),
            id: unquote(cells[idx("id")].clone()).trim().to_string(),
            given: unquote(cells[idx("given")].clone()),
            when_: unquote(cells[idx("when")].clone()),
            then_: unquote(cells[idx("then")].clone()),
            // Historical flag; conversion branches on GWT content, not this.
            // Absent column defaults to true.
            feature: feature_idx
                .map(|i| unquote(cells[i].clone()).trim() != "false")
                .unwrap_or(true),
        });
    }

    /// Column names declared in a section header (`scenarios[2]{a,b,c}:`).
    fn declared_columns(header_line: &str) -> Option<Vec<String>> {
        let open = header_line.find('{')?;
        let close = header_line.rfind('}')?;
        if close < open {
            return None;
        }
        Some(
            header_line[open + 1..close]
                .split(',')
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect(),
        )
    }

    let mut kind = String::new();
    let mut name = String::new();
    let mut purpose = String::new();
    let mut valid_scope: Vec<String> = Vec::new();
    let mut requirements: Vec<crate::sdd::spec::ir::RequirementEntry> = Vec::new();
    let mut scenarios: Vec<crate::sdd::spec::ir::ScenarioEntry> = Vec::new();
    let mut section: Option<&'static str> = None;
    let mut section_cols: Option<Vec<String>> = None;
    let mut stats = LegacyParseStats::default();

    for (line_no, line) in content.lines().enumerate() {
        let line_no = line_no + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indented = line.starts_with(char::is_whitespace);
        if !indented && let Some((key, rest)) = trimmed.split_once(':') {
            match key.trim() {
                "kind" => kind = unquote(rest),
                "name" => name = unquote(rest),
                "purpose" => purpose = unquote(rest),
                k if k.starts_with("valid_scope") => {
                    valid_scope = rest
                        .split(',')
                        .map(|v| unquote(v).trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect();
                    section = None;
                    continue;
                }
                k if k.starts_with("requirements") => {
                    section = Some("requirements");
                    section_cols = declared_columns(trimmed);
                    continue;
                }
                k if k.starts_with("scenarios") => {
                    section = Some("scenarios");
                    section_cols = declared_columns(trimmed);
                    continue;
                }
                _ => {}
            }
        }
        match section {
            Some("requirements") => {
                push_requirement_row(&split_row(trimmed), line_no, &mut stats, &mut requirements)
            }
            Some("scenarios") => push_scenario_row(
                &split_row(trimmed),
                line_no,
                section_cols.as_deref(),
                &mut stats,
                &mut scenarios,
            ),
            _ => {}
        }
    }

    if kind.trim() != "llman.sdd.spec" {
        anyhow::bail!("legacy spec kind must be `llman.sdd.spec`, got `{kind}`");
    }
    Ok((
        MainSpecDoc {
            kind,
            name,
            purpose,
            valid_scope,
            requirements,
            scenarios,
        },
        stats,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> MigrateArgs {
        MigrateArgs {
            dry_run: false,
            force: false,
            yes: true,
            no_interactive: true,
        }
    }

    fn seed_toon(dir: &Path, scenarios: &str) {
        fs::create_dir_all(dir).unwrap();
        let toon = format!(
            "kind: llman.sdd.spec\n\
             name: \"demo\"\n\
             purpose: \"demo purpose\"\n\
             valid_scope[1]: \"src/\"\n\
             requirements[2]{{req_id,title,statement}}:\n\
             \x20 r1,First,\"System MUST do X.\"\n\
             \x20 r2,Second,\"System MUST do Y.\"\n\
             {scenarios}"
        );
        fs::write(dir.join(SPEC_FILE), toon).unwrap();
    }

    fn root_spec_dir(root: &Path) -> PathBuf {
        root.join(LLMANSPEC_DIR_NAME).join("specs").join("demo")
    }

    #[test]
    fn converts_gwt_notes_as_human_in_english_and_is_idempotent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        // No config.yaml and no existing .feature → language falls back to en.
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[2]{req_id,id,given,when,then,feature}:\n",
                "  r1,acc-1,\"precondition ready\",\"run llman sdd validate demo\",\"exit code is zero\",true\n",
                "  r1,note,\"\",\"a trigger\",\"an outcome\",false\n",
            ),
        );

        run_at(root, args()).unwrap();

        assert!(
            !root_spec_dir(root).join(SPEC_FILE).exists(),
            "toon removed"
        );
        let feature = fs::read_to_string(root_spec_dir(root).join("demo.feature")).unwrap();
        assert!(feature.contains("# language: en"));
        assert!(feature.contains("@req:r1 @human"));
        assert!(feature.contains("System MUST do X."));
        // GWT note rows convert as @human (not @executable), English keywords.
        assert!(feature.contains("Scenario: acc-1"));
        assert!(feature.contains("Given precondition ready"));
        assert!(feature.contains("When run llman sdd validate demo"));
        assert!(feature.contains("Then exit code is zero"));
        // feature=false with GWT content still converts (flag is historical).
        assert!(feature.contains("Scenario: note"));
        assert!(!feature.contains("@executable"));

        // Idempotent: second run is a no-op.
        run_at(root, args()).unwrap();

        // Migrated output parses; note rows round-trip as rules.
        let doc = FEATURE_BACKEND
            .parse_main_spec(&feature, "migrated")
            .unwrap();
        assert!(doc.requirements.iter().any(|r| r.req_id == "r1"));
        assert!(
            doc.requirements
                .iter()
                .any(|r| r.title == "acc-1" && r.statement.contains("precondition ready")),
            "note row survives as a rule with synthesized statement"
        );
        assert!(doc.scenarios.is_empty(), "no acceptance scenarios minted");
        // Every migrated rule statement must carry a normative keyword —
        // otherwise the output fails `validate --strict` out of the box.
        for r in &doc.requirements {
            assert!(
                crate::sdd::spec::validation::contains_normative_keyword(&r.statement),
                "rule `{}` statement lacks a normative keyword: {}",
                r.title,
                r.statement
            );
        }
        assert!(
            feature.contains("MUST hold: Given precondition ready"),
            "note scenario carries the synthesized normative bullet"
        );
    }

    #[test]
    fn gwt_presence_decides_conversion_and_accounting() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        // r1/r2 defined; full GWT converts; empty row → note; unpaired GWT →
        // dropped_unpaired. The feature flag must not influence the outcome.
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[3]{req_id,id,given,when,then,feature}:\n",
                "  r1,acc-1,\"pre\",\"trigger\",\"result\",false\n",
                "  r2,empty,\"\",\"\",\"\",true\n",
                "  r404,orphan,\"\",\"trigger\",\"result\",true\n",
            ),
        );

        run_at(root, args()).unwrap();

        let feature = fs::read_to_string(root_spec_dir(root).join("demo.feature")).unwrap();
        assert!(feature.contains("Scenario: acc-1"));
        assert!(!feature.contains("Scenario: empty"), "no-GWT row dropped");
        assert!(
            !feature.contains("Scenario: orphan"),
            "unpaired row dropped"
        );
    }

    #[test]
    fn legacy_five_col_rows_convert_with_feature_defaulting_true() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        // The 5-column section shape ({req_id,id,given,when,then}) was emitted
        // by llman's own historical writer (specs_landing empty template);
        // rows must convert with `feature` defaulting to true.
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[2]{req_id,id,given,when,then}:\n",
                "  r1,demo-row-a,\"\",\"map_id == 1005\",count == 4\n",
                "  r2,demo-row-b,\"\",\"wind > 0\",arrow visible\n",
            ),
        );

        run_at(root, args()).unwrap();

        let feature = fs::read_to_string(root_spec_dir(root).join("demo.feature")).unwrap();
        assert!(feature.contains("Scenario: demo-row-a"));
        assert!(feature.contains("Scenario: demo-row-b"));
        assert!(feature.contains("When map_id == 1005"));
        assert!(feature.contains("Then count == 4"));
    }

    #[test]
    fn backslash_escaped_quotes_survive_conversion() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        // Real-world legacy files escape embedded quotes as \" and freely
        // embed commas inside quoted cells; neither may shred the row.
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[1]{req_id,id,given,when,then,feature}:\n",
                "  r1,demo-row,\"\",\"`map_id == \\\"1005\\\" 且 default_point == \\\"940,657|809\\\"`\",`count == 2`,true\n",
            ),
        );

        run_at(root, args()).unwrap();

        let feature = fs::read_to_string(root_spec_dir(root).join("demo.feature")).unwrap();
        assert!(
            feature.contains(r#"When `map_id == "1005" 且 default_point == "940,657|809"`"#),
            "when cell survives verbatim, got:\n{feature}"
        );
        assert!(
            feature.contains("Then `count == 2`"),
            "then cell must not absorb when fragments"
        );
        assert!(!feature.contains('\\'), "no stray escapes left behind");
    }

    #[test]
    fn malformed_rows_abort_migration_and_preserve_toon() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[2]{req_id,id,given,when,then}:\n",
                "  r1,good,\"\",\"trigger\",result\n",
                "  r1,broken,\"\",\"missing columns\"\n",
            ),
        );

        let err = run_at(root, args()).unwrap_err();

        let msg = format!("{err:#}");
        assert!(msg.contains("malformed"), "got: {msg}");
        assert!(
            msg.contains("line "),
            "offending line number reported: {msg}"
        );
        // Nothing written: spec.toon kept, no .feature minted.
        assert!(root_spec_dir(root).join(SPEC_FILE).exists());
        assert!(!root_spec_dir(root).join("demo.feature").exists());
    }

    #[test]
    fn dry_run_fails_loudly_on_malformed_rows() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[1]{req_id,id,given,when,then}:\n",
                "  r1,broken,\"\",\"only four cells here\"\n",
            ),
        );
        let dry_args = MigrateArgs {
            dry_run: true,
            ..args()
        };

        let err = run_at(root, dry_args).unwrap_err();

        let msg = format!("{err:#}");
        assert!(msg.contains("malformed legacy rows"), "got: {msg}");
    }

    #[test]
    fn parse_legacy_handles_escapes_and_column_layouts() {
        let raw = concat!(
            "kind: llman.sdd.spec\n",
            "name: \"sample-a\"\n",
            "purpose: minimal repro\n",
            "valid_scope[1]: game/\n",
            "requirements[1]{req_id,title,statement}:\n",
            "  r1,Demo rule,\"System MUST parse \\\"x,y|x,y|...\\\" format.\"\n",
            "scenarios[1]{req_id,id,given,when,then,feature}:\n",
            "  r1,demo-row,\"\",\"`map_id == \\\"1005\\\" 且 default_point == \\\"940,657|809\\\"`\",`count == 2`,true\n",
        );
        let (doc, stats) = parse_legacy_toon(raw).unwrap();
        assert!(stats.malformed.is_empty(), "got: {:?}", stats.malformed);
        assert_eq!(stats.scenario_rows_seen, 1);
        assert_eq!(
            doc.requirements[0].statement,
            "System MUST parse \"x,y|x,y|...\" format."
        );
        let sc = &doc.scenarios[0];
        assert_eq!(
            sc.when_,
            "`map_id == \"1005\" 且 default_point == \"940,657|809\"`"
        );
        assert_eq!(sc.then_, "`count == 2`");
        assert!(sc.feature);
    }

    #[test]
    fn parse_legacy_flags_malformed_rows_with_line_numbers() {
        let raw = concat!(
            "kind: llman.sdd.spec\n",
            "name: x\n",
            "purpose: p\n",
            "valid_scope[1]: game/\n",
            "requirements[1]{req_id,title,statement}:\n",
            "  r1,Demo rule,System MUST demo.\n",
            "scenarios[3]{req_id,id,given,when,then}:\n",
            "  r1,a,\"\",\"t1\",ok\n",
            "  r1,broken,\"\",\"only four\"\n",
        );
        let (doc, stats) = parse_legacy_toon(raw).unwrap();
        assert_eq!(doc.scenarios.len(), 1);
        assert_eq!(stats.scenario_rows_seen, 2);
        assert_eq!(stats.malformed.len(), 1);
        assert_eq!(stats.malformed[0].line, 9);
        assert!(stats.malformed[0].reason.contains("expected 5"));
    }

    #[test]
    fn existing_features_left_untouched() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let dir = root_spec_dir(root);
        seed_toon(&dir, "scenarios[0]:\n");
        let legacy = dir.join("legacy-acc.feature");
        let legacy_body = concat!(
            "# language: en\n",
            "Feature: demo legacy acceptance\n",
            "  @req:r1 @executable\n",
            "  Scenario: legacy-acc\n",
            "    Given seeded\n",
            "    When noop\n",
            "    Then ok\n",
        );
        fs::write(&legacy, legacy_body).unwrap();

        run_at(root, args()).unwrap();

        assert_eq!(
            fs::read_to_string(&legacy).unwrap(),
            legacy_body,
            "legacy .feature must be byte-identical"
        );
        assert!(dir.join("demo.feature").exists());
        assert!(!dir.join(SPEC_FILE).exists());
    }

    #[test]
    fn skips_when_main_feature_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let dir = root_spec_dir(root);
        seed_toon(&dir, "scenarios[0]:\n");
        let main_body = "# language: en\n# capability: demo\nFeature: demo\n  @req:r1 @human\n  Scenario: R1\n    - System MUST do X.\n";
        fs::write(dir.join("demo.feature"), main_body).unwrap();

        run_at(root, args()).unwrap();

        assert!(
            dir.join(SPEC_FILE).exists(),
            "skipped capability keeps its spec.toon"
        );
        assert_eq!(
            fs::read_to_string(dir.join("demo.feature")).unwrap(),
            main_body,
            "main feature untouched"
        );
    }

    #[test]
    fn language_prefers_bdd_default_then_locale_then_sniff() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let llmanspec = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec).unwrap();

        // bdd.default_language beats locale.
        fs::write(
            llmanspec.join("config.yaml"),
            "schema: spec-driven\nlocale: en\nbdd:\n  default_language: zh-CN\n  run_command: \"cargo test\"\n",
        )
        .unwrap();
        let specs_root = llmanspec.join("specs");
        assert_eq!(detect_gherkin_lang(root, &specs_root), "zh-CN");

        // locale mapping applies without bdd.default_language.
        fs::write(
            llmanspec.join("config.yaml"),
            "schema: spec-driven\nlocale: zh-Hans\n",
        )
        .unwrap();
        assert_eq!(detect_gherkin_lang(root, &specs_root), "zh-CN");

        // No config at all → sniff the first existing .feature header.
        fs::remove_file(llmanspec.join("config.yaml")).unwrap();
        let sniff_dir = specs_root.join("demo");
        fs::create_dir_all(&sniff_dir).unwrap();
        fs::write(
            sniff_dir.join("demo.feature"),
            "# language: zh-CN\nFeature: demo\n",
        )
        .unwrap();
        assert_eq!(detect_gherkin_lang(root, &specs_root), "zh-CN");

        // Nothing to go on → English.
        fs::remove_file(sniff_dir.join("demo.feature")).unwrap();
        assert_eq!(detect_gherkin_lang(root, &specs_root), "en");
    }

    #[test]
    fn zh_config_renders_zh_keywords_and_strips_legacy_prefixes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let llmanspec = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec).unwrap();
        fs::write(
            llmanspec.join("config.yaml"),
            "schema: spec-driven\nlocale: zh-Hans\n",
        )
        .unwrap();
        seed_toon(
            &root_spec_dir(root),
            concat!(
                "scenarios[1]{req_id,id,given,when,then,feature}:\n",
                "  r1,acc-1,\"Given legacy precondition\",\"When legacy trigger\",\"Then legacy result\",true\n",
            ),
        );

        run_at(root, args()).unwrap();

        let feature = fs::read_to_string(root_spec_dir(root).join("demo.feature")).unwrap();
        assert!(feature.contains("# language: zh-CN"));
        assert!(feature.contains("场景: acc-1"));
        assert!(feature.contains("必须成立：假如 legacy precondition"));
        assert!(feature.contains("假如 legacy precondition"));
        assert!(feature.contains("当 legacy trigger"));
        assert!(feature.contains("那么 legacy result"));
    }

    #[test]
    fn strip_step_keyword_requires_trailing_whitespace() {
        assert_eq!(strip_step_keyword("Given foo"), "foo");
        assert_eq!(strip_step_keyword("假如 foo"), "foo");
        assert_eq!(strip_step_keyword("当初只是设想"), "当初只是设想");
        assert_eq!(strip_step_keyword("Given"), "Given");
        assert_eq!(strip_step_keyword("plain"), "plain");
    }
}
