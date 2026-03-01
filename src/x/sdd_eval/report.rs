use crate::x::sdd_eval::acp;
use crate::x::sdd_eval::paths;
use crate::x::sdd_eval::playbook;
use crate::x::sdd_eval::run::{RunManifestV1, VariantManifestV1};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{SecondsFormat, Utc};
use llm_json::{RepairOptions, loads};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReportV1 {
    pub version: u32,
    pub run_id: String,
    pub generated_at: String,
    pub task_title: String,
    pub variants: Vec<VariantReportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_judge: Option<AiJudgeSummaryV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human: Option<HumanScoresV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantReportV1 {
    pub name: String,
    pub style: String,
    pub agent_kind: String,
    pub agent_preset: String,

    pub iterations_attempted: u32,
    pub files_written: usize,
    pub bytes_written: usize,
    pub terminal_commands: usize,
    pub terminal_success: usize,
    pub denied_operations: usize,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_score: Option<AiJudgeScoreV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_score: Option<HumanScoreV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiJudgeSummaryV1 {
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiJudgeScoreV1 {
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanScoresV1 {
    pub file: String,
    pub variants: BTreeMap<String, HumanScoreV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanScoreV1 {
    pub score: f64,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanPackV1 {
    pub version: u32,
    pub run_id: String,
    pub task_title: String,
    pub variants: Vec<HumanPackVariantV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanPackVariantV1 {
    pub name: String,
    pub style: String,
    pub agent_kind: String,
    pub agent_preset: String,
    pub workspace_dir: String,
    pub session_log: String,
    pub metrics_json: String,
}

pub fn generate(project_root: &Path, run_id: &str) -> Result<()> {
    let run_dir = paths::run_dir(project_root, run_id);
    if !run_dir.exists() {
        bail!("Run not found: {}", run_dir.display());
    }

    let manifest = load_manifest(&run_dir)?;
    let pb = load_playbook(&run_dir)?;

    let mut variants = Vec::new();
    for mv in &manifest.variants {
        let metrics = load_variant_metrics(&run_dir, mv)?;
        variants.push(build_variant_report(mv, &metrics));
    }

    // Optional AI judge.
    let mut ai_summary: Option<AiJudgeSummaryV1> = None;
    if pb.report.ai_judge.enabled {
        let model = pb
            .report
            .ai_judge
            .model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("ai_judge enabled but model missing"))?
            .to_string();
        ai_summary = Some(AiJudgeSummaryV1 {
            model: model.clone(),
        });

        let judge = OpenAiJudge::from_env()?;
        for vr in &mut variants {
            let score = judge
                .score_variant(&model, &pb.task, vr)
                .with_context(|| format!("AI-judge variant {}", vr.name))?;
            vr.ai_score = Some(score);
        }
    }

    // Human pack export (always generated when enabled, even before scoring).
    if pb.report.human.enabled {
        let pack = build_human_pack(&manifest, &pb, &run_dir);
        let pack_path = run_dir.join("human-pack.json");
        fs::write(&pack_path, serde_json::to_vec_pretty(&pack)?)
            .with_context(|| format!("write {}", pack_path.display()))?;

        let template_path = run_dir.join("human-scores.template.json");
        let mut template_variants = BTreeMap::new();
        for v in &manifest.variants {
            template_variants.insert(
                v.name.clone(),
                HumanScoreV1 {
                    score: 0.0,
                    notes: String::new(),
                },
            );
        }
        let template = HumanScoresV1 {
            file: "fill-me".to_string(),
            variants: template_variants,
        };
        fs::write(&template_path, serde_json::to_vec_pretty(&template)?)
            .with_context(|| format!("write {}", template_path.display()))?;
    }

    let human_scores = load_human_scores_if_present(&run_dir)?;
    if let Some(human) = &human_scores {
        for vr in &mut variants {
            if let Some(score) = human.variants.get(&vr.name) {
                vr.human_score = Some(score.clone());
            }
        }
    }

    let report = RunReportV1 {
        version: 1,
        run_id: manifest.run_id.clone(),
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        task_title: pb.task.title.clone(),
        variants,
        ai_judge: ai_summary,
        human: human_scores,
    };

    let report_json_path = run_dir.join("report.json");
    fs::write(&report_json_path, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("write {}", report_json_path.display()))?;

    let report_md_path = run_dir.join("report.md");
    fs::write(&report_md_path, render_report_md(&report))
        .with_context(|| format!("write {}", report_md_path.display()))?;

    Ok(())
}

pub fn import_human(project_root: &Path, run_id: &str, file: &Path) -> Result<()> {
    let run_dir = paths::run_dir(project_root, run_id);
    if !run_dir.exists() {
        bail!("Run not found: {}", run_dir.display());
    }
    if !file.exists() {
        bail!("Human score file not found: {}", file.display());
    }

    let raw = fs::read_to_string(file)
        .with_context(|| format!("read human score file {}", file.display()))?;
    let imported: HumanScoresV1 =
        serde_json::from_str(&raw).with_context(|| "parse human score JSON")?;

    let dst = run_dir.join("human-scores.json");
    fs::write(&dst, serde_json::to_vec_pretty(&imported)?)
        .with_context(|| format!("write {}", dst.display()))?;

    // Re-generate report to reflect merged human scores.
    generate(project_root, run_id).context("re-generate report after import")?;
    Ok(())
}

fn load_manifest(run_dir: &Path) -> Result<RunManifestV1> {
    let path = run_dir.join("manifest.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn load_playbook(run_dir: &Path) -> Result<playbook::Playbook> {
    let path = run_dir.join("playbook.yaml");
    playbook::load_from_path(&path).with_context(|| format!("load playbook {}", path.display()))
}

fn load_variant_metrics(run_dir: &Path, v: &VariantManifestV1) -> Result<acp::VariantAcpMetricsV1> {
    let path = run_dir
        .join("variants")
        .join(&v.name)
        .join("artifacts")
        .join("acp-metrics.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn build_variant_report(
    mv: &VariantManifestV1,
    metrics: &acp::VariantAcpMetricsV1,
) -> VariantReportV1 {
    let bytes_written: usize = metrics.files_written.iter().map(|r| r.bytes).sum();
    let terminal_success = metrics
        .terminal_commands
        .iter()
        .filter(|r| r.exit_code == Some(0))
        .count();

    VariantReportV1 {
        name: mv.name.clone(),
        style: mv.style.clone(),
        agent_kind: mv.agent_kind.clone(),
        agent_preset: mv.agent_preset.clone(),
        iterations_attempted: metrics.iterations_attempted,
        files_written: metrics.files_written.len(),
        bytes_written,
        terminal_commands: metrics.terminal_commands.len(),
        terminal_success,
        denied_operations: metrics.denied_operations.len(),
        ai_score: None,
        human_score: None,
    }
}

fn build_human_pack(
    manifest: &RunManifestV1,
    pb: &playbook::Playbook,
    run_dir: &Path,
) -> HumanPackV1 {
    let variants = manifest
        .variants
        .iter()
        .map(|v| HumanPackVariantV1 {
            name: v.name.clone(),
            style: v.style.clone(),
            agent_kind: v.agent_kind.clone(),
            agent_preset: v.agent_preset.clone(),
            workspace_dir: run_dir
                .join("variants")
                .join(&v.name)
                .join("workspace")
                .display()
                .to_string(),
            session_log: run_dir
                .join("variants")
                .join(&v.name)
                .join("logs")
                .join("acp-session.jsonl")
                .display()
                .to_string(),
            metrics_json: run_dir
                .join("variants")
                .join(&v.name)
                .join("artifacts")
                .join("acp-metrics.json")
                .display()
                .to_string(),
        })
        .collect::<Vec<_>>();

    HumanPackV1 {
        version: 1,
        run_id: manifest.run_id.clone(),
        task_title: pb.task.title.clone(),
        variants,
    }
}

fn load_human_scores_if_present(run_dir: &Path) -> Result<Option<HumanScoresV1>> {
    let path = run_dir.join("human-scores.json");
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let scores: HumanScoresV1 =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(scores))
}

fn render_report_md(report: &RunReportV1) -> String {
    let mut out = String::new();
    out.push_str("# sdd-eval report\n\n");
    out.push_str(&format!("run_id: `{}`\n\n", report.run_id));

    if let Some(ai) = &report.ai_judge {
        out.push_str(&format!("ai_judge: `{}`\n\n", ai.model));
    }

    out.push_str("## Variants\n\n");
    out.push_str("| variant | style | agent | preset | iters | files_written | bytes_written | term(ok/total) | denied | ai_score | human_score |\n");
    out.push_str("|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n");

    for v in &report.variants {
        let ai_score = v.ai_score.as_ref().map(|s| s.score).unwrap_or(f64::NAN);
        let human_score = v.human_score.as_ref().map(|s| s.score).unwrap_or(f64::NAN);

        let ai_score_str = if ai_score.is_nan() {
            "-".to_string()
        } else {
            format!("{:.2}", ai_score)
        };
        let human_score_str = if human_score.is_nan() {
            "-".to_string()
        } else {
            format!("{:.2}", human_score)
        };

        out.push_str(&format!(
            "| {name} | {style} | {agent} | {preset} | {iters} | {fw} | {bw} | {ok}/{total} | {denied} | {ai} | {human} |\n",
            name = v.name,
            style = v.style,
            agent = v.agent_kind,
            preset = v.agent_preset,
            iters = v.iterations_attempted,
            fw = v.files_written,
            bw = v.bytes_written,
            ok = v.terminal_success,
            total = v.terminal_commands,
            denied = v.denied_operations,
            ai = ai_score_str,
            human = human_score_str
        ));
    }

    out.push('\n');
    out
}

struct OpenAiJudge {
    api_key: String,
    api_base: String,
    runtime: tokio::runtime::Runtime,
}

impl OpenAiJudge {
    fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        if api_key.trim().is_empty() {
            bail!("OPENAI_API_KEY is required for AI judge");
        }

        let base = std::env::var("OPENAI_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                std::env::var("OPENAI_API_BASE")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| "https://api.openai.com".to_string());
        let api_base = normalize_openai_api_base(&base);

        let runtime = tokio::runtime::Runtime::new().context("create tokio runtime")?;
        Ok(Self {
            api_key,
            api_base,
            runtime,
        })
    }

    fn score_variant(
        &self,
        model: &str,
        task: &playbook::TaskConfig,
        variant: &VariantReportV1,
    ) -> Result<AiJudgeScoreV1> {
        let system = "You are a strict evaluator. Return ONLY JSON with fields {\"score\": number, \"reason\": string}. Score range: 0..10.";
        let user = format!(
            "Task: {title}\n\nPrompt:\n{prompt}\n\nVariant: {name}\nstyle: {style}\nagent: {agent}\npreset: {preset}\n\nObjective metrics:\n- iterations_attempted: {iters}\n- files_written: {fw}\n- bytes_written: {bw}\n- terminal_commands: {tc}\n- terminal_success: {ok}\n- denied_operations: {denied}\n\nReturn JSON only.",
            title = task.title,
            prompt = task.prompt,
            name = variant.name,
            style = variant.style,
            agent = variant.agent_kind,
            preset = variant.agent_preset,
            iters = variant.iterations_attempted,
            fw = variant.files_written,
            bw = variant.bytes_written,
            tc = variant.terminal_commands,
            ok = variant.terminal_success,
            denied = variant.denied_operations
        );

        self.runtime.block_on(async {
            use adk_core::Llm;
            use adk_core::{Content, GenerateContentConfig, LlmRequest};
            use adk_model::{OpenAIClient, OpenAIConfig};
            use futures::StreamExt;

            let client = OpenAIClient::new(OpenAIConfig::compatible(
                self.api_key.clone(),
                self.api_base.clone(),
                model.to_string(),
            ))
            .context("create OpenAI client")?;

            let system = Content::new("system").with_text(system);
            let user = Content::new("user").with_text(user);
            let cfg = GenerateContentConfig {
                temperature: Some(0.0),
                top_p: None,
                top_k: None,
                max_output_tokens: Some(400),
                response_schema: Some(ai_judge_score_schema()),
            };
            let req = LlmRequest::new(model, vec![system, user]).with_config(cfg);

            let mut stream = client.generate_content(req, false).await?;
            let mut out = String::new();
            while let Some(chunk) = stream.next().await {
                let resp = chunk?;
                let Some(content) = resp.content else {
                    if resp.turn_complete {
                        break;
                    }
                    continue;
                };
                for part in content.parts {
                    if let Some(text) = part.text() {
                        out.push_str(text);
                    }
                }
                if resp.turn_complete {
                    break;
                }
            }

            parse_ai_score(&out)
        })
    }
}

fn parse_ai_score(raw: &str) -> Result<AiJudgeScoreV1> {
    let value = match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => v,
        Err(_) => loads(raw, &RepairOptions::default())
            .map_err(|e| anyhow!("failed to parse AI judge JSON: {e}"))?,
    };

    let score = value
        .get("score")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow!("AI judge JSON missing numeric field `score`"))?;
    let reason = value
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(AiJudgeScoreV1 { score, reason })
}

fn ai_judge_score_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["score", "reason"],
        "properties": {
            "score": { "type": "number" },
            "reason": { "type": "string" }
        }
    })
}

fn normalize_openai_api_base(input: &str) -> String {
    let mut base = input.trim().trim_end_matches('/').to_string();
    if base.ends_with("/v1") {
        return base;
    }
    base.push_str("/v1");
    base
}
