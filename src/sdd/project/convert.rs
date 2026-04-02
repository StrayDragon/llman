use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::{SpecStyle, load_required_config, write_config};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::backend::{DumpOptions, backend_for_style};
use crate::sdd::spec::fence::render_code_fence;
use crate::sdd::spec::ison::{compose_with_frontmatter, split_frontmatter};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConvertArgs {
    pub to: SpecStyle,
    pub project: bool,
    pub file: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub dry_run: bool,
}

pub fn run(root: &Path, args: ConvertArgs) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let mut config = load_required_config(&llmanspec_dir)?;

    if !args.project && args.file.is_none() {
        return Err(anyhow!("convert requires --project or --file <path>"));
    }
    if args.project && args.file.is_some() {
        return Err(anyhow!("convert cannot use both --project and --file"));
    }
    if args.output.is_some() && args.file.is_none() {
        return Err(anyhow!("--output requires --file"));
    }

    if args.to == config.spec_style {
        if args.dry_run {
            println!(
                "No-op (dry-run): project already uses spec_style: {}",
                config.spec_style.as_str()
            );
            return Ok(());
        }
        return Ok(());
    }

    if args.project {
        convert_project(root, &llmanspec_dir, &mut config, args.to, args.dry_run)?;
        return Ok(());
    }

    let file = args.file.expect("validated --file present");
    convert_file(
        root,
        &llmanspec_dir,
        config.spec_style,
        args.to,
        &file,
        args.output.as_ref(),
        args.dry_run,
    )
}

#[derive(Debug, Clone)]
enum ConvertFileKind {
    MainSpec {
        capability: String,
    },
    DeltaSpec {
        change_id: String,
        capability: String,
    },
}

fn classify_llmanspec_file(llmanspec_dir: &Path, path: &Path) -> Result<ConvertFileKind> {
    let relative = path.strip_prefix(llmanspec_dir).map_err(|_| {
        anyhow!(
            "unsupported file path: {} (must live under {})",
            path.display(),
            llmanspec_dir.display()
        )
    })?;
    let parts = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();

    if parts.len() == 3 && parts[0] == "specs" && parts[2] == "spec.md" {
        return Ok(ConvertFileKind::MainSpec {
            capability: parts[1].to_string(),
        });
    }
    if parts.len() == 5 && parts[0] == "changes" && parts[2] == "specs" && parts[4] == "spec.md" {
        return Ok(ConvertFileKind::DeltaSpec {
            change_id: parts[1].to_string(),
            capability: parts[3].to_string(),
        });
    }

    Err(anyhow!(
        "unsupported file path: {} (expected llmanspec/specs/<capability>/spec.md or llmanspec/changes/<change>/specs/<capability>/spec.md)",
        path.display()
    ))
}

fn convert_file(
    root: &Path,
    llmanspec_dir: &Path,
    from_style: SpecStyle,
    to_style: SpecStyle,
    file: &Path,
    output: Option<&PathBuf>,
    dry_run: bool,
) -> Result<()> {
    let file_path = resolve_path(root, file);
    if !file_path.exists() {
        return Err(anyhow!("convert file not found: {}", file_path.display()));
    }
    let kind = classify_llmanspec_file(llmanspec_dir, &file_path)?;

    let source_backend = backend_for_style(from_style);
    let target_backend = backend_for_style(to_style);

    let content = fs::read_to_string(&file_path).map_err(|err| anyhow!("read failed: {err}"))?;

    let converted = match kind {
        ConvertFileKind::MainSpec { capability } => {
            let (frontmatter_yaml, body) = split_frontmatter(&content);
            let Some(frontmatter_yaml) = frontmatter_yaml else {
                return Err(anyhow!(
                    "spec is missing YAML frontmatter: {}",
                    file_path.display()
                ));
            };
            let context = format!("spec `{}` during convert", capability);
            let doc = source_backend.parse_main_spec(&body, &context)?;
            let payload = target_backend.dump_main_spec(&doc, DumpOptions::default())?;
            let body = render_code_fence(to_style.as_str(), &payload);
            let rebuilt = compose_with_frontmatter(Some(&frontmatter_yaml), &body);

            // Verify round-trip semantics in the target style.
            let (_fm2, rebuilt_body) = split_frontmatter(&rebuilt);
            let reparsed = target_backend.parse_main_spec(&rebuilt_body, &context)?;
            if reparsed != doc {
                return Err(anyhow!(
                    "convert reparse mismatch for {}",
                    file_path.display()
                ));
            }
            rebuilt
        }
        ConvertFileKind::DeltaSpec {
            change_id,
            capability,
        } => {
            let context = format!(
                "delta spec `{}` for change `{}` during convert",
                capability, change_id
            );
            let doc = source_backend.parse_delta_spec(&content, &context)?;
            let payload = target_backend.dump_delta_spec(&doc, DumpOptions::default())?;
            let rebuilt = render_code_fence(to_style.as_str(), &payload);
            let reparsed = target_backend.parse_delta_spec(&rebuilt, &context)?;
            if reparsed != doc {
                return Err(anyhow!(
                    "convert reparse mismatch for {}",
                    file_path.display()
                ));
            }
            rebuilt
        }
    };

    if let Some(output) = output {
        let output_path = resolve_path(root, output);
        if dry_run {
            print!("{converted}");
            return Ok(());
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write_with_mode(&output_path, converted.as_bytes(), None)?;
        println!("{}", output_path.display());
        return Ok(());
    }

    // stdout mode (always safe; no on-disk writes).
    print!("{converted}");
    Ok(())
}

#[derive(Debug, Clone)]
struct PreparedWrite {
    path: PathBuf,
    content: String,
}

fn convert_project(
    root: &Path,
    llmanspec_dir: &Path,
    config: &mut crate::sdd::project::config::SddConfig,
    to_style: SpecStyle,
    dry_run: bool,
) -> Result<()> {
    let from_style = config.spec_style;
    let source_backend = backend_for_style(from_style);
    let target_backend = backend_for_style(to_style);

    let mut main_specs: Vec<(PathBuf, String, MainSpecPayload)> = Vec::new();
    let mut delta_specs: Vec<(PathBuf, DeltaSpecPayload)> = Vec::new();

    // Collect + pre-parse all main specs.
    let specs_dir = llmanspec_dir.join("specs");
    if specs_dir.exists() {
        for entry in fs::read_dir(&specs_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let capability = entry.file_name().to_string_lossy().to_string();
            let spec_path = entry.path().join("spec.md");
            if !spec_path.exists() {
                continue;
            }
            let content = fs::read_to_string(&spec_path)
                .map_err(|err| anyhow!("failed to read spec: {} ({})", spec_path.display(), err))?;
            let (frontmatter_yaml, body) = split_frontmatter(&content);
            let Some(frontmatter_yaml) = frontmatter_yaml else {
                return Err(anyhow!(
                    "spec is missing YAML frontmatter: {}",
                    spec_path.display()
                ));
            };
            let context = format!("spec `{}` during project convert", capability);
            let doc = source_backend.parse_main_spec(&body, &context)?;
            main_specs.push((
                spec_path,
                context,
                MainSpecPayload {
                    frontmatter_yaml,
                    doc,
                },
            ));
        }
    }

    // Collect + pre-parse all active change delta specs (skip `changes/archive`).
    let changes_dir = llmanspec_dir.join("changes");
    if changes_dir.exists() {
        for entry in fs::read_dir(&changes_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let change_id = entry.file_name().to_string_lossy().to_string();
            if change_id == "archive" {
                continue;
            }
            let specs_dir = entry.path().join("specs");
            if !specs_dir.exists() {
                continue;
            }
            for spec_entry in fs::read_dir(&specs_dir)? {
                let spec_entry = spec_entry?;
                if !spec_entry.file_type()?.is_dir() {
                    continue;
                }
                let capability = spec_entry.file_name().to_string_lossy().to_string();
                let delta_path = spec_entry.path().join("spec.md");
                if !delta_path.exists() {
                    continue;
                }
                let content = fs::read_to_string(&delta_path).map_err(|err| {
                    anyhow!(
                        "failed to read delta spec: {} ({})",
                        delta_path.display(),
                        err
                    )
                })?;
                let context = format!(
                    "delta spec `{}` for change `{}` during project convert",
                    capability, change_id
                );
                let doc = source_backend.parse_delta_spec(&content, &context)?;
                delta_specs.push((delta_path, DeltaSpecPayload { context, doc }));
            }
        }
    }

    let mut writes: Vec<PreparedWrite> = Vec::new();

    for (path, context, payload) in &main_specs {
        let dumped = target_backend.dump_main_spec(&payload.doc, DumpOptions::default())?;
        let body = render_code_fence(to_style.as_str(), &dumped);
        let rebuilt = compose_with_frontmatter(Some(&payload.frontmatter_yaml), &body);

        // Verify round-trip semantics.
        let (_fm2, rebuilt_body) = split_frontmatter(&rebuilt);
        let reparsed = target_backend.parse_main_spec(&rebuilt_body, context)?;
        if reparsed != payload.doc {
            return Err(anyhow!("convert reparse mismatch for {}", path.display()));
        }

        writes.push(PreparedWrite {
            path: path.clone(),
            content: rebuilt,
        });
    }

    for (path, payload) in &delta_specs {
        let dumped = target_backend.dump_delta_spec(&payload.doc, DumpOptions::default())?;
        let rebuilt = render_code_fence(to_style.as_str(), &dumped);
        let reparsed = target_backend.parse_delta_spec(&rebuilt, &payload.context)?;
        if reparsed != payload.doc {
            return Err(anyhow!("convert reparse mismatch for {}", path.display()));
        }

        writes.push(PreparedWrite {
            path: path.clone(),
            content: rebuilt,
        });
    }

    writes.sort_by(|a, b| a.path.cmp(&b.path));

    if dry_run {
        println!(
            "Dry-run: would convert project spec_style {} -> {}",
            from_style.as_str(),
            to_style.as_str()
        );
        for item in &writes {
            println!("{}", display_relative(root, &item.path));
        }
        println!("(dry-run) config not updated");
        return Ok(());
    }

    for item in &writes {
        if let Some(parent) = item.path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write_with_mode(&item.path, item.content.as_bytes(), None)?;
    }

    // Update config last (avoid leaving a repo in a mixed-style config state).
    config.spec_style = to_style;
    write_config(llmanspec_dir, config)?;

    println!(
        "Converted {} files and updated llmanspec/config.yaml spec_style to {}",
        writes.len(),
        to_style.as_str()
    );
    Ok(())
}

#[derive(Debug, Clone)]
struct MainSpecPayload {
    frontmatter_yaml: String,
    doc: crate::sdd::spec::ir::MainSpecDoc,
}

#[derive(Debug, Clone)]
struct DeltaSpecPayload {
    context: String,
    doc: crate::sdd::spec::ir::DeltaSpecDoc,
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn display_relative(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}
