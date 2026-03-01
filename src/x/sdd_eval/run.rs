use crate::x::sdd_eval::{acp, paths, playbook, presets, report};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifestV1 {
    pub version: u32,
    pub run_id: String,
    pub created_at: String,
    pub playbook_path: String,
    pub playbook_name: String,
    pub variants: Vec<VariantManifestV1>,
    pub max_iterations: u32,
    pub task_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantManifestV1 {
    pub name: String,
    pub style: String,
    pub agent_kind: String,
    pub agent_preset: String,
    pub agent_command: String,
    pub agent_args: Vec<String>,
    #[serde(default)]
    pub injected_env_keys: Vec<String>,
}

pub fn create_run(
    project_root: &Path,
    playbook_path: &Path,
    pb: &playbook::PlaybookV1,
) -> Result<PathBuf> {
    let playbook_abs = playbook::normalize_playbook_path(playbook_path)?;

    let run_id = generate_run_id(&pb.display_name("run"));
    let run_dir = paths::run_dir(project_root, &run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run dir {}", run_dir.display()))?;

    let used_playbook_path = run_dir.join("playbook.yaml");
    fs::copy(&playbook_abs, &used_playbook_path).with_context(|| {
        format!(
            "copy playbook {} -> {}",
            playbook_abs.display(),
            used_playbook_path.display()
        )
    })?;

    let variants_dir = run_dir.join("variants");
    fs::create_dir_all(&variants_dir)
        .with_context(|| format!("create {}", variants_dir.display()))?;

    let mut variants = Vec::with_capacity(pb.variants.len());
    for v in &pb.variants {
        let v_dir = variants_dir.join(&v.name);
        let workspace_root = v_dir.join("workspace");
        fs::create_dir_all(&workspace_root)
            .with_context(|| format!("create variant workspace {}", v.name))?;
        fs::create_dir_all(v_dir.join("logs"))
            .with_context(|| format!("create variant logs {}", v.name))?;
        fs::create_dir_all(v_dir.join("artifacts"))
            .with_context(|| format!("create variant artifacts {}", v.name))?;

        copy_project_root(project_root, &workspace_root)
            .with_context(|| format!("copy project into workspace for {}", v.name))?;
        init_workspace_sdd(&workspace_root, v.style)
            .with_context(|| format!("init SDD workspace for {}", v.name))?;

        let agent_command = v.agent.command_or_default()?;

        variants.push(VariantManifestV1 {
            name: v.name.clone(),
            style: match v.style {
                playbook::WorkflowStyle::Sdd => "sdd".to_string(),
                playbook::WorkflowStyle::SddLegacy => "sdd-legacy".to_string(),
            },
            agent_kind: match v.agent.kind {
                playbook::AgentKind::ClaudeCode => "claude-code-acp".to_string(),
                playbook::AgentKind::Codex => "codex-acp".to_string(),
                playbook::AgentKind::Fake => "fake-acp".to_string(),
            },
            agent_preset: v.agent.preset.clone(),
            agent_command,
            agent_args: v.agent.args.clone(),
            injected_env_keys: Vec::new(),
        });
    }

    let manifest = RunManifestV1 {
        version: 1,
        run_id: run_id.clone(),
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        playbook_path: playbook_abs.display().to_string(),
        playbook_name: playbook::playbook_file_stem(playbook_path),
        variants,
        max_iterations: pb.sdd_loop.max_iterations,
        task_title: pb.task.title.clone(),
    };

    let manifest_path = run_dir.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("write {}", manifest_path.display()))?;

    Ok(run_dir)
}

pub fn execute_run(run_dir: &Path, pb: &playbook::PlaybookV1) -> Result<()> {
    let manifest_path = run_dir.join("manifest.json");
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let mut manifest: RunManifestV1 =
        serde_json::from_str(&raw).with_context(|| "parse run manifest")?;

    for v in &pb.variants {
        let v_dir = run_dir.join("variants").join(&v.name);
        let workspace_root = v_dir.join("workspace");
        let logs_dir = v_dir.join("logs");
        let artifacts_dir = v_dir.join("artifacts");
        fs::create_dir_all(&logs_dir).with_context(|| format!("create {}", logs_dir.display()))?;
        fs::create_dir_all(&artifacts_dir)
            .with_context(|| format!("create {}", artifacts_dir.display()))?;

        let preset = presets::resolve_env(v.agent.kind, &v.agent.preset)
            .with_context(|| format!("resolve preset for variant {}", v.name))?;
        let mut injected_env_keys: Vec<String> = preset.env.keys().cloned().collect();
        injected_env_keys.sort();

        if let Some(mv) = manifest.variants.iter_mut().find(|mv| mv.name == v.name) {
            mv.injected_env_keys = injected_env_keys.clone();
        }

        let session_log_path = logs_dir.join("acp-session.jsonl");
        let agent_command = v.agent.command_or_default()?;

        let run_result = acp::run_variant(acp::VariantRunParams {
            workspace_root,
            style: v.style,
            agent_command,
            agent_args: v.agent.args.clone(),
            agent_env: preset.env,
            max_iterations: pb.sdd_loop.max_iterations,
            task_title: pb.task.title.clone(),
            task_prompt: pb.task.prompt.clone(),
            session_log_path,
        })
        .with_context(|| format!("run ACP variant {}", v.name))?;

        let metrics_path = artifacts_dir.join("acp-metrics.json");
        fs::write(
            &metrics_path,
            serde_json::to_vec_pretty(&run_result.metrics)?,
        )
        .with_context(|| format!("write {}", metrics_path.display()))?;
    }

    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("write {}", manifest_path.display()))?;

    Ok(())
}

pub fn generate_report(project_root: &Path, run_id: &str) -> Result<()> {
    report::generate(project_root, run_id)
}

pub fn import_human_scores(project_root: &Path, run_id: &str, file: &Path) -> Result<()> {
    report::import_human(project_root, run_id, file)
}

fn generate_run_id(prefix: &str) -> String {
    let now = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let safe_prefix = prefix
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    format!("{now}-{safe_prefix}")
}

fn init_workspace_sdd(workspace_root: &Path, style: playbook::WorkflowStyle) -> Result<()> {
    use crate::sdd::project::templates::TemplateStyle;

    let template_style = match style {
        playbook::WorkflowStyle::Sdd => TemplateStyle::New,
        playbook::WorkflowStyle::SddLegacy => TemplateStyle::Legacy,
    };

    let llmanspec = workspace_root.join(crate::sdd::shared::constants::LLMANSPEC_DIR_NAME);
    if !llmanspec.exists() {
        crate::sdd::project::init::run(workspace_root, None, template_style)
            .context("llman sdd init (workspace)")?;
    }

    crate::sdd::project::update::run(workspace_root, template_style)
        .context("llman sdd update (workspace)")?;

    Ok(())
}

fn copy_project_root(src_root: &Path, dst_root: &Path) -> Result<()> {
    if !dst_root.exists() {
        fs::create_dir_all(dst_root).with_context(|| format!("create {}", dst_root.display()))?;
    }

    let config_rel_skip = crate::config::resolve_config_dir(None)
        .ok()
        .and_then(|cfg| cfg.strip_prefix(src_root).ok().map(PathBuf::from));

    let walker = WalkBuilder::new(src_root)
        .hidden(false)
        .follow_links(false)
        .build();

    for entry in walker {
        let entry = entry?;
        let src_path = entry.path();
        if src_path == src_root {
            continue;
        }

        let rel = src_path
            .strip_prefix(src_root)
            .expect("walk paths are under src_root");
        if config_rel_skip
            .as_ref()
            .is_some_and(|skip| rel.starts_with(skip))
        {
            continue;
        }
        if should_skip_copy(rel) {
            continue;
        }

        let dst_path = dst_root.join(rel);
        let file_type = entry.file_type();
        if file_type.is_some_and(|ft| ft.is_symlink()) {
            // Avoid copying symlinks in v1 (simplifies sandbox + avoids surprising traversal).
            continue;
        }

        if file_type.is_some_and(|ft| ft.is_dir()) {
            fs::create_dir_all(&dst_path)
                .with_context(|| format!("create {}", dst_path.display()))?;
            continue;
        }

        if file_type.is_some_and(|ft| ft.is_file()) {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create {}", parent.display()))?;
            }
            fs::copy(src_path, &dst_path).with_context(|| {
                format!("copy {} -> {}", src_path.display(), dst_path.display())
            })?;
        }
    }

    Ok(())
}

fn should_skip_copy(rel: &Path) -> bool {
    for comp in rel.components() {
        let Component::Normal(name) = comp else {
            continue;
        };

        if name == ".git"
            || name == ".llman"
            || name == "target"
            || name == "node_modules"
            || name == ".venv"
            || name == "dist"
            || name == "build"
        {
            return true;
        }

        // Secret-ish files that should never end up in run artifacts by default.
        if name == ".env" || name.to_string_lossy().starts_with(".env.") {
            return true;
        }
        if name == ".npmrc" || name == ".pypirc" || name == ".netrc" {
            return true;
        }
    }
    false
}
