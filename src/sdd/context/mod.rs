pub mod chat;
pub mod embed;
pub mod index;
pub mod retrieve;
pub mod tree;

pub use index::*;

use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::parser::parse_spec;
use anyhow::{Context as _, Result};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

/// Which retrieval/index backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Traditional embedding RAG (vector similarity).
    Rag,
    /// PageIndex-style agentic tree retrieval (default).
    Pageindex,
}

impl Backend {
    /// Parse a backend name (`rag` / `pageindex`).
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "rag" => Ok(Backend::Rag),
            "pageindex" | "page-index" => Ok(Backend::Pageindex),
            other => anyhow::bail!(
                "invalid backend {:?} (expected `rag` or `pageindex`)",
                other
            ),
        }
    }
}

/// Resolve the effective backend with priority: CLI flag > env var > default.
///
/// - `cli`: value of the `--backend` flag, if present.
/// - `LLMAN_SDD_INDEX_BACKEND`: environment override.
/// - default: `pageindex`.
pub fn resolve_backend(cli: Option<String>) -> Result<Backend> {
    if let Some(raw) = cli {
        let raw = raw.trim();
        if raw.is_empty() {
            return Ok(Backend::Pageindex);
        }
        return Backend::parse(raw);
    }
    match std::env::var("LLMAN_SDD_INDEX_BACKEND") {
        Ok(v) if !v.trim().is_empty() => Backend::parse(&v),
        _ => Ok(Backend::Pageindex),
    }
}

/// Find the llmanspec directory by walking up from start_dir
fn find_llmanspec_dir(start_dir: &Path) -> Result<PathBuf> {
    let mut dir = Some(start_dir.to_path_buf());
    while let Some(d) = dir {
        let candidate = d.join(LLMANSPEC_DIR_NAME);
        if candidate.is_dir() {
            return Ok(candidate);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    anyhow::bail!(
        "Could not find {} directory. Run `llman sdd init` first.",
        LLMANSPEC_DIR_NAME
    );
}

/// Run the `context` command: find specs relevant to a task and/or paths.
///
/// Dispatches to the selected backend. Both backends emit the same top-level JSON
/// shape (`status`/`direct`/`related`/`summary`); `status.quality` is `semantic`
/// for rag and `agentic` for pageindex.
pub async fn context_run(
    task: Option<String>,
    paths: Vec<String>,
    top: usize,
    backend: Backend,
) -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let _config = load_required_config(&llmanspec_dir)?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    match backend {
        Backend::Rag => context_run_rag(&context_dir, &specs_dir, task, paths, top).await,
        Backend::Pageindex => {
            context_run_pageindex(&context_dir, &specs_dir, task, paths, top).await
        }
    }
}

/// rag backend: vector-similarity retrieval over the embedding index.
async fn context_run_rag(
    context_dir: &Path,
    specs_dir: &Path,
    task: Option<String>,
    paths: Vec<String>,
    top: usize,
) -> Result<()> {
    // Check freshness — single gate, scoped to the rag backend.
    match check_freshness(context_dir, specs_dir, Backend::Rag) {
        IndexFreshness::Fresh => {}
        IndexFreshness::Stale { .. } => {
            print_err(
                "index_stale",
                "Index is stale. Run `llman sdd index rebuild --backend rag --run-async` to rebuild in background, then retry.",
            );
            return Ok(());
        }
        IndexFreshness::Missing => {
            print_err(
                "index_missing",
                "No embedding index found for the rag backend.\n\
                 1. Run `llman sdd index rebuild --backend rag --run-async` in background (~30s) then retry.\n\
                 2. Run `llman sdd index rebuild --backend rag` (synchronous).",
            );
            return Ok(());
        }
        IndexFreshness::Corrupted(msg) => {
            print_err(
                "index_corrupted",
                &format!(
                    "Index corrupted ({}). Rebuild with `llman sdd index rebuild --backend rag`.",
                    msg
                ),
            );
            return Ok(());
        }
    }

    let backend_dir = resolve_backend_dir(context_dir, Backend::Rag);
    let index = ContextIndex::load(&backend_dir)?;

    // Path filter: only consider specs whose valid_scope covers any of the given paths
    let path_filtered_specs: std::collections::HashSet<String> = if paths.is_empty() {
        // No paths given: include all specs
        index.chunks.iter().map(|c| c.spec_id.clone()).collect()
    } else {
        // Load spec metadata to get valid_scope for each spec
        let mut filtered = std::collections::HashSet::new();
        for chunk in &index.chunks {
            if filtered.contains(&chunk.spec_id) {
                continue;
            }
            // Check if any of the given paths matches this spec's common paths
            // Since specs.json has validScope, we use the chunk text as heuristic
            // For now: include specs whose name partially matches the paths
            for p in &paths {
                let p_lower = p.to_lowercase();
                // If the path contains the spec name or vice versa
                if p_lower.contains(&chunk.spec_id.to_lowercase())
                    || chunk.spec_id.to_lowercase().contains(&p_lower)
                {
                    filtered.insert(chunk.spec_id.clone());
                    break;
                }
            }
        }
        // If paths given but no filtered specs, include all (path prefix matching)
        if filtered.is_empty() {
            index.chunks.iter().map(|c| c.spec_id.clone()).collect()
        } else {
            filtered
        }
    };

    // Embed query via API
    let query_vector: Option<Vec<f32>> = if let Some(task_text) = &task {
        if !task_text.is_empty() {
            match embed_query(task_text).await {
                Ok(v) => Some(v),
                Err(e) => {
                    print_err("api_error", &e);
                    return Ok(());
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Compute similarities (only for path-filtered specs)
    let chunk_scores: Vec<(usize, f32)> = if let Some(ref qv) = query_vector {
        (0..index.chunk_count())
            .filter(|i| path_filtered_specs.contains(&index.chunks[*i].spec_id))
            .map(|i| {
                let sim = cosine_sim(qv, index.chunk_vector(i));
                (i, sim)
            })
            .collect()
    } else {
        (0..index.chunk_count())
            .filter(|i| path_filtered_specs.contains(&index.chunks[*i].spec_id))
            .map(|i| (i, 1.0))
            .collect()
    };

    // Max-pool per spec → z-score → tier
    let mut spec_max: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    for (i, score) in &chunk_scores {
        let entry = spec_max
            .entry(index.chunks[*i].spec_id.clone())
            .or_insert(0.0);
        if *score > *entry {
            *entry = *score;
        }
    }

    let scores: Vec<f32> = spec_max.values().copied().collect();
    let normalized = z_score_normalize(&scores);
    let spec_ids: Vec<&String> = spec_max.keys().collect();

    let mut direct = Vec::new();
    let mut related = Vec::new();

    for (i, z) in normalized.iter().enumerate() {
        if *z > 0.6 {
            direct.push(serde_json::json!({
                "id": spec_ids[i], "zScore": z, "reason": "semantic match"
            }));
        } else if *z > -0.2 {
            related.push(serde_json::json!({
                "id": spec_ids[i], "zScore": z, "reason": "semantic match"
            }));
        }
    }

    direct.sort_by(|a, b| {
        b["zScore"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["zScore"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    related.sort_by(|a, b| {
        b["zScore"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["zScore"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let direct: Vec<_> = direct.into_iter().take(top).collect();
    let related: Vec<_> = related.into_iter().take(top).collect();

    let output = serde_json::json!({
        "status": { "ok": true, "quality": "semantic", "qualityNote": null },
        "direct": direct,
        "related": related,
        "summary": {
            "totalSpecs": spec_max.len(),
            "tierDirect": direct.len(),
            "tierRelated": related.len(),
            "unrelatedCount": spec_max.len().saturating_sub(direct.len() + related.len()),
            "staleWarnings": [],
            "readRecommended": direct.iter().map(|d| d["id"].as_str().unwrap_or("").to_string()).collect::<Vec<_>>(),
            "paths": paths,
        },
    });
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

/// pageindex backend: agentic tree retrieval.
async fn context_run_pageindex(
    context_dir: &Path,
    specs_dir: &Path,
    task: Option<String>,
    paths: Vec<String>,
    top: usize,
) -> Result<()> {
    match check_freshness(context_dir, specs_dir, Backend::Pageindex) {
        IndexFreshness::Fresh => run_pageindex_retrieval(context_dir, task, paths, top).await,
        IndexFreshness::Stale { .. } => {
            print_err(
                "index_stale",
                "pageindex tree index is stale. Run `llman sdd index rebuild --backend pageindex`, then retry.",
            );
            Ok(())
        }
        IndexFreshness::Missing => {
            print_err(
                "index_missing",
                "No pageindex tree index found.\
                 \nRun `llman sdd index rebuild --backend pageindex` (no model required), then retry.",
            );
            Ok(())
        }
        IndexFreshness::Corrupted(msg) => {
            print_err(
                "index_corrupted",
                &format!(
                    "pageindex tree index corrupted ({}). Rebuild with `llman sdd index rebuild --backend pageindex`.",
                    msg
                ),
            );
            Ok(())
        }
    }
}

/// Load the pageindex tree and run the agentic retrieval loop.
async fn run_pageindex_retrieval(
    context_dir: &Path,
    task: Option<String>,
    paths: Vec<String>,
    top: usize,
) -> Result<()> {
    let backend_dir = resolve_backend_dir(context_dir, Backend::Pageindex);
    let tree = match tree::TreeIndex::load(&backend_dir) {
        Ok(t) => t,
        Err(e) => {
            print_err(
                "index_corrupted",
                &format!("Failed to load pageindex tree index: {e}"),
            );
            return Ok(());
        }
    };

    let chat_cfg = match chat::ChatConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            print_err("api_error", &format!("pageindex chat config error: {e}"));
            return Ok(());
        }
    };
    let invoker = chat::OpenAiInvoker::new(&chat_cfg);

    let task_str = task.clone().unwrap_or_default();
    let out = match retrieve::retrieve(&invoker, &tree, &task_str, &paths).await {
        Ok(o) => o,
        Err(e) => {
            print_err("api_error", &format!("pageindex retrieval failed: {e}"));
            return Ok(());
        }
    };

    print_pageindex_output(out, &tree, &paths, top);
    Ok(())
}

/// Render the pageindex retrieval result as the shared output JSON shape.
fn print_pageindex_output(
    out: retrieve::RetrievalOutput,
    tree: &tree::TreeIndex,
    paths: &[String],
    top: usize,
) {
    let direct: Vec<serde_json::Value> = out
        .direct
        .into_iter()
        .take(top)
        .map(|e| serde_json::json!({ "id": e.id, "reason": e.reason }))
        .collect();
    let related: Vec<serde_json::Value> = out
        .related
        .into_iter()
        .take(top)
        .map(|e| serde_json::json!({ "id": e.id, "reason": e.reason }))
        .collect();
    let tier_direct = direct.len();
    let tier_related = related.len();
    let read_recommended: Vec<String> = direct
        .iter()
        .map(|d| d["id"].as_str().unwrap_or("").to_string())
        .collect();
    let quality_note = if out.truncated {
        Some(format!(
            "agentic loop hit the {}-round tool-call limit; result may be incomplete",
            retrieve::MAX_TOOL_ROUNDS
        ))
    } else {
        None
    };

    let output = serde_json::json!({
        "status": { "ok": true, "quality": "agentic", "qualityNote": quality_note },
        "direct": direct,
        "related": related,
        "summary": {
            "totalSpecs": tree.docs.len(),
            "tierDirect": tier_direct,
            "tierRelated": tier_related,
            "unrelatedCount": tree.docs.len().saturating_sub(tier_direct + tier_related),
            "toolCalls": out.tool_calls,
            "staleWarnings": [],
            "readRecommended": read_recommended,
            "paths": paths,
        },
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}

/// Helper: embed a single query text via async-openai.
async fn embed_query(text: &str) -> std::result::Result<Vec<f32>, String> {
    let cfg = resolve_api_config(ResolveCtx {
        cli_api_host: None,
        cli_api_key: None,
        cli_model: None,
    });
    embed::embed_texts(&[text], &cfg.api_host, &cfg.api_key, &cfg.model)
        .await
        .map_err(|e| format!("Embedding API error: {}", e))?
        .into_iter()
        .next()
        .ok_or_else(|| "No embeddings in response".to_string())
}

/// Helper: print error JSON
fn print_err(error_kind: &str, msg: &str) {
    let output = serde_json::json!({
        "status": {
            "ok": false,
            "quality": "unavailable",
            "qualityNote": msg,
            "errorKind": error_kind,
        },
        "direct": [],
        "related": [],
        "summary": { "totalSpecs": 0, "error": true },
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}

/// Check index freshness for every backend and print status.
pub fn index_check() -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    // pageindex is the default, list it first.
    print_index_status(&context_dir, &specs_dir, Backend::Pageindex);
    print_index_status(&context_dir, &specs_dir, Backend::Rag);

    Ok(())
}

fn backend_label(backend: Backend) -> &'static str {
    match backend {
        Backend::Pageindex => "pageindex",
        Backend::Rag => "rag",
    }
}

fn print_index_status(context_dir: &Path, specs_dir: &Path, backend: Backend) {
    let label = backend_label(backend);
    match check_freshness(context_dir, specs_dir, backend) {
        IndexFreshness::Fresh => {
            let backend_dir = resolve_backend_dir(context_dir, backend);
            match backend {
                Backend::Rag => {
                    if let Ok(index) = ContextIndex::load(&backend_dir) {
                        println!(
                            "[{}] fresh (built {}, {} specs, {} chunks, model: {})",
                            label,
                            index.metadata.build_timestamp,
                            index.metadata.spec_count,
                            index.metadata.chunk_count,
                            index.metadata.model,
                        );
                    } else {
                        println!("[{}] fresh (details unavailable)", label);
                    }
                }
                Backend::Pageindex => match pageindex_summary(&backend_dir) {
                    Some((docs, ts, model)) => {
                        let model = if model.is_empty() {
                            "<unset>".to_string()
                        } else {
                            model
                        };
                        println!(
                            "[{}] fresh (built {}, {} specs, chat model: {})",
                            label, ts, docs, model,
                        )
                    }
                    None => println!("[{}] fresh (details unavailable)", label),
                },
            }
        }
        IndexFreshness::Stale { .. } => println!(
            "[{}] stale (current specs differ from index). Rebuild: `llman sdd index rebuild --backend {}`",
            label, label,
        ),
        IndexFreshness::Missing => {
            if matches!(backend, Backend::Rag)
                && let Ok(Some(lock)) = check_rebuild_lock(context_dir)
            {
                println!(
                    "[{}] building (PID {}, {:.1}% done, {}/{} chunks)",
                    label, lock.pid, lock.progress_pct, lock.chunks_done, lock.chunks_total
                );
                return;
            }
            println!(
                "[{}] missing. Build: `llman sdd index rebuild --backend {}`",
                label, label,
            );
        }
        IndexFreshness::Corrupted(msg) => println!(
            "[{}] corrupted ({}). Rebuild: `llman sdd index rebuild --backend {}`",
            label, msg, label,
        ),
    }
}

/// Rebuild the index for the selected backend.
pub async fn index_rebuild(
    api_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    _run_async: bool,
    backend: Backend,
) -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    match backend {
        Backend::Rag => index_rebuild_rag(&context_dir, &specs_dir, api_url, model, api_key).await,
        Backend::Pageindex => index_rebuild_pageindex(&context_dir, &specs_dir).await,
    }
}

/// rag backend rebuild: scan specs → chunks → embeddings → write index.
async fn index_rebuild_rag(
    context_dir: &Path,
    specs_dir: &Path,
    api_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
) -> Result<()> {
    // Resolve API config: CLI args > env vars > defaults
    let cfg = resolve_api_config(ResolveCtx {
        cli_api_host: api_url,
        cli_api_key: api_key,
        cli_model: model,
    });
    let api_host = &cfg.api_host;
    let api_key = &cfg.api_key;
    let model = &cfg.model;

    // Always write to the canonical rag subdir.
    let rag_dir = context_dir.join(backend_subdir(Backend::Rag));
    std::fs::create_dir_all(&rag_dir)?;

    // 1. Scan specs and extract chunks
    eprintln!("Scanning specs...");
    let mut specs_meta: Vec<serde_json::Value> = Vec::new();
    let mut chunks: Vec<Chunk> = Vec::new();

    let mut entries: Vec<PathBuf> = fs::read_dir(specs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    entries.sort();

    for spec_dir in &entries {
        let spec_file = spec_dir.join("spec.toon");
        if !spec_file.exists() {
            continue;
        }
        let content = fs::read_to_string(&spec_file)?;
        let spec_id = spec_dir.file_name().unwrap().to_string_lossy().to_string();

        // Parse spec using the parser
        match parse_spec(&content, &spec_id) {
            Ok(spec) => {
                specs_meta.push(serde_json::json!({
                    "id": spec_id,
                    "purpose": spec.overview,
                    "reqCount": spec.requirements.len(),
                }));

                for req in &spec.requirements {
                    // Extract req_id and statement from text
                    let text = &req.text;
                    // Try to find req_id pattern (r1, r2, etc.)
                    let req_id = text.split(':').next().unwrap_or("r?").trim();
                    let statement = text.split(':').skip(1).collect::<Vec<_>>().join(":");

                    // Build chunk text: spec_id + purpose + req statement
                    let chunk_text = format!("[{}] {} | {}", spec_id, spec.overview, statement);
                    chunks.push(Chunk {
                        spec_id: spec_id.clone(),
                        req_id: req_id.to_string(),
                        text: chunk_text,
                    });
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {}", spec_id, e);
            }
        }
    }

    eprintln!("Found {} specs, {} chunks", specs_meta.len(), chunks.len());

    if chunks.is_empty() {
        anyhow::bail!(
            "No spec chunks found. Are there spec files in {}?",
            specs_dir.display()
        );
    }

    // 2. Call embedding API via async-openai
    eprintln!("Embedding {} chunks via {}...", chunks.len(), api_host);

    let chunk_texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
    let all_embeddings = embed::embed_texts(&chunk_texts, api_host, api_key, model)
        .await
        .context("Failed to embed chunks")?;

    let all_vectors: Vec<f32> = all_embeddings.iter().flatten().copied().collect();

    let dim = if !chunks.is_empty() {
        all_vectors.len() / chunks.len()
    } else {
        1024
    };

    // 3. Write index files
    eprintln!("Writing index to {}...", rag_dir.display());

    // specs.json
    let specs_json = serde_json::to_string_pretty(&specs_meta)?;
    fs::write(rag_dir.join("specs.json"), &specs_json)?;

    // chunks.json
    let chunks_json = serde_json::to_string_pretty(&chunks)?;
    fs::write(rag_dir.join("chunks.json"), &chunks_json)?;

    // vectors.bin (flat f32)
    let vec_bytes: Vec<u8> = all_vectors.iter().flat_map(|f| f.to_le_bytes()).collect();
    fs::write(rag_dir.join("vectors.bin"), &vec_bytes)?;

    // metadata.toml
    let metadata = ContextMetadata {
        version: 1,
        spec_hash: compute_spec_hash(specs_dir)?,
        spec_count: specs_meta.len(),
        chunk_count: chunks.len(),
        build_timestamp: chrono::Utc::now().to_rfc3339(),
        model: model.clone(),
        embedding_dim: dim,
    };
    let toml_str = toml::to_string(&metadata)?;
    fs::write(rag_dir.join("metadata.toml"), &toml_str)?;

    // Remove rebuild lock if present
    let _ = fs::remove_file(context_dir.join(".rebuild.lock"));

    println!(
        "Index rebuilt successfully ({} specs, {} chunks, dim={}, model={})",
        specs_meta.len(),
        chunks.len(),
        dim,
        model,
    );

    Ok(())
}

/// pageindex backend rebuild: build the spec tree index (no LLM).
///
/// Maps the parsed spec IR (`MainSpecDoc`) directly into a `TreeIndex` and
/// serializes it to `.context/pageindex/tree.json`. No embedding or chat model
/// is contacted — the spec tree is already structured, so building is a pure
/// transform.
async fn index_rebuild_pageindex(context_dir: &Path, specs_dir: &Path) -> Result<()> {
    use crate::sdd::spec::backend::{BACKEND, SpecBackend};
    use crate::sdd::spec::ir::MainSpecDoc;

    let pageindex_dir = context_dir.join(backend_subdir(Backend::Pageindex));
    std::fs::create_dir_all(&pageindex_dir)?;

    eprintln!("Scanning specs for pageindex tree (no LLM)...");
    let mut entries: Vec<PathBuf> = fs::read_dir(specs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    entries.sort();

    let mut parsed: Vec<(String, MainSpecDoc)> = Vec::new();
    for spec_dir in &entries {
        let spec_file = spec_dir.join("spec.toon");
        if !spec_file.exists() {
            continue;
        }
        let content = fs::read_to_string(&spec_file)?;
        let spec_id = spec_dir.file_name().unwrap().to_string_lossy().to_string();
        let ctx = format!("spec `{}`", spec_id);
        match BACKEND.parse_main_spec(&content, &ctx) {
            Ok(doc) => parsed.push((spec_id, doc)),
            Err(e) => eprintln!("Warning: failed to parse {}: {}", spec_id, e),
        }
    }

    if parsed.is_empty() {
        anyhow::bail!(
            "No specs parsed. Are there spec files in {}?",
            specs_dir.display()
        );
    }

    eprintln!("Building tree from {} specs...", parsed.len());
    let docs = tree::build_docs(&parsed);
    let spec_hash = compute_spec_hash(specs_dir)?;
    let chat_model = std::env::var("LLMAN_SDD_INDEX_CHAT_MODEL").unwrap_or_default();
    let tree = tree::TreeIndex::new(
        docs,
        spec_hash,
        chrono::Utc::now().to_rfc3339(),
        chat_model.clone(),
    );
    tree.save(&pageindex_dir)?;

    println!(
        "pageindex tree index rebuilt ({} specs, chat_model={})",
        parsed.len(),
        if chat_model.is_empty() {
            "<unset>"
        } else {
            &chat_model
        },
    );

    Ok(())
}

/// Resolved embedding API configuration.
struct ApiConfig {
    api_host: String,
    api_key: String,
    model: String,
}

/// CLI-sourced partial config for resolution.
struct ResolveCtx {
    cli_api_host: Option<String>,
    cli_api_key: Option<String>,
    cli_model: Option<String>,
}

/// Resolve embedding API config with priority:
/// CLI args > LLMAN_SDD_INDEX_OPENAI_* env vars > hardcoded defaults.
fn resolve_api_config(ctx: ResolveCtx) -> ApiConfig {
    let api_host = ctx.cli_api_host.unwrap_or_else(|| {
        std::env::var("LLMAN_SDD_INDEX_OPENAI_API_HOST")
            .unwrap_or_else(|_| "http://coral:11534/v1".to_string())
    });
    let api_key = ctx.cli_api_key.unwrap_or_else(|| {
        std::env::var("LLMAN_SDD_INDEX_OPENAI_API_KEY")
            .unwrap_or_else(|_| "omlx-gdpzzt2g5351xhqm".to_string())
    });
    let model = ctx.cli_model.unwrap_or_else(|| {
        std::env::var("LLMAN_SDD_INDEX_MODEL").unwrap_or_else(|_| "bge-m3-mlx-8bit".to_string())
    });
    ApiConfig {
        api_host,
        api_key,
        model,
    }
}
