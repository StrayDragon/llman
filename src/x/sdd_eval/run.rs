use crate::x::sdd_eval::{acp, paths, playbook, presets, report};
use crate::x::sdd_eval::secrets::SecretSet;
use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use ignore::WalkBuilder;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
    pb: &playbook::Playbook,
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
    for (variant_id, variant) in &pb.variants {
        let v_dir = variants_dir.join(variant_id);
        let workspace_root = v_dir.join("workspace");
        fs::create_dir_all(&workspace_root)
            .with_context(|| format!("create variant workspace {}", variant_id))?;
        fs::create_dir_all(v_dir.join("logs"))
            .with_context(|| format!("create variant logs {}", variant_id))?;
        fs::create_dir_all(v_dir.join("artifacts"))
            .with_context(|| format!("create variant artifacts {}", variant_id))?;

        let agent_command = variant.agent.command_or_default()?;

        variants.push(VariantManifestV1 {
            name: variant_id.clone(),
            style: match variant.style {
                playbook::WorkflowStyle::Sdd => "sdd".to_string(),
                playbook::WorkflowStyle::SddLegacy => "sdd-legacy".to_string(),
            },
            agent_kind: match variant.agent.kind {
                playbook::AgentKind::ClaudeCode => "claude-code-acp".to_string(),
                playbook::AgentKind::Codex => "codex-acp".to_string(),
                playbook::AgentKind::Fake => "fake-acp".to_string(),
            },
            agent_preset: variant.agent.preset.clone(),
            agent_command,
            agent_args: variant.agent.args.clone(),
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

pub fn execute_run(project_root: &Path, run_dir: &Path, pb: &playbook::Playbook) -> Result<()> {
    let manifest_path = run_dir.join("manifest.json");
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let mut manifest: RunManifestV1 =
        serde_json::from_str(&raw).with_context(|| "parse run manifest")?;

    execute_workflow(project_root, run_dir, pb, &mut manifest)?;

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

fn execute_workflow(
    project_root: &Path,
    run_dir: &Path,
    pb: &playbook::Playbook,
    manifest: &mut RunManifestV1,
) -> Result<()> {
    let jobs = &pb.workflow.jobs;
    let job_order = compute_job_order(jobs)?;

    let run_id = manifest.run_id.clone();
    let mut state = WorkflowState::new();

    for job_id in job_order {
        let job = jobs
            .get(&job_id)
            .expect("job_id comes from job map keys");

        let variants = job
            .strategy
            .as_ref()
            .and_then(|s| s.matrix.as_ref())
            .map(|m| m.variant.clone())
            .unwrap_or_default();

        if variants.is_empty() {
            execute_job(
                project_root,
                run_dir,
                pb,
                manifest,
                &mut state,
                &run_id,
                &job_id,
                job,
                None,
            )?;
        } else {
            for variant_id in variants {
                execute_job(
                    project_root,
                    run_dir,
                    pb,
                    manifest,
                    &mut state,
                    &run_id,
                    &job_id,
                    job,
                    Some(&variant_id),
                )?;
            }
        }
    }

    Ok(())
}

fn compute_job_order(jobs: &IndexMap<String, playbook::WorkflowJob>) -> Result<Vec<String>> {
    for (job_id, job) in jobs {
        for need in &job.needs {
            if !jobs.contains_key(need) {
                bail!("workflow.jobs.{job_id}.needs references unknown job id `{need}`");
            }
        }
    }

    let order: Vec<String> = jobs.keys().cloned().collect();

    let mut indegree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dependents: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for id in &order {
        indegree.insert(id.clone(), 0);
    }
    for (job_id, job) in jobs {
        for need in &job.needs {
            *indegree
                .get_mut(job_id)
                .expect("job_id is present in indegree") += 1;
            dependents
                .entry(need.clone())
                .or_default()
                .push(job_id.clone());
        }
    }

    let mut remaining: std::collections::HashSet<String> = order.iter().cloned().collect();
    let mut out = Vec::with_capacity(order.len());

    while !remaining.is_empty() {
        let mut next: Option<String> = None;
        for id in &order {
            if remaining.contains(id)
                && *indegree.get(id).expect("id present in indegree") == 0
            {
                next = Some(id.clone());
                break;
            }
        }

        let Some(id) = next else {
            let mut involved: Vec<String> = order
                .iter()
                .filter(|id| remaining.contains(*id))
                .cloned()
                .collect();
            involved.sort();
            bail!(
                "workflow.jobs contains a dependency cycle (needs). Remaining jobs: {}",
                involved.join(", ")
            );
        };

        remaining.remove(&id);
        out.push(id.clone());
        if let Some(deps) = dependents.get(&id) {
            for dep in deps {
                let v = indegree
                    .get_mut(dep)
                    .expect("dependent job exists in indegree");
                *v = v.saturating_sub(1);
            }
        }
    }

    Ok(out)
}

struct WorkflowState {
    variants: std::collections::HashMap<String, VariantState>,
}

impl WorkflowState {
    fn new() -> Self {
        Self {
            variants: std::collections::HashMap::new(),
        }
    }

    fn variant_mut(&mut self, variant_id: &str) -> &mut VariantState {
        self.variants
            .entry(variant_id.to_string())
            .or_insert_with(VariantState::new)
    }

}

#[derive(Default)]
struct VariantState {
    secrets: SecretSet,
    run_terminal_seq: u64,
    run_terminal_commands: Vec<acp::TerminalCommandRecord>,
    acp_metrics: Option<acp::VariantAcpMetricsV1>,
}

impl VariantState {
    fn new() -> Self {
        Self::default()
    }

    fn merged_metrics(&self) -> Option<acp::VariantAcpMetricsV1> {
        let mut out = self.acp_metrics.clone()?;
        out.terminal_commands.extend(self.run_terminal_commands.clone());
        Some(out)
    }
}

struct InterpolationContext<'a> {
    matrix_variant: Option<&'a str>,
    variant: Option<&'a playbook::VariantConfig>,
    task: &'a playbook::TaskConfig,
    run_id: &'a str,
    run_dir: &'a Path,
}

fn execute_job(
    project_root: &Path,
    run_dir: &Path,
    pb: &playbook::Playbook,
    manifest: &mut RunManifestV1,
    state: &mut WorkflowState,
    run_id: &str,
    job_id: &str,
    job: &playbook::WorkflowJob,
    matrix_variant: Option<&str>,
) -> Result<()> {
    let ctx = InterpolationContext {
        matrix_variant,
        variant: matrix_variant.and_then(|id| pb.variants.get(id)),
        task: &pb.task,
        run_id,
        run_dir,
    };

    for (idx, step) in job.steps.iter().enumerate() {
        let step_name = step
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("step {}/{}", idx + 1, job.steps.len()));

        if let Some(uses) = step.uses.as_deref() {
            let uses = interpolate_string(uses, &ctx)
                .map_err(|e| anyhow::anyhow!("job `{job_id}` {step_name}: interpolate uses: {e}"))?;
            if let Err(err) = dispatch_builtin_action(
                project_root,
                run_dir,
                pb,
                manifest,
                state,
                matrix_variant,
                &uses,
            ) {
                bail!("job `{job_id}` {step_name}: {uses}: {err}");
            }
            continue;
        }

        if let Some(run) = step.run.as_deref() {
            let run = interpolate_string(run, &ctx)
                .map_err(|e| anyhow::anyhow!("job `{job_id}` {step_name}: interpolate run: {e}"))?;
            let cwd = match step.cwd.as_deref() {
                Some(v) => Some(
                    interpolate_string(v, &ctx)
                        .map_err(|e| anyhow::anyhow!("job `{job_id}` {step_name}: interpolate cwd: {e}"))?,
                ),
                None => None,
            };

            if let Err(err) = execute_run_step(
                project_root,
                run_dir,
                pb,
                manifest,
                state,
                matrix_variant,
                &run,
                cwd.as_deref(),
            ) {
                bail!("job `{job_id}` {step_name}: run: {err}");
            }
            continue;
        }

        bail!("job `{job_id}` {step_name}: step must specify exactly one of `uses` or `run`");
    }

    Ok(())
}

fn interpolate_string(input: &str, ctx: &InterpolationContext<'_>) -> Result<String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${{") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 3..];
        let Some(end) = after_start.find("}}") else {
            bail!("unterminated interpolation (missing `}}`)");
        };
        let raw_path = after_start[..end].trim();
        if raw_path.is_empty() {
            bail!("empty interpolation path is not allowed");
        }
        let value = resolve_interpolation_path(raw_path, ctx)?;
        out.push_str(&value);
        rest = &after_start[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_interpolation_path(path: &str, ctx: &InterpolationContext<'_>) -> Result<String> {
    match path {
        "matrix.variant" => ctx
            .matrix_variant
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("`matrix.variant` is not available (not a matrix job)")),
        "variant.style" => {
            let Some(variant) = ctx.variant else {
                bail!("`variant.style` is not available (no variant context)");
            };
            Ok(match variant.style {
                playbook::WorkflowStyle::Sdd => "sdd".to_string(),
                playbook::WorkflowStyle::SddLegacy => "sdd-legacy".to_string(),
            })
        }
        "variant.agent.kind" => {
            let Some(variant) = ctx.variant else {
                bail!("`variant.agent.kind` is not available (no variant context)");
            };
            Ok(match variant.agent.kind {
                playbook::AgentKind::ClaudeCode => "claude-code-acp".to_string(),
                playbook::AgentKind::Codex => "codex-acp".to_string(),
                playbook::AgentKind::Fake => "fake-acp".to_string(),
            })
        }
        "task.title" => Ok(ctx.task.title.clone()),
        "task.prompt" => Ok(ctx.task.prompt.clone()),
        "run.run_id" => Ok(ctx.run_id.to_string()),
        "run.run_dir" => Ok(ctx.run_dir.display().to_string()),
        other => bail!("unknown interpolation path `{}`", other),
    }
}

fn dispatch_builtin_action(
    project_root: &Path,
    run_dir: &Path,
    pb: &playbook::Playbook,
    manifest: &mut RunManifestV1,
    state: &mut WorkflowState,
    matrix_variant: Option<&str>,
    uses: &str,
) -> Result<()> {
    let Some(action) = BuiltinActionId::parse(uses) else {
        bail!("unknown built-in action: {}", uses);
    };

    match action {
        BuiltinActionId::WorkspacePrepare => {
            let variant_id = matrix_variant
                .ok_or_else(|| anyhow::anyhow!("workspace.prepare requires matrix.variant"))?;
            builtin_workspace_prepare(project_root, run_dir, pb, variant_id)
        }
        BuiltinActionId::SddPrepare => {
            let variant_id =
                matrix_variant.ok_or_else(|| anyhow::anyhow!("sdd.prepare requires matrix.variant"))?;
            builtin_sdd_prepare(run_dir, pb, variant_id)
        }
        BuiltinActionId::AcpSddLoop => {
            let variant_id = matrix_variant
                .ok_or_else(|| anyhow::anyhow!("acp.sdd-loop requires matrix.variant"))?;
            builtin_acp_sdd_loop(run_dir, pb, manifest, state, variant_id)
        }
        BuiltinActionId::ReportGenerate => {
            if matrix_variant.is_some() {
                bail!("report.generate MUST NOT run in a matrix job");
            }
            report::generate(project_root, &manifest.run_id)
        }
    }
}

fn execute_run_step(
    _project_root: &Path,
    run_dir: &Path,
    _pb: &playbook::Playbook,
    _manifest: &mut RunManifestV1,
    state: &mut WorkflowState,
    matrix_variant: Option<&str>,
    run: &str,
    cwd: Option<&str>,
) -> Result<()> {
    if run.contains('\n') || run.contains('\r') {
        bail!("`run` must be a single command (newlines are not allowed)");
    }

    let argv = split_shell_words(run).context("parse `run` as shellwords argv")?;
    if argv.is_empty() {
        bail!("`run` must not be empty");
    }
    for token in &argv {
        if is_shell_operator_token(token) {
            bail!("shell operator token is not allowed in `run`: {}", token);
        }
    }

    let command = argv
        .first()
        .expect("argv checked non-empty")
        .trim()
        .to_string();
    if command.is_empty() {
        bail!("`run` command must not be empty");
    }
    if !acp::is_allowed_command(&command) {
        bail!("terminal command is not allowed: {}", command);
    }

    let (sandbox_root, secrets, record_variant) = match matrix_variant {
        Some(variant_id) => {
            let paths = VariantPaths::new(run_dir, variant_id);
            let secrets = state.variant_mut(variant_id).secrets.clone();
            (paths.workspace_root, secrets, Some(variant_id))
        }
        None => (run_dir.to_path_buf(), SecretSet::new(), None),
    };

    let cwd = resolve_cwd_under_root(&sandbox_root, cwd)?;

    let args = argv.iter().skip(1).cloned().collect::<Vec<_>>();

    let start = Instant::now();
    let output = std::process::Command::new(&command)
        .args(&args)
        .current_dir(&cwd)
        .output();
    let duration_ms = start.elapsed().as_millis() as u64;

    let (exit_code, combined) = match output {
        Ok(out) => {
            let mut combined = String::new();
            combined.push_str(&String::from_utf8_lossy(&out.stdout));
            if !out.stderr.is_empty() {
                if !combined.ends_with('\n') {
                    combined.push('\n');
                }
                combined.push_str(&String::from_utf8_lossy(&out.stderr));
            }
            (out.status.code().map(|c| c as u32), combined)
        }
        Err(err) => (None, format!("Failed to execute: {err}")),
    };

    let redacted = secrets.redact(&combined);
    let (output, truncated) = acp::truncate_tail_to_byte_limit(&redacted, 20_000);

    if let Some(variant_id) = record_variant {
        let v = state.variant_mut(variant_id);
        v.run_terminal_seq += 1;
        let record = acp::TerminalCommandRecord {
            terminal_id: format!("run-{}", v.run_terminal_seq),
            command: command.clone(),
            args: args.clone(),
            cwd: cwd.display().to_string(),
            duration_ms,
            exit_code,
            truncated,
            output: output.clone(),
        };
        v.run_terminal_commands.push(record);
        if v.acp_metrics.is_some() {
            write_variant_metrics(run_dir, variant_id, v)?;
        }
    }

    if exit_code != Some(0) {
        bail!("`run` failed (exit_code={:?})", exit_code);
    }

    Ok(())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinActionId {
    WorkspacePrepare,
    SddPrepare,
    AcpSddLoop,
    ReportGenerate,
}

impl BuiltinActionId {
    fn parse(uses: &str) -> Option<Self> {
        match uses {
            "builtin:sdd-eval/workspace.prepare" => Some(Self::WorkspacePrepare),
            "builtin:sdd-eval/sdd.prepare" => Some(Self::SddPrepare),
            "builtin:sdd-eval/acp.sdd-loop" => Some(Self::AcpSddLoop),
            "builtin:sdd-eval/report.generate" => Some(Self::ReportGenerate),
            _ => None,
        }
    }
}

struct VariantPaths {
    workspace_root: PathBuf,
    logs_dir: PathBuf,
    artifacts_dir: PathBuf,
}

impl VariantPaths {
    fn new(run_dir: &Path, variant_id: &str) -> Self {
        let variant_dir = run_dir.join("variants").join(variant_id);
        Self {
            workspace_root: variant_dir.join("workspace"),
            logs_dir: variant_dir.join("logs"),
            artifacts_dir: variant_dir.join("artifacts"),
        }
    }
}

fn builtin_workspace_prepare(
    project_root: &Path,
    run_dir: &Path,
    _pb: &playbook::Playbook,
    variant_id: &str,
) -> Result<()> {
    let paths = VariantPaths::new(run_dir, variant_id);
    fs::create_dir_all(&paths.logs_dir)
        .with_context(|| format!("create {}", paths.logs_dir.display()))?;
    fs::create_dir_all(&paths.artifacts_dir)
        .with_context(|| format!("create {}", paths.artifacts_dir.display()))?;

    copy_project_root(project_root, &paths.workspace_root)
        .with_context(|| format!("copy project into workspace for {}", variant_id))?;
    Ok(())
}

fn builtin_sdd_prepare(run_dir: &Path, pb: &playbook::Playbook, variant_id: &str) -> Result<()> {
    let Some(variant) = pb.variants.get(variant_id) else {
        bail!("unknown variant id `{}`", variant_id);
    };
    let paths = VariantPaths::new(run_dir, variant_id);
    init_workspace_sdd(&paths.workspace_root, variant.style)
        .with_context(|| format!("init SDD workspace for {}", variant_id))?;
    Ok(())
}

fn builtin_acp_sdd_loop(
    run_dir: &Path,
    pb: &playbook::Playbook,
    manifest: &mut RunManifestV1,
    state: &mut WorkflowState,
    variant_id: &str,
) -> Result<()> {
    let Some(variant) = pb.variants.get(variant_id) else {
        bail!("unknown variant id `{}`", variant_id);
    };
    let paths = VariantPaths::new(run_dir, variant_id);

    fs::create_dir_all(&paths.logs_dir).with_context(|| format!("create {}", paths.logs_dir.display()))?;
    fs::create_dir_all(&paths.artifacts_dir)
        .with_context(|| format!("create {}", paths.artifacts_dir.display()))?;

    let preset = presets::resolve_env(variant.agent.kind, &variant.agent.preset)
        .with_context(|| format!("resolve preset for variant {}", variant_id))?;
    let mut injected_env_keys: Vec<String> = preset.env.keys().cloned().collect();
    injected_env_keys.sort();

    let Some(mv) = manifest.variants.iter_mut().find(|mv| mv.name == variant_id) else {
        bail!("run manifest missing variant `{}`", variant_id);
    };
    mv.injected_env_keys = injected_env_keys;

    let v_state = state.variant_mut(variant_id);
    for value in preset.env.values() {
        v_state.secrets.push(value.clone());
    }

    let session_log_path = paths.logs_dir.join("acp-session.jsonl");
    let agent_command = variant.agent.command_or_default()?;

    let run_result = acp::run_variant(acp::VariantRunParams {
        workspace_root: paths.workspace_root.clone(),
        style: variant.style,
        agent_command,
        agent_args: variant.agent.args.clone(),
        agent_env: preset.env,
        max_iterations: pb.sdd_loop.max_iterations,
        task_title: pb.task.title.clone(),
        task_prompt: pb.task.prompt.clone(),
        session_log_path,
    })
    .with_context(|| format!("run ACP variant {}", variant_id))?;

    v_state.acp_metrics = Some(run_result.metrics);
    write_variant_metrics(run_dir, variant_id, v_state)?;

    Ok(())
}

fn write_variant_metrics(run_dir: &Path, variant_id: &str, v_state: &VariantState) -> Result<()> {
    let Some(merged) = v_state.merged_metrics() else {
        return Ok(());
    };
    let paths = VariantPaths::new(run_dir, variant_id);
    fs::create_dir_all(&paths.artifacts_dir)
        .with_context(|| format!("create {}", paths.artifacts_dir.display()))?;
    let metrics_path = paths.artifacts_dir.join("acp-metrics.json");
    fs::write(&metrics_path, serde_json::to_vec_pretty(&merged)?)
        .with_context(|| format!("write {}", metrics_path.display()))?;
    Ok(())
}

fn resolve_cwd_under_root(root: &Path, cwd: Option<&str>) -> Result<PathBuf> {
    let cwd = cwd
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(".");

    let rel = Path::new(cwd);
    if rel.is_absolute() {
        bail!("`cwd` must be a relative path under the sandbox root");
    }
    if rel.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!("path traversal is not allowed in `cwd`");
    }

    let full = root.join(rel);
    if acp::has_symlink_prefix(root, &full) {
        bail!("symlink traversal is not allowed in `cwd`");
    }
    if !full.is_dir() {
        bail!("`cwd` does not exist or is not a directory: {}", full.display());
    }

    Ok(full)
}

fn is_shell_operator_token(token: &str) -> bool {
    matches!(token, "&&" | "||" | "|" | ";" | ">" | "<")
}

fn split_shell_words(input: &str) -> Result<Vec<String>> {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Normal,
        Single,
        Double,
    }

    let mut mode = Mode::Normal;
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut token_started = false;

    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match mode {
            Mode::Normal => {
                if ch.is_whitespace() {
                    if token_started {
                        tokens.push(std::mem::take(&mut current));
                        token_started = false;
                    }
                    continue;
                }

                match ch {
                    '\'' => {
                        mode = Mode::Single;
                        token_started = true;
                    }
                    '"' => {
                        mode = Mode::Double;
                        token_started = true;
                    }
                    '\\' => {
                        token_started = true;
                        let Some(next) = chars.next() else {
                            bail!("unterminated escape in `run`");
                        };
                        current.push(next);
                    }
                    other => {
                        token_started = true;
                        current.push(other);
                    }
                }
            }
            Mode::Single => match ch {
                '\'' => mode = Mode::Normal,
                other => {
                    token_started = true;
                    current.push(other);
                }
            },
            Mode::Double => match ch {
                '"' => mode = Mode::Normal,
                '\\' => {
                    token_started = true;
                    let Some(next) = chars.next() else {
                        bail!("unterminated escape in `run`");
                    };
                    current.push(next);
                }
                other => {
                    token_started = true;
                    current.push(other);
                }
            },
        }
    }

    if mode != Mode::Normal {
        bail!("unterminated quote in `run`");
    }

    if token_started {
        tokens.push(current);
    }

    Ok(tokens)
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
