use crate::fs_utils::atomic_write_with_mode;
use crate::path_utils::safe_parent_for_creation;
use crate::skills::shared::git::find_git_root;
use crate::tool::command::{SyncIgnoreArgs, SyncIgnoreTarget};
use anyhow::{Context, Result, bail};
use llm_json::{RepairOptions, loads};
use rust_i18n::t;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IgnoreRules {
    pub ignore: BTreeSet<String>,
    pub include: BTreeSet<String>,
}

impl IgnoreRules {
    pub fn is_empty(&self) -> bool {
        self.ignore.is_empty() && self.include.is_empty()
    }

    pub fn union_inplace(&mut self, other: &IgnoreRules) {
        self.ignore.extend(other.ignore.iter().cloned());
        self.include.extend(other.include.iter().cloned());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    OpenCodeIgnore,
    CursorIgnore,
    ClaudeShared,
    ClaudeLocal,
}

impl Target {
    pub fn path(self, root: &Path) -> PathBuf {
        match self {
            Target::OpenCodeIgnore => root.join(".ignore"),
            Target::CursorIgnore => root.join(".cursorignore"),
            Target::ClaudeShared => root.join(".claude/settings.json"),
            Target::ClaudeLocal => root.join(".claude/settings.local.json"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Target::OpenCodeIgnore => ".ignore",
            Target::CursorIgnore => ".cursorignore",
            Target::ClaudeShared => ".claude/settings.json",
            Target::ClaudeLocal => ".claude/settings.local.json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanAction {
    Create,
    Update,
    Unchanged,
    Delete,
}

impl PlanAction {
    fn label(self) -> String {
        match self {
            PlanAction::Create => t!("tool.sync_ignore.preview.action.create").to_string(),
            PlanAction::Update => t!("tool.sync_ignore.preview.action.update").to_string(),
            PlanAction::Unchanged => t!("tool.sync_ignore.preview.action.unchanged").to_string(),
            PlanAction::Delete => t!("tool.sync_ignore.preview.action.delete").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct SyncNote {
    kind: SyncNoteKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncNoteKind {
    Info,
    Warning,
}

#[derive(Debug, Clone)]
struct TargetPlan {
    target: Target,
    path: PathBuf,
    action: PlanAction,
    content: Option<Vec<u8>>,
    added_claude_rules: Vec<String>,
    notes: Vec<SyncNote>,
}

pub fn run(args: &SyncIgnoreArgs) -> Result<()> {
    println!("{}", t!("tool.sync_ignore.start"));

    let cwd = std::env::current_dir().context("get current directory")?;
    let root = resolve_root(&cwd, args.force)?;

    if args.interactive {
        return run_interactive(args, &root);
    }

    run_non_interactive(args, &root)
}

fn resolve_root(cwd: &Path, force: bool) -> Result<PathBuf> {
    if force {
        return Ok(cwd.to_path_buf());
    }

    if let Some(root) = find_git_root(cwd) {
        return Ok(root);
    }

    bail!(t!("tool.sync_ignore.error.no_git_repo"));
}

fn run_non_interactive(args: &SyncIgnoreArgs, root: &Path) -> Result<()> {
    let sources = discover_sources(root, &args.input)?;
    let (union, parse_notes) = load_union_rules(&sources, args.verbose)?;
    let targets = resolve_targets(args, root)?;

    let plans = build_plans(root, &union, &targets, args.verbose)?;
    print_preview(
        root,
        &sources,
        &union,
        &parse_notes,
        &plans,
        args.verbose,
        true,
    );

    if !args.yes {
        println!("{}", t!("tool.sync_ignore.dry_run_hint"));
        return Ok(());
    }

    apply_plans(&plans)?;
    println!("{}", t!("tool.sync_ignore.apply_done"));
    Ok(())
}

fn run_interactive(args: &SyncIgnoreArgs, root: &Path) -> Result<()> {
    use inquire::{Confirm, MultiSelect};

    let sources = discover_sources(root, &args.input)?;
    let (union, parse_notes) = load_union_rules(&sources, args.verbose)?;

    let targets_all = [
        Target::OpenCodeIgnore,
        Target::CursorIgnore,
        Target::ClaudeShared,
        Target::ClaudeLocal,
    ];

    let defaults: Vec<Target> = {
        let mut out = vec![
            Target::OpenCodeIgnore,
            Target::CursorIgnore,
            Target::ClaudeShared,
        ];
        if root.join(".claude/settings.local.json").exists() {
            out.push(Target::ClaudeLocal);
        }
        out
    };

    let options: Vec<String> = targets_all
        .iter()
        .map(|target| {
            let path = target.path(root);
            let status_key = if path.exists() {
                "tool.sync_ignore.interactive.exists"
            } else {
                "tool.sync_ignore.interactive.missing"
            };
            format!("{} ({})", target.label(), t!(status_key))
        })
        .collect();

    let select_prompt = t!("tool.sync_ignore.interactive.select_targets").to_string();
    let mut selector = MultiSelect::new(&select_prompt, options);
    let default_indices: Vec<usize> = targets_all
        .iter()
        .enumerate()
        .filter_map(|(idx, target)| defaults.contains(target).then_some(idx))
        .collect();
    selector = selector.with_default(&default_indices);

    let selected = selector.prompt()?;
    let selected_targets: Vec<Target> = selected
        .into_iter()
        .filter_map(|item| {
            targets_all
                .iter()
                .find(|tgt| item.starts_with(tgt.label()))
                .copied()
        })
        .collect();

    let deselected_existing: Vec<Target> = targets_all
        .iter()
        .copied()
        .filter(|target| target.path(root).exists() && !selected_targets.contains(target))
        .collect();

    let mut delete_targets = Vec::new();
    for target in deselected_existing {
        let prompt = t!(
            "tool.sync_ignore.interactive.confirm_delete",
            path = target.label()
        );
        let should_delete = Confirm::new(&prompt).with_default(false).prompt()?;
        if should_delete {
            delete_targets.push(target);
        }
    }

    let mut plans = build_plans(root, &union, &selected_targets, args.verbose)?;
    for target in delete_targets {
        plans.push(TargetPlan {
            target,
            path: target.path(root),
            action: PlanAction::Delete,
            content: None,
            added_claude_rules: Vec::new(),
            notes: Vec::new(),
        });
    }

    print_preview(
        root,
        &sources,
        &union,
        &parse_notes,
        &plans,
        args.verbose,
        false,
    );

    let should_apply = Confirm::new(&t!("tool.sync_ignore.interactive.confirm_apply"))
        .with_default(false)
        .prompt()?;
    if !should_apply {
        println!("{}", t!("tool.sync_ignore.cancelled"));
        return Ok(());
    }

    apply_plans(&plans)?;
    println!("{}", t!("tool.sync_ignore.apply_done"));
    Ok(())
}

fn discover_sources(root: &Path, extra_inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    let candidates = [
        root.join(".ignore"),
        root.join(".cursorignore"),
        root.join(".claude/settings.json"),
        root.join(".claude/settings.local.json"),
    ];
    for path in candidates {
        if path.exists() {
            sources.push(path);
        }
    }

    for input in extra_inputs {
        let path = if input.is_absolute() {
            input.clone()
        } else {
            root.join(input)
        };
        if path.file_name().and_then(|name| name.to_str()) == Some(".gitignore") {
            bail!(t!(
                "tool.sync_ignore.error.gitignore_not_supported",
                path = path.display()
            ));
        }
        if path.exists() && !sources.contains(&path) {
            sources.push(path);
        }
    }

    Ok(sources)
}

fn load_union_rules(sources: &[PathBuf], verbose: bool) -> Result<(IgnoreRules, Vec<SyncNote>)> {
    let mut union = IgnoreRules::default();
    let mut notes = Vec::new();

    if sources.is_empty() {
        notes.push(SyncNote {
            kind: SyncNoteKind::Info,
            message: t!("tool.sync_ignore.note.no_sources").to_string(),
        });
        return Ok((union, notes));
    }

    for path in sources {
        let content = fs::read_to_string(path)
            .with_context(|| t!("tool.sync_ignore.error.read_failed", path = path.display()))?;
        let (rules, mut rule_notes) = if looks_like_json(path) {
            if !looks_like_claude_settings(path) {
                bail!(t!(
                    "tool.sync_ignore.error.unsupported_json_input",
                    path = path.display()
                ));
            }
            parse_claude_settings(&content, verbose)?
        } else {
            (parse_gitignore_like(&content), Vec::new())
        };
        union.union_inplace(&rules);
        notes.append(&mut rule_notes);
    }

    Ok((union, notes))
}

fn looks_like_json(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

fn looks_like_claude_settings(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !file_name.ends_with(".json") {
        return false;
    }

    let in_claude_dir = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".claude");

    in_claude_dir && file_name.starts_with("settings")
}

fn parse_gitignore_like(content: &str) -> IgnoreRules {
    let mut out = IgnoreRules::default();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('!') {
            let pattern = rest.trim();
            if !pattern.is_empty() {
                out.include.insert(pattern.to_string());
            }
            continue;
        }
        out.ignore.insert(line.to_string());
    }
    out
}

fn parse_claude_settings(content: &str, verbose: bool) -> Result<(IgnoreRules, Vec<SyncNote>)> {
    let mut notes = Vec::new();

    let settings: Value = match serde_json::from_str(content) {
        Ok(value) => value,
        Err(_) => loads(content, &RepairOptions::default())
            .with_context(|| t!("tool.sync_ignore.error.parse_claude_failed").to_string())?,
    };

    if verbose && settings.get("permissions").is_none() {
        notes.push(SyncNote {
            kind: SyncNoteKind::Info,
            message: t!("tool.sync_ignore.note.claude_missing_permissions").to_string(),
        });
    }

    let mut out = IgnoreRules::default();
    let deny = settings
        .get("permissions")
        .and_then(|p| p.get("deny"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for item in deny {
        let Some(rule) = item.as_str() else {
            continue;
        };
        if let Some(pattern) = extract_read_pattern(rule) {
            out.ignore.insert(pattern);
        } else if verbose {
            notes.push(SyncNote {
                kind: SyncNoteKind::Info,
                message: t!("tool.sync_ignore.note.claude_skip_non_read", rule = rule).to_string(),
            });
        }
    }

    Ok((out, notes))
}

fn extract_read_pattern(rule: &str) -> Option<String> {
    let trimmed = rule.trim();
    let inner = trimmed.strip_prefix("Read(")?.strip_suffix(')')?;
    let inner_trimmed = inner.trim();
    let mut pattern = inner_trimmed;
    if let Some(rest) = pattern.strip_prefix("./") {
        pattern = rest;
    }
    if let Some(rest) = pattern.strip_prefix('/') {
        pattern = rest;
    }
    let normalized = pattern.trim();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized.to_string())
}

fn resolve_targets(args: &SyncIgnoreArgs, root: &Path) -> Result<Vec<Target>> {
    if args.target.is_empty() {
        let mut targets = vec![
            Target::OpenCodeIgnore,
            Target::CursorIgnore,
            Target::ClaudeShared,
        ];
        if root.join(".claude/settings.local.json").exists() {
            targets.push(Target::ClaudeLocal);
        }
        return Ok(targets);
    }

    let mut out = Vec::new();
    for target in &args.target {
        match target {
            SyncIgnoreTarget::Opencode => push_unique(&mut out, Target::OpenCodeIgnore),
            SyncIgnoreTarget::Cursor => push_unique(&mut out, Target::CursorIgnore),
            SyncIgnoreTarget::ClaudeShared => push_unique(&mut out, Target::ClaudeShared),
            SyncIgnoreTarget::ClaudeLocal => push_unique(&mut out, Target::ClaudeLocal),
            SyncIgnoreTarget::All => {
                push_unique(&mut out, Target::OpenCodeIgnore);
                push_unique(&mut out, Target::CursorIgnore);
                push_unique(&mut out, Target::ClaudeShared);
                push_unique(&mut out, Target::ClaudeLocal);
            }
        }
    }

    Ok(out)
}

fn push_unique(out: &mut Vec<Target>, target: Target) {
    if !out.contains(&target) {
        out.push(target);
    }
}

fn build_plans(
    root: &Path,
    union: &IgnoreRules,
    targets: &[Target],
    verbose: bool,
) -> Result<Vec<TargetPlan>> {
    let mut plans = Vec::new();
    for target in targets {
        let path = target.path(root);
        let plan = match target {
            Target::OpenCodeIgnore | Target::CursorIgnore => {
                build_gitignore_like_plan(*target, &path, union)?
            }
            Target::ClaudeShared | Target::ClaudeLocal => {
                build_claude_plan(*target, &path, union, verbose)?
            }
        };
        plans.push(plan);
    }
    Ok(plans)
}

fn build_gitignore_like_plan(
    target: Target,
    path: &Path,
    union: &IgnoreRules,
) -> Result<TargetPlan> {
    let content = render_gitignore_like(union);
    let desired = content.into_bytes();

    let action = if path.exists() {
        let current = fs::read_to_string(path)
            .with_context(|| t!("tool.sync_ignore.error.read_failed", path = path.display()))?;
        if normalize_newlines(&current)
            == normalize_newlines(std::str::from_utf8(&desired).unwrap_or_default())
        {
            PlanAction::Unchanged
        } else {
            PlanAction::Update
        }
    } else if union.is_empty() {
        PlanAction::Unchanged
    } else {
        PlanAction::Create
    };

    Ok(TargetPlan {
        target,
        path: path.to_path_buf(),
        action,
        content: (action != PlanAction::Unchanged).then_some(desired),
        added_claude_rules: Vec::new(),
        notes: Vec::new(),
    })
}

fn build_claude_plan(
    target: Target,
    path: &Path,
    union: &IgnoreRules,
    verbose: bool,
) -> Result<TargetPlan> {
    let mut notes = Vec::new();
    if !union.include.is_empty() {
        notes.push(SyncNote {
            kind: SyncNoteKind::Warning,
            message: t!(
                "tool.sync_ignore.warn.include_not_supported_in_claude",
                count = union.include.len()
            )
            .to_string(),
        });
    }

    let desired_read_rules = render_claude_read_rules(&union.ignore);
    if desired_read_rules.is_empty() && !path.exists() {
        return Ok(TargetPlan {
            target,
            path: path.to_path_buf(),
            action: PlanAction::Unchanged,
            content: None,
            added_claude_rules: Vec::new(),
            notes,
        });
    }

    let existing = read_claude_settings(path, verbose)?;
    let (merged, added) = merge_deny_rules(&existing.deny, &desired_read_rules);

    let action = if !path.exists() {
        PlanAction::Create
    } else if added.is_empty() {
        PlanAction::Unchanged
    } else {
        PlanAction::Update
    };

    let content = if action == PlanAction::Unchanged {
        None
    } else {
        let updated = build_updated_claude_settings(existing.value, merged.clone());
        let pretty = serde_json::to_string_pretty(&updated)
            .with_context(|| t!("tool.sync_ignore.error.serialize_failed"))?;
        let mut pretty_with_newline = pretty;
        pretty_with_newline.push('\n');
        let patched = if let Some(original) = existing.original.as_deref() {
            match patch_permissions_deny_array(original, &merged) {
                Some(patched) => patched,
                None => {
                    notes.push(SyncNote {
                        kind: SyncNoteKind::Warning,
                        message: t!(
                            "tool.sync_ignore.warn.claude_jsonc_patch_fallback",
                            path = target.label()
                        )
                        .to_string(),
                    });
                    pretty_with_newline
                }
            }
        } else {
            pretty_with_newline
        };
        Some(patched.into_bytes())
    };

    Ok(TargetPlan {
        target,
        path: path.to_path_buf(),
        action,
        content,
        added_claude_rules: added,
        notes,
    })
}

fn render_gitignore_like(rules: &IgnoreRules) -> String {
    let mut out = String::new();
    for pattern in &rules.ignore {
        out.push_str(pattern);
        out.push('\n');
    }
    for pattern in &rules.include {
        out.push('!');
        out.push_str(pattern);
        out.push('\n');
    }
    out
}

fn render_claude_read_rules(ignore: &BTreeSet<String>) -> Vec<String> {
    ignore
        .iter()
        .map(|pattern| {
            let mut normalized = pattern.as_str();
            if let Some(rest) = normalized.strip_prefix("./") {
                normalized = rest;
            }
            if let Some(rest) = normalized.strip_prefix('/') {
                normalized = rest;
            }
            format!("Read(./{})", normalized)
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ClaudeSettingsRead {
    original: Option<String>,
    value: Value,
    deny: Vec<String>,
}

fn read_claude_settings(path: &Path, verbose: bool) -> Result<ClaudeSettingsRead> {
    if !path.exists() {
        return Ok(ClaudeSettingsRead {
            original: None,
            value: json!({}),
            deny: Vec::new(),
        });
    }

    let original = fs::read_to_string(path)
        .with_context(|| t!("tool.sync_ignore.error.read_failed", path = path.display()))?;

    let value: Value = match serde_json::from_str(&original) {
        Ok(value) => value,
        Err(_) => loads(&original, &RepairOptions::default())
            .with_context(|| t!("tool.sync_ignore.error.parse_claude_failed").to_string())?,
    };

    let deny = value
        .get("permissions")
        .and_then(|p| p.get("deny"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    if verbose && value.get("permissions").is_none() {
        eprintln!("{}", t!("tool.sync_ignore.note.claude_missing_permissions"));
    }

    Ok(ClaudeSettingsRead {
        original: Some(original),
        value,
        deny,
    })
}

fn merge_deny_rules(
    existing: &[String],
    desired_read_rules: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for item in existing {
        if seen.insert(item.clone()) {
            merged.push(item.clone());
        }
    }

    let mut added = Vec::new();
    for item in desired_read_rules {
        if seen.insert(item.clone()) {
            merged.push(item.clone());
            added.push(item.clone());
        }
    }

    (merged, added)
}

fn build_updated_claude_settings(mut value: Value, deny: Vec<String>) -> Value {
    if !value.is_object() {
        value = json!({});
    }

    let permissions = value
        .as_object_mut()
        .expect("value is object")
        .entry("permissions")
        .or_insert_with(|| json!({}));

    if !permissions.is_object() {
        *permissions = json!({});
    }

    if let Some(obj) = permissions.as_object_mut() {
        obj.insert(
            "deny".to_string(),
            Value::Array(deny.into_iter().map(Value::String).collect()),
        );
    }

    value
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn apply_plans(plans: &[TargetPlan]) -> Result<()> {
    for plan in plans {
        match plan.action {
            PlanAction::Unchanged => continue,
            PlanAction::Delete => {
                if plan.path.exists() {
                    fs::remove_file(&plan.path).with_context(|| {
                        t!(
                            "tool.sync_ignore.error.delete_failed",
                            path = plan.path.display()
                        )
                    })?;
                }
            }
            PlanAction::Create | PlanAction::Update => {
                let Some(content) = &plan.content else {
                    continue;
                };
                if let Some(parent) = safe_parent_for_creation(&plan.path) {
                    fs::create_dir_all(parent).with_context(|| {
                        t!(
                            "tool.sync_ignore.error.create_dir_failed",
                            path = parent.display()
                        )
                    })?;
                }
                let mode = desired_write_mode(&plan.path, plan.action);
                atomic_write_with_mode(&plan.path, content, mode).with_context(|| {
                    t!(
                        "tool.sync_ignore.error.write_failed",
                        path = plan.path.display()
                    )
                })?;
            }
        }
    }
    Ok(())
}

fn desired_write_mode(path: &Path, action: PlanAction) -> Option<u32> {
    let default_mode = 0o644;
    match action {
        PlanAction::Create => Some(default_mode),
        PlanAction::Update => existing_file_mode(path).or(Some(default_mode)),
        PlanAction::Unchanged | PlanAction::Delete => None,
    }
}

fn existing_file_mode(path: &Path) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).ok()?;
        Some(metadata.permissions().mode() & 0o777)
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

fn print_preview(
    root: &Path,
    sources: &[PathBuf],
    union: &IgnoreRules,
    global_notes: &[SyncNote],
    plans: &[TargetPlan],
    verbose: bool,
    show_dry_run_hint: bool,
) {
    println!(
        "{}",
        t!("tool.sync_ignore.preview.root", path = root.display())
    );

    println!("{}", t!("tool.sync_ignore.preview.sources_title"));
    if sources.is_empty() {
        println!("  - {}", t!("tool.sync_ignore.preview.sources_none"));
    } else {
        for src in sources {
            println!("  - {}", src.display());
        }
    }

    println!(
        "{}",
        t!(
            "tool.sync_ignore.preview.union_stats",
            ignore = union.ignore.len(),
            include = union.include.len()
        )
    );

    println!("{}", t!("tool.sync_ignore.preview.targets_title"));
    println!("{}", render_preview_table(union, plans));

    print_preview_notes(global_notes, plans, verbose);
    print_preview_details(plans, verbose);

    if show_dry_run_hint {
        println!("{}", t!("tool.sync_ignore.preview.dry_run_default"));
    }
}

fn render_preview_table(union: &IgnoreRules, plans: &[TargetPlan]) -> String {
    use comfy_table::{Cell, ContentArrangement, Table};

    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new(t!("tool.sync_ignore.preview.table.header.target").to_string()),
        Cell::new(t!("tool.sync_ignore.preview.table.header.action").to_string()),
        Cell::new(t!("tool.sync_ignore.preview.table.header.ignore").to_string()),
        Cell::new(t!("tool.sync_ignore.preview.table.header.include").to_string()),
        Cell::new(t!("tool.sync_ignore.preview.table.header.notes").to_string()),
    ]);

    for plan in plans {
        let (ignore_count, include_count) = plan_written_rule_counts(union, plan);

        let ignore_cell = if plan.action == PlanAction::Delete {
            t!("tool.sync_ignore.preview.table.value.none").to_string()
        } else {
            ignore_count.to_string()
        };
        let include_cell = if plan.action == PlanAction::Delete {
            t!("tool.sync_ignore.preview.table.value.none").to_string()
        } else {
            include_count.to_string()
        };

        table.add_row(vec![
            Cell::new(plan.target.label()),
            Cell::new(plan.action.label()),
            Cell::new(ignore_cell),
            Cell::new(include_cell),
            Cell::new(plan_notes_summary(plan)),
        ]);
    }

    table.to_string()
}

fn plan_written_rule_counts(union: &IgnoreRules, plan: &TargetPlan) -> (usize, usize) {
    match plan.target {
        Target::OpenCodeIgnore | Target::CursorIgnore => (union.ignore.len(), union.include.len()),
        Target::ClaudeShared | Target::ClaudeLocal => (union.ignore.len(), 0),
    }
}

fn plan_notes_summary(plan: &TargetPlan) -> String {
    let mut parts = Vec::new();

    if matches!(plan.target, Target::ClaudeShared | Target::ClaudeLocal)
        && !plan.added_claude_rules.is_empty()
    {
        parts.push(
            t!(
                "tool.sync_ignore.preview.notes.claude_added",
                count = plan.added_claude_rules.len()
            )
            .to_string(),
        );
    }

    let warn_count = plan
        .notes
        .iter()
        .filter(|note| matches!(note.kind, SyncNoteKind::Warning))
        .count();
    if warn_count > 0 {
        parts.push(
            t!(
                "tool.sync_ignore.preview.notes.warn_count",
                count = warn_count
            )
            .to_string(),
        );
    }

    if parts.is_empty() {
        t!("tool.sync_ignore.preview.table.value.none").to_string()
    } else {
        parts.join(", ")
    }
}

fn print_preview_notes(global_notes: &[SyncNote], plans: &[TargetPlan], verbose: bool) {
    let warnings = collect_note_messages(global_notes, plans, SyncNoteKind::Warning);
    if !warnings.is_empty() {
        println!();
        println!("{}", t!("tool.sync_ignore.preview.warnings_title"));
        for message in warnings {
            println!("  - {message}");
        }
    }

    if verbose {
        let infos = collect_note_messages(global_notes, plans, SyncNoteKind::Info);
        if !infos.is_empty() {
            println!();
            println!("{}", t!("tool.sync_ignore.preview.notes_title"));
            for message in infos {
                println!("  - {message}");
            }
        }
    }
}

fn collect_note_messages(
    global_notes: &[SyncNote],
    plans: &[TargetPlan],
    kind: SyncNoteKind,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for note in global_notes {
        if note.kind == kind {
            out.insert(note.message.clone());
        }
    }
    for plan in plans {
        for note in &plan.notes {
            if note.kind == kind {
                out.insert(note.message.clone());
            }
        }
    }
    out
}

fn print_preview_details(plans: &[TargetPlan], verbose: bool) {
    const MAX_LINES: usize = 60;
    const MAX_ITEMS: usize = 60;

    for plan in plans {
        match plan.action {
            PlanAction::Create | PlanAction::Update => match plan.target {
                Target::OpenCodeIgnore | Target::CursorIgnore => {
                    let Some(content) = &plan.content else {
                        continue;
                    };
                    println!();
                    println!(
                        "{}",
                        t!(
                            "tool.sync_ignore.preview.detail_title",
                            path = plan.target.label()
                        )
                    );
                    let text = String::from_utf8_lossy(content);
                    let lines: Vec<&str> = text.lines().collect();
                    let limit = if verbose { lines.len() } else { MAX_LINES };
                    for line in lines.iter().take(limit) {
                        println!("{line}");
                    }
                    if !verbose && lines.len() > limit {
                        println!(
                            "{}",
                            t!(
                                "tool.sync_ignore.preview.truncated_lines",
                                count = lines.len() - limit
                            )
                        );
                    }
                }
                Target::ClaudeShared | Target::ClaudeLocal => {
                    if plan.added_claude_rules.is_empty() {
                        continue;
                    }
                    println!();
                    println!(
                        "{}",
                        t!(
                            "tool.sync_ignore.preview.claude_added_title",
                            path = plan.target.label(),
                            count = plan.added_claude_rules.len()
                        )
                    );
                    let limit = if verbose {
                        plan.added_claude_rules.len()
                    } else {
                        MAX_ITEMS
                    };
                    for item in plan.added_claude_rules.iter().take(limit) {
                        println!("  - {item}");
                    }
                    if !verbose && plan.added_claude_rules.len() > limit {
                        println!(
                            "{}",
                            t!(
                                "tool.sync_ignore.preview.truncated_items",
                                count = plan.added_claude_rules.len() - limit
                            )
                        );
                    }
                }
            },
            PlanAction::Delete => {
                println!();
                println!(
                    "{}",
                    t!(
                        "tool.sync_ignore.preview.delete_title",
                        path = plan.target.label()
                    )
                );
            }
            PlanAction::Unchanged => {}
        }
    }
}

fn patch_permissions_deny_array(original: &str, deny: &[String]) -> Option<String> {
    let root_start = original.find('{')?;
    let root_end = find_matching_bracket(original, root_start, b'{', b'}')?;
    let root_content_start = root_start + 1;
    let root_content_end = root_end;

    let permissions_value_start = find_object_member_value_start(
        original,
        root_content_start,
        root_content_end,
        "permissions",
    )?;
    let permissions_value_start = skip_ws_and_comments(original, permissions_value_start);
    if original.as_bytes().get(permissions_value_start)? != &b'{' {
        return None;
    }
    let permissions_end = find_matching_bracket(original, permissions_value_start, b'{', b'}')?;
    let permissions_content_start = permissions_value_start + 1;
    let permissions_content_end = permissions_end;

    let deny_value_start = find_object_member_value_start(
        original,
        permissions_content_start,
        permissions_content_end,
        "deny",
    )?;
    let deny_value_start = skip_ws_and_comments(original, deny_value_start);
    if original.as_bytes().get(deny_value_start)? != &b'[' {
        return None;
    }
    let array_start = deny_value_start;
    let array_end = find_matching_bracket(original, array_start, b'[', b']')?;

    let indent = line_indent(original, array_start);
    let new_array = render_json_string_array(deny, &indent);

    let mut out = String::new();
    out.push_str(&original[..array_start]);
    out.push_str(&new_array);
    out.push_str(&original[array_end + 1..]);
    Some(out)
}

fn render_json_string_array(items: &[String], line_indent: &str) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }

    let item_indent = format!("{line_indent}  ");
    let mut out = String::new();
    out.push('[');
    out.push('\n');
    for (idx, item) in items.iter().enumerate() {
        out.push_str(&item_indent);
        out.push_str(&serde_json::to_string(item).unwrap_or_else(|_| "\"\"".to_string()));
        if idx + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(line_indent);
    out.push(']');
    out
}

fn line_indent(text: &str, idx: usize) -> String {
    let line_start = text[..idx].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let slice = &text[line_start..idx];
    slice.chars().take_while(|ch| ch.is_whitespace()).collect()
}

fn skip_ws_and_comments(text: &str, mut idx: usize) -> usize {
    let bytes = text.as_bytes();
    loop {
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            idx += 2;
            while idx < bytes.len() && bytes[idx] != b'\n' {
                idx += 1;
            }
            continue;
        }

        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            idx += 2;
            while idx + 1 < bytes.len() {
                if bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            continue;
        }

        break;
    }
    idx
}

fn parse_json_string(text: &str, start_quote: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(start_quote)? != &b'"' {
        return None;
    }

    let mut out = String::new();
    let mut idx = start_quote + 1;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' => {
                idx += 1;
                let esc = *bytes.get(idx)?;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = text.get(idx + 1..idx + 5)?;
                        let code = u16::from_str_radix(hex, 16).ok()?;
                        out.push(char::from_u32(code as u32)?);
                        idx += 4;
                    }
                    _ => return None,
                }
                idx += 1;
            }
            b'"' => return Some((out, idx + 1)),
            other => {
                out.push(other as char);
                idx += 1;
            }
        }
    }

    None
}

fn find_object_member_value_start(
    text: &str,
    start: usize,
    end: usize,
    key: &str,
) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut idx = start;
    let mut depth_obj = 0usize;
    let mut depth_arr = 0usize;

    while idx < end {
        idx = skip_ws_and_comments(text, idx);
        if idx >= end {
            break;
        }

        match bytes[idx] {
            b'"' => {
                let (s, next) = parse_json_string(text, idx)?;
                let after = skip_ws_and_comments(text, next);
                if depth_obj == 0 && depth_arr == 0 && s == key && bytes.get(after) == Some(&b':') {
                    return Some(skip_ws_and_comments(text, after + 1));
                }
                idx = next;
            }
            b'{' => {
                depth_obj += 1;
                idx += 1;
            }
            b'}' => {
                depth_obj = depth_obj.saturating_sub(1);
                idx += 1;
            }
            b'[' => {
                depth_arr += 1;
                idx += 1;
            }
            b']' => {
                depth_arr = depth_arr.saturating_sub(1);
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    None
}

fn find_matching_bracket(text: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(start)? != &open {
        return None;
    }

    let mut idx = start;
    let mut depth = 0usize;
    while idx < bytes.len() {
        idx = skip_ws_and_comments(text, idx);
        if idx >= bytes.len() {
            break;
        }

        match bytes[idx] {
            b'"' => {
                let (_, next) = parse_json_string(text, idx)?;
                idx = next;
            }
            ch if ch == open => {
                depth += 1;
                idx += 1;
            }
            ch if ch == close => {
                depth = depth.saturating_sub(1);
                idx += 1;
                if depth == 0 {
                    return Some(idx - 1);
                }
            }
            _ => idx += 1,
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gitignore_like_supports_include_and_comments() {
        let rules = parse_gitignore_like(
            r#"
# comment

secrets/**
!dist/
        "#,
        );
        assert!(rules.ignore.contains("secrets/**"));
        assert!(rules.include.contains("dist/"));
        assert_eq!(rules.ignore.len(), 1);
        assert_eq!(rules.include.len(), 1);
    }

    #[test]
    fn test_render_gitignore_like_orders_ignore_then_include() {
        let mut rules = IgnoreRules::default();
        rules.ignore.insert("b".to_string());
        rules.ignore.insert("a".to_string());
        rules.include.insert("c".to_string());
        let text = render_gitignore_like(&rules);
        assert_eq!(text, "a\nb\n!c\n");
    }

    #[test]
    fn test_extract_read_pattern_normalizes_prefixes() {
        assert_eq!(
            extract_read_pattern("Read(./.env)").as_deref(),
            Some(".env")
        );
        assert_eq!(extract_read_pattern("Read(.env)").as_deref(), Some(".env"));
        assert_eq!(
            extract_read_pattern("Read(/secrets/**)").as_deref(),
            Some("secrets/**")
        );
    }

    #[test]
    fn test_merge_deny_rules_preserves_existing_and_appends_new() {
        let existing = vec![
            "WebFetch(domain:example.com)".to_string(),
            "Read(./a)".to_string(),
        ];
        let desired = vec!["Read(./a)".to_string(), "Read(./b)".to_string()];
        let (merged, added) = merge_deny_rules(&existing, &desired);
        assert_eq!(merged[0], "WebFetch(domain:example.com)");
        assert_eq!(merged[1], "Read(./a)");
        assert_eq!(merged[2], "Read(./b)");
        assert_eq!(added, vec!["Read(./b)".to_string()]);
    }

    #[test]
    fn test_patch_permissions_deny_array_replaces_array_only() {
        let original = r#"
{
  // keep comment
  "permissions": {
    "deny": [
      "Read(./a)"
    ]
  }
}
"#;
        let deny = vec!["Read(./a)".to_string(), "Read(./b)".to_string()];
        let patched = patch_permissions_deny_array(original, &deny).expect("patched");
        assert!(patched.contains("// keep comment"));
        assert!(patched.contains("\"Read(./b)\""));
        assert!(patched.contains("\"deny\": ["));
    }

    #[test]
    fn test_parse_claude_settings_supports_jsonc_and_extracts_read_rules() {
        let content = r#"
{
  // comment
  "permissions": {
    "deny": [
      "Read(./secrets/**)",
      "WebFetch(domain:example.com)"
    ]
  }
}
"#;
        let (rules, _) = parse_claude_settings(content, true).expect("parse settings");
        assert!(rules.ignore.contains("secrets/**"));
        assert!(!rules.ignore.contains("./secrets/**"));
    }

    #[test]
    fn test_union_inplace_dedupes_and_renders_deterministically() {
        let mut left = IgnoreRules::default();
        left.ignore.insert("b".to_string());
        left.include.insert("d".to_string());

        let mut right = IgnoreRules::default();
        right.ignore.insert("a".to_string());
        right.ignore.insert("b".to_string());
        right.include.insert("c".to_string());

        left.union_inplace(&right);
        assert_eq!(left.ignore.len(), 2);
        assert_eq!(left.include.len(), 2);

        let text = render_gitignore_like(&left);
        assert_eq!(text, "a\nb\n!c\n!d\n");
    }
}
