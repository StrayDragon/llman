use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use inquire::{Confirm, Text};
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Component, Path, PathBuf};

const OPENSPEC_DIR_NAME: &str = "openspec";
const OPENSPEC_CONFIG_FILE: &str = "config.yaml";
const OPENSPEC_SCHEMA: &str = "spec-driven";

#[derive(Debug, Clone)]
pub struct InteropArgs {
    pub style: String,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Import,
    Export,
}

impl Direction {
    fn source_dir(self) -> &'static str {
        match self {
            Self::Import => OPENSPEC_DIR_NAME,
            Self::Export => LLMANSPEC_DIR_NAME,
        }
    }

    fn target_dir(self) -> &'static str {
        match self {
            Self::Import => LLMANSPEC_DIR_NAME,
            Self::Export => OPENSPEC_DIR_NAME,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum FileOpKind {
    Copy,
    CopyWithFrontmatterFill,
    WriteOpenSpecConfig,
    WriteOpenSpecChangeMetadata,
}

#[derive(Debug, Clone)]
struct PlannedFile {
    source: Option<PathBuf>,
    target: PathBuf,
    relative: PathBuf,
    kind: FileOpKind,
}

#[derive(Debug, Clone)]
struct MigrationPlan {
    source_root: PathBuf,
    target_root: PathBuf,
    files: Vec<PlannedFile>,
    warnings: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct MigrationSummary {
    copied_files: usize,
    generated_files: usize,
    warnings: Vec<PathBuf>,
    deleted_source: bool,
}

trait PromptAdapter {
    fn confirm_execute(&self, prompt: &str) -> Result<bool>;
    fn input_phrase(&self, prompt: &str) -> Result<String>;
    fn confirm_delete_source(&self, prompt: &str, default: bool) -> Result<bool>;
}

struct InquirePromptAdapter;

impl PromptAdapter for InquirePromptAdapter {
    fn confirm_execute(&self, prompt: &str) -> Result<bool> {
        Confirm::new(prompt)
            .with_default(false)
            .prompt()
            .map_err(|err| anyhow!(t!("errors.inquire_error", error = err)))
    }

    fn input_phrase(&self, prompt: &str) -> Result<String> {
        Text::new(prompt)
            .prompt()
            .map_err(|err| anyhow!(t!("errors.inquire_error", error = err)))
    }

    fn confirm_delete_source(&self, prompt: &str, default: bool) -> Result<bool> {
        Confirm::new(prompt)
            .with_default(default)
            .prompt()
            .map_err(|err| anyhow!(t!("errors.inquire_error", error = err)))
    }
}

pub fn run_import(args: InteropArgs) -> Result<()> {
    run_with_prompt_adapter(
        args,
        Direction::Import,
        &InquirePromptAdapter,
        is_interactive_terminal,
    )
    .map(|_| ())
}

pub fn run_export(args: InteropArgs) -> Result<()> {
    run_with_prompt_adapter(
        args,
        Direction::Export,
        &InquirePromptAdapter,
        is_interactive_terminal,
    )
    .map(|_| ())
}

fn run_with_prompt_adapter<I>(
    args: InteropArgs,
    direction: Direction,
    prompt: &dyn PromptAdapter,
    is_interactive: I,
) -> Result<MigrationSummary>
where
    I: Fn() -> bool,
{
    validate_style(direction, &args.style)?;

    let root = args.path.unwrap_or_else(|| PathBuf::from("."));
    let created_date = Utc::now().format("%Y-%m-%d").to_string();
    let plan = build_plan(&root, direction)?;

    print_plan(direction, &plan);

    if !is_interactive() {
        return Err(anyhow!(msg_non_interactive(direction)));
    }

    if !prompt.confirm_execute(&msg_confirm_execute(direction))? {
        return Err(anyhow!(msg_cancelled(direction)));
    }

    let phrase = msg_confirm_phrase(direction);
    let input = prompt.input_phrase(&msg_confirm_phrase_prompt(direction, &phrase))?;
    if input.trim() != phrase {
        return Err(anyhow!(msg_phrase_mismatch(direction)));
    }

    let mut summary = apply_plan(&plan, &created_date)?;
    summary.warnings = plan.warnings.clone();

    println!(
        "{}",
        msg_apply_done(
            direction,
            summary.copied_files,
            summary.generated_files,
            plan.source_root.display(),
            plan.target_root.display(),
        )
    );

    let delete_prompt = msg_delete_source_prompt(direction, plan.source_root.display());
    if prompt.confirm_delete_source(&delete_prompt, false)? {
        fs::remove_dir_all(&plan.source_root).with_context(|| {
            format!(
                "remove migrated source directory {}",
                plan.source_root.display()
            )
        })?;
        summary.deleted_source = true;
        println!(
            "{}",
            msg_source_deleted(direction, plan.source_root.display())
        );
    } else {
        println!("{}", msg_source_kept(direction, plan.source_root.display()));
    }

    Ok(summary)
}

fn validate_style(direction: Direction, style: &str) -> Result<()> {
    if style.trim() == "openspec" {
        return Ok(());
    }
    Err(anyhow!(msg_style_only(direction, style)))
}

fn build_plan(root: &Path, direction: Direction) -> Result<MigrationPlan> {
    let source_root = root.join(direction.source_dir());
    let target_root = root.join(direction.target_dir());
    if !source_root.exists() {
        return Err(anyhow!(msg_source_missing(
            direction,
            source_root.display()
        )));
    }
    if !source_root.is_dir() {
        return Err(anyhow!(msg_source_not_dir(
            direction,
            source_root.display()
        )));
    }

    let mut files = Vec::new();
    if source_root.join("specs").exists() {
        collect_dir_files(
            direction,
            &source_root,
            &target_root,
            &source_root.join("specs"),
            &mut files,
        )?;
    }
    if source_root.join("changes").exists() {
        collect_dir_files(
            direction,
            &source_root,
            &target_root,
            &source_root.join("changes"),
            &mut files,
        )?;
    }

    let mut warnings = Vec::new();
    for entry in read_dir_sorted(&source_root)? {
        let name = entry.file_name();
        if name == "specs" || name == "changes" || name == OPENSPEC_CONFIG_FILE {
            continue;
        }

        let source_path = entry.path();
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            return Err(anyhow!(msg_symlink_unsupported(
                direction,
                source_path.display()
            )));
        }

        let rel = source_path
            .strip_prefix(&source_root)
            .map_err(|_| anyhow!("failed to derive relative path"))?
            .to_path_buf();
        ensure_safe_relative_path(&rel)?;
        warnings.push(rel.clone());

        if metadata.is_dir() {
            collect_dir_files(
                direction,
                &source_root,
                &target_root,
                &source_path,
                &mut files,
            )?;
        } else if metadata.is_file() {
            let target = target_root.join(&rel);
            ensure_target_within_root(&target_root, &target)?;
            files.push(PlannedFile {
                source: Some(source_path),
                target,
                relative: rel,
                kind: FileOpKind::Copy,
            });
        }
    }

    if direction == Direction::Export {
        add_export_metadata_ops(&source_root, &target_root, &mut files)?;
    }

    sort_planned_files(&mut files);
    validate_planned_targets(direction, &files)?;

    Ok(MigrationPlan {
        source_root,
        target_root,
        files,
        warnings,
    })
}

fn collect_dir_files(
    direction: Direction,
    source_root: &Path,
    target_root: &Path,
    dir: &Path,
    out: &mut Vec<PlannedFile>,
) -> Result<()> {
    for entry in read_dir_sorted(dir)? {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(anyhow!(msg_symlink_unsupported(direction, path.display())));
        }

        if file_type.is_dir() {
            collect_dir_files(direction, source_root, target_root, &path, out)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let relative = path
            .strip_prefix(source_root)
            .map_err(|_| anyhow!("failed to derive relative path"))?
            .to_path_buf();
        ensure_safe_relative_path(&relative)?;

        let kind = if direction == Direction::Import && is_main_spec_path(&relative) {
            FileOpKind::CopyWithFrontmatterFill
        } else {
            FileOpKind::Copy
        };

        let target = target_root.join(&relative);
        ensure_target_within_root(target_root, &target)?;

        out.push(PlannedFile {
            source: Some(path),
            target,
            relative,
            kind,
        });
    }

    Ok(())
}

fn add_export_metadata_ops(
    source_root: &Path,
    target_root: &Path,
    out: &mut Vec<PlannedFile>,
) -> Result<()> {
    let config_target = target_root.join(OPENSPEC_CONFIG_FILE);
    if !config_target.exists() {
        out.push(PlannedFile {
            source: None,
            target: config_target,
            relative: PathBuf::from(OPENSPEC_CONFIG_FILE),
            kind: FileOpKind::WriteOpenSpecConfig,
        });
    }

    let changes_dir = source_root.join("changes");
    if !changes_dir.exists() {
        return Ok(());
    }

    for entry in read_dir_sorted(&changes_dir)? {
        let metadata = entry.metadata()?;
        if !metadata.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == "archive" || name.starts_with('.') {
            continue;
        }

        let rel = PathBuf::from("changes").join(name).join(".openspec.yaml");
        ensure_safe_relative_path(&rel)?;
        let target = target_root.join(&rel);
        if target.exists() {
            continue;
        }

        out.push(PlannedFile {
            source: None,
            target,
            relative: rel,
            kind: FileOpKind::WriteOpenSpecChangeMetadata,
        });
    }

    Ok(())
}

fn sort_planned_files(files: &mut [PlannedFile]) {
    files.sort_by(|a, b| {
        a.relative
            .to_string_lossy()
            .cmp(&b.relative.to_string_lossy())
            .then(a.kind.cmp(&b.kind))
    });
}

fn validate_planned_targets(direction: Direction, files: &[PlannedFile]) -> Result<()> {
    let mut conflicts = Vec::new();
    let mut seen = HashSet::new();

    for file in files {
        if !seen.insert(file.target.clone()) {
            return Err(anyhow!(msg_duplicate_target(
                direction,
                file.target.display()
            )));
        }
        if file.target.exists() {
            conflicts.push(file.relative.display().to_string());
        }
    }

    if conflicts.is_empty() {
        return Ok(());
    }

    conflicts.sort();
    Err(anyhow!(msg_conflicts(direction, &conflicts)))
}

fn apply_plan(plan: &MigrationPlan, created_date: &str) -> Result<MigrationSummary> {
    let mut summary = MigrationSummary::default();

    for file in &plan.files {
        if let Some(parent) = file.target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create directory {}", parent.display()))?;
        }

        match file.kind {
            FileOpKind::Copy => {
                let source = file.source.as_ref().expect("copy source path");
                fs::copy(source, &file.target).with_context(|| {
                    format!(
                        "copy file {} -> {}",
                        source.display(),
                        file.target.display()
                    )
                })?;
                summary.copied_files += 1;
            }
            FileOpKind::CopyWithFrontmatterFill => {
                let source = file.source.as_ref().expect("spec source path");
                let content = fs::read_to_string(source)
                    .with_context(|| format!("read source spec {}", source.display()))?;
                let updated = ensure_llman_frontmatter(&content)?;
                fs::write(&file.target, updated)
                    .with_context(|| format!("write transformed spec {}", file.target.display()))?;
                summary.copied_files += 1;
            }
            FileOpKind::WriteOpenSpecConfig => {
                fs::write(&file.target, format!("schema: {}\n", OPENSPEC_SCHEMA))
                    .with_context(|| format!("write {}", file.target.display()))?;
                summary.generated_files += 1;
            }
            FileOpKind::WriteOpenSpecChangeMetadata => {
                let content = format!("schema: {}\ncreated: {}\n", OPENSPEC_SCHEMA, created_date);
                fs::write(&file.target, content)
                    .with_context(|| format!("write {}", file.target.display()))?;
                summary.generated_files += 1;
            }
        }
    }

    Ok(summary)
}

fn ensure_llman_frontmatter(content: &str) -> Result<String> {
    let normalized = normalize_newlines(content);

    let Some((frontmatter_yaml, body)) = split_frontmatter(&normalized) else {
        return Ok(render_default_frontmatter(&normalized));
    };

    let parsed: serde_yaml::Value = match serde_yaml::from_str(&frontmatter_yaml) {
        Ok(value) => value,
        Err(_) => return Ok(render_default_frontmatter(&normalized)),
    };

    let mut mapping = match parsed {
        serde_yaml::Value::Mapping(mapping) => mapping,
        _ => return Ok(render_default_frontmatter(&normalized)),
    };

    let mut changed = false;
    changed |= ensure_frontmatter_key(
        &mut mapping,
        "llman_spec_valid_scope",
        default_scope_value(),
    );
    changed |= ensure_frontmatter_key(
        &mut mapping,
        "llman_spec_valid_commands",
        default_commands_value(),
    );
    changed |= ensure_frontmatter_key(
        &mut mapping,
        "llman_spec_evidence",
        default_evidence_value(),
    );

    if !changed {
        return Ok(normalized);
    }

    let yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(mapping))?;
    let yaml = yaml.trim().to_string();
    Ok(format_frontmatter_with_body(&yaml, &body))
}

fn ensure_frontmatter_key(
    map: &mut serde_yaml::Mapping,
    key: &str,
    default_value: serde_yaml::Value,
) -> bool {
    let yaml_key = serde_yaml::Value::String(key.to_string());
    let should_set = match map.get(&yaml_key) {
        None => true,
        Some(value) => !is_non_empty_string_or_sequence(value),
    };

    if should_set {
        map.insert(yaml_key, default_value);
        return true;
    }

    false
}

fn default_scope_value() -> serde_yaml::Value {
    serde_yaml::Value::Sequence(vec![
        serde_yaml::Value::String("src".to_string()),
        serde_yaml::Value::String("tests".to_string()),
    ])
}

fn default_commands_value() -> serde_yaml::Value {
    serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("just test".to_string())])
}

fn default_evidence_value() -> serde_yaml::Value {
    serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
        "Imported from OpenSpec".to_string(),
    )])
}

fn is_non_empty_string_or_sequence(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::String(text) => !text.trim().is_empty(),
        serde_yaml::Value::Sequence(items) => items
            .iter()
            .any(|item| matches!(item, serde_yaml::Value::String(text) if !text.trim().is_empty())),
        _ => false,
    }
}

fn render_default_frontmatter(content: &str) -> String {
    let yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
        [
            (
                serde_yaml::Value::String("llman_spec_valid_scope".to_string()),
                default_scope_value(),
            ),
            (
                serde_yaml::Value::String("llman_spec_valid_commands".to_string()),
                default_commands_value(),
            ),
            (
                serde_yaml::Value::String("llman_spec_evidence".to_string()),
                default_evidence_value(),
            ),
        ],
    )))
    .expect("serialize default frontmatter");

    format_frontmatter_with_body(yaml.trim(), content.trim_start_matches('\n'))
}

fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let mut lines = content.lines();
    if lines.next()? != "---" {
        return None;
    }

    let mut yaml_lines = Vec::new();
    let mut found_end = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_end = true;
            break;
        }
        yaml_lines.push(line);
    }

    if !found_end {
        return None;
    }

    let body = lines.collect::<Vec<_>>().join("\n");
    Some((yaml_lines.join("\n"), body))
}

fn format_frontmatter_with_body(yaml: &str, body: &str) -> String {
    if body.trim().is_empty() {
        return format!("---\n{}\n---\n", yaml.trim());
    }
    format!(
        "---\n{}\n---\n\n{}",
        yaml.trim(),
        body.trim_start_matches('\n')
    )
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

fn print_plan(direction: Direction, plan: &MigrationPlan) {
    println!(
        "{}",
        msg_dry_run_header(
            direction,
            plan.source_root.display(),
            plan.target_root.display(),
        )
    );

    if plan.files.is_empty() {
        println!("{}", msg_dry_run_empty(direction));
    } else {
        for file in &plan.files {
            println!("{}", format_plan_item(direction, file));
        }
    }

    println!("{}", msg_dry_run_total(direction, plan.files.len()));

    for warning in &plan.warnings {
        eprintln!("{}", msg_warning_non_standard(direction, warning.display()));
    }
}

fn format_plan_item(direction: Direction, file: &PlannedFile) -> String {
    match file.kind {
        FileOpKind::Copy => match direction {
            Direction::Import => t!(
                "sdd.import.plan_item_copy",
                source = file.relative.display(),
                target = file.relative.display()
            )
            .to_string(),
            Direction::Export => t!(
                "sdd.export.plan_item_copy",
                source = file.relative.display(),
                target = file.relative.display()
            )
            .to_string(),
        },
        FileOpKind::CopyWithFrontmatterFill => match direction {
            Direction::Import => t!(
                "sdd.import.plan_item_copy_frontmatter",
                source = file.relative.display(),
                target = file.relative.display()
            )
            .to_string(),
            Direction::Export => t!(
                "sdd.export.plan_item_copy_frontmatter",
                source = file.relative.display(),
                target = file.relative.display()
            )
            .to_string(),
        },
        FileOpKind::WriteOpenSpecConfig | FileOpKind::WriteOpenSpecChangeMetadata => {
            match direction {
                Direction::Import => t!(
                    "sdd.import.plan_item_generate",
                    target = file.relative.display()
                )
                .to_string(),
                Direction::Export => t!(
                    "sdd.export.plan_item_generate",
                    target = file.relative.display()
                )
                .to_string(),
            }
        }
    }
}

fn read_dir_sorted(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        entries.push(entry?);
    }
    entries.sort_by_key(|a| a.file_name());
    Ok(entries)
}

fn is_main_spec_path(relative: &Path) -> bool {
    let mut components = relative.components();
    matches!(components.next(), Some(Component::Normal(first)) if first == "specs")
        && relative.file_name().is_some_and(|name| name == "spec.md")
}

fn ensure_safe_relative_path(path: &Path) -> Result<()> {
    if path.is_absolute() {
        return Err(anyhow!("absolute path is not allowed: {}", path.display()));
    }

    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("unsafe path component in {}", path.display()));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    Ok(())
}

fn ensure_target_within_root(root: &Path, target: &Path) -> Result<()> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| anyhow!("target path escapes root: {}", target.display()))?;
    ensure_safe_relative_path(relative)
}

fn is_interactive_terminal() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn msg_style_only(direction: Direction, style: &str) -> String {
    match direction {
        Direction::Import => t!("sdd.import.style_only", style = style).to_string(),
        Direction::Export => t!("sdd.export.style_only", style = style).to_string(),
    }
}

fn msg_source_missing(direction: Direction, source: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.source_missing", source = source).to_string(),
        Direction::Export => t!("sdd.export.source_missing", source = source).to_string(),
    }
}

fn msg_source_not_dir(direction: Direction, source: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.source_not_dir", source = source).to_string(),
        Direction::Export => t!("sdd.export.source_not_dir", source = source).to_string(),
    }
}

fn msg_symlink_unsupported(direction: Direction, path: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.symlink_unsupported", path = path).to_string(),
        Direction::Export => t!("sdd.export.symlink_unsupported", path = path).to_string(),
    }
}

fn msg_duplicate_target(direction: Direction, target: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.duplicate_target", target = target).to_string(),
        Direction::Export => t!("sdd.export.duplicate_target", target = target).to_string(),
    }
}

fn msg_conflicts(direction: Direction, conflicts: &[String]) -> String {
    let list = conflicts.join("\n- ");
    match direction {
        Direction::Import => t!("sdd.import.conflicts", conflicts = list).to_string(),
        Direction::Export => t!("sdd.export.conflicts", conflicts = list).to_string(),
    }
}

fn msg_dry_run_header(
    direction: Direction,
    source: impl std::fmt::Display,
    target: impl std::fmt::Display,
) -> String {
    match direction {
        Direction::Import => t!(
            "sdd.import.dry_run_header",
            source = source,
            target = target
        )
        .to_string(),
        Direction::Export => t!(
            "sdd.export.dry_run_header",
            source = source,
            target = target
        )
        .to_string(),
    }
}

fn msg_dry_run_empty(direction: Direction) -> String {
    match direction {
        Direction::Import => t!("sdd.import.dry_run_empty").to_string(),
        Direction::Export => t!("sdd.export.dry_run_empty").to_string(),
    }
}

fn msg_dry_run_total(direction: Direction, count: usize) -> String {
    match direction {
        Direction::Import => t!("sdd.import.dry_run_total", count = count).to_string(),
        Direction::Export => t!("sdd.export.dry_run_total", count = count).to_string(),
    }
}

fn msg_warning_non_standard(direction: Direction, path: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.warning_non_standard", path = path).to_string(),
        Direction::Export => t!("sdd.export.warning_non_standard", path = path).to_string(),
    }
}

fn msg_non_interactive(direction: Direction) -> String {
    match direction {
        Direction::Import => t!("sdd.import.non_interactive").to_string(),
        Direction::Export => t!("sdd.export.non_interactive").to_string(),
    }
}

fn msg_confirm_execute(direction: Direction) -> String {
    match direction {
        Direction::Import => t!("sdd.import.confirm_execute").to_string(),
        Direction::Export => t!("sdd.export.confirm_execute").to_string(),
    }
}

fn msg_confirm_phrase(direction: Direction) -> String {
    match direction {
        Direction::Import => t!("sdd.import.confirm_phrase").to_string(),
        Direction::Export => t!("sdd.export.confirm_phrase").to_string(),
    }
}

fn msg_confirm_phrase_prompt(direction: Direction, phrase: &str) -> String {
    match direction {
        Direction::Import => t!("sdd.import.confirm_phrase_prompt", phrase = phrase).to_string(),
        Direction::Export => t!("sdd.export.confirm_phrase_prompt", phrase = phrase).to_string(),
    }
}

fn msg_phrase_mismatch(direction: Direction) -> String {
    match direction {
        Direction::Import => t!("sdd.import.phrase_mismatch").to_string(),
        Direction::Export => t!("sdd.export.phrase_mismatch").to_string(),
    }
}

fn msg_cancelled(direction: Direction) -> String {
    match direction {
        Direction::Import => t!("sdd.import.cancelled").to_string(),
        Direction::Export => t!("sdd.export.cancelled").to_string(),
    }
}

fn msg_apply_done(
    direction: Direction,
    copied: usize,
    generated: usize,
    source: impl std::fmt::Display,
    target: impl std::fmt::Display,
) -> String {
    match direction {
        Direction::Import => t!(
            "sdd.import.apply_done",
            copied = copied,
            generated = generated,
            source = source,
            target = target
        )
        .to_string(),
        Direction::Export => t!(
            "sdd.export.apply_done",
            copied = copied,
            generated = generated,
            source = source,
            target = target
        )
        .to_string(),
    }
}

fn msg_delete_source_prompt(direction: Direction, source: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.delete_source_prompt", source = source).to_string(),
        Direction::Export => t!("sdd.export.delete_source_prompt", source = source).to_string(),
    }
}

fn msg_source_deleted(direction: Direction, source: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.source_deleted", source = source).to_string(),
        Direction::Export => t!("sdd.export.source_deleted", source = source).to_string(),
    }
}

fn msg_source_kept(direction: Direction, source: impl std::fmt::Display) -> String {
    match direction {
        Direction::Import => t!("sdd.import.source_kept", source = source).to_string(),
        Direction::Export => t!("sdd.export.source_kept", source = source).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_locale;
    use std::sync::Mutex;
    use tempfile::tempdir;

    struct TestPromptAdapter {
        confirm_execute: bool,
        phrase_input: String,
        confirm_delete: bool,
        delete_defaults: Mutex<Vec<bool>>,
    }

    impl TestPromptAdapter {
        fn approve_keep_source() -> Self {
            Self {
                confirm_execute: true,
                phrase_input: "MIGRATE".to_string(),
                confirm_delete: false,
                delete_defaults: Mutex::new(Vec::new()),
            }
        }

        fn approve_delete_source() -> Self {
            Self {
                confirm_execute: true,
                phrase_input: "MIGRATE".to_string(),
                confirm_delete: true,
                delete_defaults: Mutex::new(Vec::new()),
            }
        }
    }

    impl PromptAdapter for TestPromptAdapter {
        fn confirm_execute(&self, _prompt: &str) -> Result<bool> {
            Ok(self.confirm_execute)
        }

        fn input_phrase(&self, _prompt: &str) -> Result<String> {
            Ok(self.phrase_input.clone())
        }

        fn confirm_delete_source(&self, _prompt: &str, default: bool) -> Result<bool> {
            self.delete_defaults
                .lock()
                .expect("lock delete defaults")
                .push(default);
            Ok(self.confirm_delete)
        }
    }

    fn always_interactive() -> bool {
        true
    }

    fn create_open_spec_source(root: &Path) {
        fs::create_dir_all(root.join("openspec/specs/sample")).expect("create specs dir");
        fs::create_dir_all(root.join("openspec/changes/add-sample/specs/sample"))
            .expect("create active change spec dir");
        fs::create_dir_all(root.join("openspec/changes/archive/2026-02-25-add-old/specs/sample"))
            .expect("create archive change spec dir");

        fs::write(
            root.join("openspec/specs/sample/spec.md"),
            "# Sample\n\n## Purpose\ntext\n\n## Requirements\n",
        )
        .expect("write main spec");
        fs::write(
            root.join("openspec/changes/add-sample/proposal.md"),
            "## Why\nneed\n",
        )
        .expect("write proposal");
        fs::write(
            root.join("openspec/changes/archive/2026-02-25-add-old/tasks.md"),
            "- [x] done\n",
        )
        .expect("write archived tasks");
    }

    #[test]
    fn import_copies_standard_scope_and_fills_frontmatter() {
        init_locale();
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        create_open_spec_source(root);

        let prompt = TestPromptAdapter::approve_keep_source();
        let result = run_with_prompt_adapter(
            InteropArgs {
                style: "openspec".to_string(),
                path: Some(root.to_path_buf()),
            },
            Direction::Import,
            &prompt,
            always_interactive,
        )
        .expect("import succeeds");

        let spec = fs::read_to_string(root.join("llmanspec/specs/sample/spec.md"))
            .expect("read target spec");
        assert!(spec.contains("llman_spec_valid_scope"));
        assert!(spec.contains("llman_spec_valid_commands"));
        assert!(spec.contains("llman_spec_evidence"));

        assert!(
            root.join("llmanspec/changes/add-sample/proposal.md")
                .exists()
        );
        assert!(
            root.join("llmanspec/changes/archive/2026-02-25-add-old/tasks.md")
                .exists()
        );

        assert!(root.join("openspec").exists());
        assert!(!result.deleted_source);
        let defaults = prompt.delete_defaults.lock().expect("lock defaults");
        assert_eq!(defaults.as_slice(), &[false]);
    }

    #[test]
    fn import_copies_non_standard_entries_with_warning() {
        init_locale();
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        create_open_spec_source(root);
        fs::create_dir_all(root.join("openspec/explorations")).expect("create explorations");
        fs::write(root.join("openspec/explorations/notes.md"), "notes").expect("write note");

        let prompt = TestPromptAdapter::approve_keep_source();
        let result = run_with_prompt_adapter(
            InteropArgs {
                style: "openspec".to_string(),
                path: Some(root.to_path_buf()),
            },
            Direction::Import,
            &prompt,
            always_interactive,
        )
        .expect("import succeeds");

        assert!(root.join("llmanspec/explorations/notes.md").exists());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning == Path::new("explorations"))
        );
    }

    #[test]
    fn import_fails_on_target_conflicts_without_writing() {
        init_locale();
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        create_open_spec_source(root);

        fs::create_dir_all(root.join("llmanspec/specs/sample")).expect("create llmanspec dir");
        fs::write(
            root.join("llmanspec/specs/sample/spec.md"),
            "existing content",
        )
        .expect("write existing target");

        let prompt = TestPromptAdapter::approve_keep_source();
        let err = run_with_prompt_adapter(
            InteropArgs {
                style: "openspec".to_string(),
                path: Some(root.to_path_buf()),
            },
            Direction::Import,
            &prompt,
            always_interactive,
        )
        .expect_err("must fail on conflict");

        let err_text = err.to_string();
        assert!(err_text.contains("conflicts") || err_text.contains("Conflict"));

        let target = fs::read_to_string(root.join("llmanspec/specs/sample/spec.md"))
            .expect("read existing target");
        assert_eq!(target, "existing content");
    }

    #[test]
    fn export_creates_openspec_metadata_files() {
        init_locale();
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        fs::create_dir_all(root.join("llmanspec/specs/sample")).expect("create main spec dir");
        fs::create_dir_all(root.join("llmanspec/changes/add-sample")).expect("create change dir");
        fs::write(
            root.join("llmanspec/specs/sample/spec.md"),
            "---\nllman_spec_valid_scope:\n  - src\nllman_spec_valid_commands:\n  - just test\nllman_spec_evidence:\n  - local\n---\n\n# Sample\n",
        )
        .expect("write spec");
        fs::write(
            root.join("llmanspec/changes/add-sample/proposal.md"),
            "## Why\nneed\n",
        )
        .expect("write proposal");

        let prompt = TestPromptAdapter::approve_keep_source();
        run_with_prompt_adapter(
            InteropArgs {
                style: "openspec".to_string(),
                path: Some(root.to_path_buf()),
            },
            Direction::Export,
            &prompt,
            always_interactive,
        )
        .expect("export succeeds");

        let config =
            fs::read_to_string(root.join("openspec/config.yaml")).expect("read openspec config");
        assert!(config.contains("schema: spec-driven"));

        let meta = fs::read_to_string(root.join("openspec/changes/add-sample/.openspec.yaml"))
            .expect("read change metadata");
        assert!(meta.contains("schema: spec-driven"));
        assert!(meta.contains("created:"));
    }

    #[test]
    fn delete_source_defaults_to_no_and_can_be_confirmed_yes() {
        init_locale();
        let dir_keep = tempdir().expect("tempdir keep");
        create_open_spec_source(dir_keep.path());

        let keep_prompt = TestPromptAdapter::approve_keep_source();
        let keep_result = run_with_prompt_adapter(
            InteropArgs {
                style: "openspec".to_string(),
                path: Some(dir_keep.path().to_path_buf()),
            },
            Direction::Import,
            &keep_prompt,
            always_interactive,
        )
        .expect("import keeps source");
        assert!(!keep_result.deleted_source);
        assert!(dir_keep.path().join("openspec").exists());

        let dir_delete = tempdir().expect("tempdir delete");
        create_open_spec_source(dir_delete.path());

        let delete_prompt = TestPromptAdapter::approve_delete_source();
        let delete_result = run_with_prompt_adapter(
            InteropArgs {
                style: "openspec".to_string(),
                path: Some(dir_delete.path().to_path_buf()),
            },
            Direction::Import,
            &delete_prompt,
            always_interactive,
        )
        .expect("import deletes source");

        assert!(delete_result.deleted_source);
        assert!(!dir_delete.path().join("openspec").exists());
    }
}
