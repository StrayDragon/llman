use crate::arg_utils::split_shell_args;
use crate::config::Config as LlmanConfig;
use crate::x::arena::contest::{ContestConfigV1, PromptVariantConfig};
use crate::x::arena::dataset::{DatasetConfigV1, TaskKind, TaskSnapshot, requires_repo_template};
use crate::x::arena::jsonl;
use crate::x::arena::models::normalize_openai_api_base;
use crate::x::arena::paths::ArenaPaths;
use anyhow::{Context, Result, anyhow};
use clap::Args;
use llm_json::{RepairOptions, loads};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const ENV_TEST_FAKE_RUNNER: &str = "LLMAN_ARENA_TEST_FAKE_RUNNER";

#[derive(Args, Debug, Clone)]
pub struct GenArgs {
    #[arg(long)]
    pub contest: String,
    #[arg(long)]
    pub dataset: String,
    #[arg(long)]
    pub rounds: u32,
    #[arg(long)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestantSnapshot {
    pub id: String,
    pub model: String,
    pub prompt_id: String,
    pub prompt_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRecord {
    pub match_id: String,
    pub task: TaskSnapshot,
    pub contestant_a: ContestantSnapshot,
    pub contestant_b: ContestantSnapshot,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRecord {
    pub match_id: String,
    pub side: MatchSide,
    pub contestant_id: String,
    pub output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchSide {
    A,
    B,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRecord {
    pub match_id: String,
    pub side: MatchSide,
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub match_id: String,
    pub side: MatchSide,
    pub command: String,
    pub status: VerificationStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    Ok,
    Failed,
    Skipped,
    Error,
}

trait ArenaRunner {
    fn generate_text(
        &self,
        model: &str,
        params: RunnerParams,
        system: &str,
        user: &str,
    ) -> Result<String>;
}

#[derive(Debug, Clone, Copy)]
struct RunnerParams {
    temperature: f32,
    top_p: Option<f32>,
    top_k: Option<i32>,
    max_output_tokens: i32,
    timeout_secs: u64,
    retries: u32,
    structured_output: bool,
}

impl From<&ContestConfigV1> for RunnerParams {
    fn from(value: &ContestConfigV1) -> Self {
        Self {
            temperature: value.temperature,
            top_p: value.top_p,
            top_k: value.top_k,
            max_output_tokens: value.max_output_tokens,
            timeout_secs: value.timeout_secs,
            retries: value.retries,
            structured_output: value.structured_output,
        }
    }
}

pub fn run(args: &GenArgs) -> Result<()> {
    if args.rounds == 0 {
        return Err(anyhow!("--rounds must be > 0"));
    }

    let contest = crate::x::arena::contest::load_by_name(&args.contest)?;
    let dataset = crate::x::arena::dataset::load_by_name(&args.dataset)?;

    validate_gen_inputs(&contest, &dataset)?;

    let paths = ArenaPaths::resolve()?;
    paths.ensure_dirs()?;

    let seed = args.seed.unwrap_or_else(default_seed);
    let run_id = format!("run_{}", seed);
    let run_dir = paths.run_dir(&run_id);
    if run_dir.exists() {
        return Err(anyhow!(
            "Run already exists: {} (use a different --seed)",
            run_dir.display()
        ));
    }
    fs::create_dir_all(&run_dir).with_context(|| format!("create {}", run_dir.display()))?;

    let matches_path = run_dir.join("matches.jsonl");
    let gens_path = run_dir.join("generations.jsonl");
    let applies_path = run_dir.join("applies.jsonl");
    let verifs_path = run_dir.join("verifications.jsonl");

    let mut matches_w = jsonl::create_writer(&matches_path)?;
    let mut gens_w = jsonl::create_writer(&gens_path)?;
    let mut applies_w = jsonl::create_writer(&applies_path)?;
    let mut verifs_w = jsonl::create_writer(&verifs_path)?;

    let contestants = expand_contestants(&contest)?;
    if contestants.len() < 2 {
        return Err(anyhow!(
            "Not enough contestants (need >= 2, got {})",
            contestants.len()
        ));
    }

    let llman = LlmanConfig::new()?;
    let mut rng = PseudoRng::new(seed);
    let runner: Box<dyn ArenaRunner> = if env_truthy(ENV_TEST_FAKE_RUNNER) {
        Box::new(FakeRunner)
    } else {
        Box::new(OpenAiRunner::from_env()?)
    };

    for i in 0..args.rounds {
        let task = pick_task(&dataset, &mut rng)?;
        let (a, b) = pick_two(&contestants, &mut rng)?;
        let match_id = format!("{:06}", i + 1);

        let record = MatchRecord {
            match_id: match_id.clone(),
            task: task.clone(),
            contestant_a: a.clone(),
            contestant_b: b.clone(),
            seed,
        };
        jsonl::write_line(&mut matches_w, &record)?;

        let gen_a = generate_one(runner.as_ref(), &llman, &contest, &record, MatchSide::A)?;
        jsonl::write_line(&mut gens_w, &gen_a)?;
        if record.task.kind == TaskKind::Repo {
            let (apply, verifs) = eval_repo_generation(&dataset, &contest, &record, &gen_a)?;
            jsonl::write_line(&mut applies_w, &apply)?;
            for v in verifs {
                jsonl::write_line(&mut verifs_w, &v)?;
            }
        }

        let gen_b = generate_one(runner.as_ref(), &llman, &contest, &record, MatchSide::B)?;
        jsonl::write_line(&mut gens_w, &gen_b)?;
        if record.task.kind == TaskKind::Repo {
            let (apply, verifs) = eval_repo_generation(&dataset, &contest, &record, &gen_b)?;
            jsonl::write_line(&mut applies_w, &apply)?;
            for v in verifs {
                jsonl::write_line(&mut verifs_w, &v)?;
            }
        }

        matches_w.flush()?;
        gens_w.flush()?;
        applies_w.flush()?;
        verifs_w.flush()?;
    }

    println!("✅ Run created: {}", run_dir.display());
    println!("run_id: {run_id}");
    Ok(())
}

fn validate_gen_inputs(contest: &ContestConfigV1, dataset: &DatasetConfigV1) -> Result<()> {
    if contest.models.is_empty() {
        return Err(anyhow!("Contest models must be non-empty"));
    }
    if requires_repo_template(dataset) && dataset.repo_template_path.is_none() {
        return Err(anyhow!(
            "Dataset contains repo tasks but repo_template_path is missing"
        ));
    }
    if contest.verify.is_empty() {
        return Err(anyhow!(
            "Contest verify commands must be non-empty (set `verify = [\"...\"]`)"
        ));
    }
    Ok(())
}

fn expand_contestants(contest: &ContestConfigV1) -> Result<Vec<ContestantSnapshot>> {
    let mut out = Vec::new();
    for PromptVariantConfig { id, prompt_name } in &contest.prompts {
        for model in &contest.models {
            out.push(ContestantSnapshot {
                id: format!("{id}@{model}"),
                model: model.clone(),
                prompt_id: id.clone(),
                prompt_name: prompt_name.clone(),
            });
        }
    }
    Ok(out)
}

fn pick_task(dataset: &DatasetConfigV1, rng: &mut PseudoRng) -> Result<TaskSnapshot> {
    let idx = rng.next_usize(dataset.tasks.len());
    Ok(TaskSnapshot::from(
        dataset.tasks.get(idx).expect("task index"),
    ))
}

fn pick_two<T: Clone>(items: &[T], rng: &mut PseudoRng) -> Result<(T, T)> {
    if items.len() < 2 {
        return Err(anyhow!("need at least 2 items"));
    }
    let a = rng.next_usize(items.len());
    let mut b = rng.next_usize(items.len());
    while b == a {
        b = rng.next_usize(items.len());
    }
    Ok((items[a].clone(), items[b].clone()))
}

fn generate_one(
    runner: &dyn ArenaRunner,
    llman: &LlmanConfig,
    contest: &ContestConfigV1,
    record: &MatchRecord,
    side: MatchSide,
) -> Result<GenerationRecord> {
    let contestant = match side {
        MatchSide::A => &record.contestant_a,
        MatchSide::B => &record.contestant_b,
    };

    let prompt_path = llman.rule_file_path(&contest.app, &contestant.prompt_name);
    let prompt = fs::read_to_string(&prompt_path)
        .with_context(|| format!("read prompt {}", prompt_path.display()))?;

    let user_base = build_user_prompt(contest, record);

    let params = RunnerParams::from(contest);
    let output = generate_with_repairs(
        runner,
        contest,
        &contestant.model,
        params,
        &prompt,
        &user_base,
    )?;

    Ok(GenerationRecord {
        match_id: record.match_id.clone(),
        side,
        contestant_id: contestant.id.clone(),
        output,
    })
}

fn eval_repo_generation(
    dataset: &DatasetConfigV1,
    contest: &ContestConfigV1,
    record: &MatchRecord,
    generation: &GenerationRecord,
) -> Result<(ApplyRecord, Vec<VerificationRecord>)> {
    let repo_template = dataset
        .repo_template_path
        .as_ref()
        .expect("validated repo_template_path");

    let temp = tempfile::TempDir::new().context("create temp dir")?;
    let workspace = temp.path().join("workspace");
    copy_dir_all(repo_template, &workspace)?;

    let apply = git_apply(
        &record.match_id,
        generation.side,
        &workspace,
        &generation.output,
    )?;
    let mut verifs = Vec::new();
    if !apply.ok {
        verifs.push(VerificationRecord {
            match_id: record.match_id.clone(),
            side: generation.side,
            command: "<skipped>".to_string(),
            status: VerificationStatus::Skipped,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
        });
        return Ok((apply, verifs));
    }

    let commands = record
        .task
        .verify
        .clone()
        .unwrap_or_else(|| contest.verify.clone());

    for cmd in commands {
        let parsed =
            split_shell_args(&cmd).map_err(|e| anyhow!("Invalid verify command: {cmd}: {e}"))?;
        if parsed.is_empty() {
            continue;
        }
        let (program, args) = parsed.split_first().expect("non-empty");
        let output = Command::new(program)
            .args(args)
            .current_dir(&workspace)
            .output()
            .with_context(|| format!("run verify: {cmd}"))?;

        let status = output.status.code();
        let stdout = truncate_lossy(&output.stdout, 20_000);
        let stderr = truncate_lossy(&output.stderr, 20_000);

        let ok = output.status.success();
        verifs.push(VerificationRecord {
            match_id: record.match_id.clone(),
            side: generation.side,
            command: cmd.clone(),
            status: if ok {
                VerificationStatus::Ok
            } else {
                VerificationStatus::Failed
            },
            exit_code: status,
            stdout,
            stderr,
        });

        if !ok {
            break;
        }
    }

    Ok((apply, verifs))
}

fn git_apply(match_id: &str, side: MatchSide, workspace: &Path, diff: &str) -> Result<ApplyRecord> {
    let mut child = Command::new("git")
        .arg("apply")
        .arg("--whitespace=nowarn")
        .current_dir(workspace)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("spawn git apply")?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("missing stdin"))?;
        stdin
            .write_all(diff.as_bytes())
            .context("write diff to stdin")?;
        if diff.as_bytes().last() != Some(&b'\n') {
            stdin
                .write_all(b"\n")
                .context("write trailing newline to stdin")?;
        }
    }

    let output = child.wait_with_output().context("wait git apply")?;
    let ok = output.status.success();

    Ok(ApplyRecord {
        match_id: match_id.to_string(),
        side,
        ok,
        stdout: truncate_lossy(&output.stdout, 20_000),
        stderr: truncate_lossy(&output.stderr, 20_000),
    })
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("create {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("read dir {}", src.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!("copy {} -> {}", src_path.display(), dst_path.display())
            })?;
        }
    }
    Ok(())
}

fn truncate_lossy(bytes: &[u8], max_len: usize) -> String {
    let mut s = String::from_utf8_lossy(bytes).to_string();
    if s.len() > max_len {
        s.truncate(max_len);
        s.push_str("\n…(truncated)");
    }
    s
}

fn default_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_nanos() as u64
}

struct PseudoRng {
    state: u64,
}

impl PseudoRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // LCG constants from Numerical Recipes (good enough for deterministic matchmaking).
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper
    }
}

struct OpenAiRunner {
    api_key: String,
    api_base: String,
    runtime: tokio::runtime::Runtime,
}

impl OpenAiRunner {
    fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        if api_key.trim().is_empty() {
            return Err(anyhow!("OPENAI_API_KEY is required"));
        }

        let base = env::var("OPENAI_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                env::var("OPENAI_API_BASE")
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

    async fn generate_once(&self, model: &str, req: adk_core::LlmRequest) -> Result<String> {
        use adk_core::Llm;
        use adk_model::{OpenAIClient, OpenAIConfig};
        use futures::StreamExt;

        // Create a per-request client (simple + avoids interior mutability). Optimize later if needed.
        let client = OpenAIClient::new(OpenAIConfig::compatible(
            self.api_key.clone(),
            self.api_base.clone(),
            model.to_string(),
        ))
        .context("create OpenAI client")?;

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
        Ok(out)
    }
}

impl ArenaRunner for OpenAiRunner {
    fn generate_text(
        &self,
        model: &str,
        params: RunnerParams,
        system: &str,
        user: &str,
    ) -> Result<String> {
        let req = build_request(model, params, system, user);
        self.runtime.block_on(async {
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(params.timeout_secs),
                    self.generate_once(model, req.clone()),
                )
                .await
                {
                    Ok(Ok(text)) => return Ok(text),
                    Ok(Err(_)) if attempt <= params.retries => continue,
                    Ok(Err(e)) => return Err(e),
                    Err(_) if attempt <= params.retries => continue,
                    Err(_) => {
                        return Err(anyhow!(
                            "LLM request timed out after {}s",
                            params.timeout_secs
                        ));
                    }
                }
            }
        })
    }
}

struct FakeRunner;

impl ArenaRunner for FakeRunner {
    fn generate_text(
        &self,
        model: &str,
        _params: RunnerParams,
        system: &str,
        user: &str,
    ) -> Result<String> {
        if user.contains("OUTPUT FORMAT: Output ONLY a unified diff") {
            if system.contains("ARENA_FAKE_DIFF_FAIL") {
                return Ok("not a diff".to_string());
            }

            let content = if system.contains("ARENA_FAKE_DIFF_OK") {
                format!("arena-fake-ok ({model})")
            } else {
                format!("arena-fake ({model})")
            };

            return Ok(format!(
                "diff --git a/arena_fake.txt b/arena_fake.txt\n\
new file mode 100644\n\
--- /dev/null\n\
+++ b/arena_fake.txt\n\
@@ -0,0 +1 @@\n\
+{content}\n"
            ));
        }

        Ok(format!("FAKE({model}): {user}"))
    }
}

fn env_truthy(name: &str) -> bool {
    let Ok(value) = env::var(name) else {
        return false;
    };
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y"
    )
}

fn build_user_prompt(contest: &ContestConfigV1, record: &MatchRecord) -> String {
    let mut user = record.task.prompt.clone();

    match (record.task.kind, contest.structured_output) {
        (TaskKind::Repo, false) => {
            user.push_str(
                "\n\nOUTPUT FORMAT: Output ONLY a unified diff (git apply compatible). No commentary.\n",
            );
        }
        (TaskKind::Repo, true) => {
            user.push_str("\n\nOUTPUT FORMAT: Return ONLY a JSON object with a single string field `output`.\nThe `output` field MUST contain a unified diff (git apply compatible). No commentary.\n");
        }
        (TaskKind::Text, true) => {
            user.push_str(
                "\n\nOUTPUT FORMAT: Return ONLY a JSON object with a single string field `output`.\nNo commentary.\n",
            );
        }
        (TaskKind::Text, false) => {}
    }

    user
}

fn structured_output_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["output"],
        "properties": {
            "output": { "type": "string" }
        }
    })
}

fn parse_structured_output(raw: &str) -> Result<String> {
    let value = match serde_json::from_str::<Value>(raw) {
        Ok(value) => value,
        Err(_) => loads(raw, &RepairOptions::default())
            .map_err(|e| anyhow!("failed to parse structured output JSON: {}", e))?,
    };
    let output = value
        .get("output")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("structured output JSON missing string field `output`"))?;
    Ok(output.to_string())
}

fn generate_with_repairs(
    runner: &dyn ArenaRunner,
    contest: &ContestConfigV1,
    model: &str,
    params: RunnerParams,
    system: &str,
    user_base: &str,
) -> Result<String> {
    let attempts = contest.repair_retries.saturating_add(1);
    let mut last_error: Option<anyhow::Error> = None;
    let mut last_raw: Option<String> = None;

    for attempt_idx in 0..attempts {
        let user = if let Some(err) = &last_error {
            format!(
                "{base}\n\nThe previous output was invalid.\nerror: {err}\n\nTry again. Follow OUTPUT FORMAT exactly.",
                base = user_base,
                err = err
            )
        } else {
            user_base.to_string()
        };

        let raw = runner.generate_text(model, params, system, &user)?;
        last_raw = Some(raw.clone());
        if !contest.structured_output {
            return Ok(raw);
        }

        match parse_structured_output(&raw) {
            Ok(out) => return Ok(out),
            Err(e) => {
                last_error = Some(e);
                if attempt_idx + 1 >= attempts {
                    break;
                }
            }
        }
    }

    if let Some(raw) = last_raw {
        if let Some(err) = last_error {
            eprintln!(
                "Warning: structured output was invalid after {} attempt(s); falling back to raw output. error: {}",
                attempts, err
            );
        }
        return Ok(raw);
    }

    Err(last_error.unwrap_or_else(|| anyhow!("generation failed")))
}

fn build_request(
    model: &str,
    params: RunnerParams,
    system: &str,
    user: &str,
) -> adk_core::LlmRequest {
    use adk_core::{Content, GenerateContentConfig, LlmRequest};

    let system = Content::new("system").with_text(system);
    let user = Content::new("user").with_text(user);
    let cfg = GenerateContentConfig {
        temperature: Some(params.temperature),
        top_p: params.top_p,
        top_k: params.top_k,
        max_output_tokens: Some(params.max_output_tokens),
        response_schema: if params.structured_output {
            Some(structured_output_schema())
        } else {
            None
        },
    };
    LlmRequest::new(model, vec![system, user]).with_config(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn contestant_expansion_uses_prompt_order_then_model_order() {
        let contest = ContestConfigV1 {
            version: 1,
            name: "x".into(),
            app: "codex".into(),
            models: vec!["m1".into(), "m2".into()],
            temperature: 0.0,
            top_p: None,
            top_k: None,
            max_output_tokens: 10,
            timeout_secs: 1,
            retries: 0,
            structured_output: false,
            repair_retries: 0,
            verify: vec!["true".into()],
            prompts: vec![
                PromptVariantConfig {
                    id: "p1".into(),
                    prompt_name: "a".into(),
                },
                PromptVariantConfig {
                    id: "p2".into(),
                    prompt_name: "b".into(),
                },
            ],
        };
        let expanded = expand_contestants(&contest).unwrap();
        let ids = expanded.into_iter().map(|c| c.id).collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "p1@m1", "p1@m2", //
                "p2@m1", "p2@m2"
            ]
        );
    }

    #[test]
    fn git_apply_accepts_diff_without_trailing_newline() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(temp.path())
            .output()
            .expect("git init");

        let diff = "diff --git a/arena_smoke.txt b/arena_smoke.txt\n\
new file mode 100644\n\
--- /dev/null\n\
+++ b/arena_smoke.txt\n\
@@ -0,0 +1 @@\n\
+hello";

        let apply = git_apply("000001", MatchSide::A, temp.path(), diff).expect("git apply");
        assert!(apply.ok, "expected ok apply, stderr: {}", apply.stderr);

        let path = temp.path().join("arena_smoke.txt");
        assert!(path.exists(), "expected {}", path.display());
        assert_eq!(fs::read_to_string(path).expect("read file"), "hello\n");
    }
}
