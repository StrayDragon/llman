pub mod embed;
pub mod index;

pub use index::*;

use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::spec::parser::parse_spec;
use anyhow::{Context as _, Result};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

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
/// Single path: if index exists + fresh + API works → semantic results.
/// Otherwise → clear error, no fallback.
pub fn context_run(task: Option<String>, paths: Vec<String>, top: usize) -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let _config = load_required_config(&llmanspec_dir)?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    // Check freshness — single gate
    match check_freshness(&context_dir, &specs_dir) {
        IndexFreshness::Fresh => {}
        IndexFreshness::Stale { .. } => {
            print_err(
                "index_stale",
                "Index is stale. Run `llman sdd index rebuild --async` to rebuild in background, then retry.",
            );
            return Ok(());
        }
        IndexFreshness::Missing => {
            print_err(
                "index_missing",
                "No embedding index found.\n\
                 1. Run `llman sdd index rebuild --run-async` in background (~30s) then retry.\n\
                 2. Run `llman sdd index rebuild` (synchronous).",
            );
            return Ok(());
        }
        IndexFreshness::Corrupted(msg) => {
            print_err(
                "index_corrupted",
                &format!(
                    "Index corrupted ({}). Rebuild with `llman sdd index rebuild`.",
                    msg
                ),
            );
            return Ok(());
        }
    }

    let index = ContextIndex::load(&context_dir)?;

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
            match embed_query(task_text) {
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

/// Helper: embed a single query text via native Rust HTTP
fn embed_query(text: &str) -> std::result::Result<Vec<f32>, String> {
    let cfg = resolve_api_config(ResolveCtx {
        cli_api_host: None,
        cli_api_key: None,
        cli_model: None,
    });
    embed::embed_texts(&[text], &cfg.api_host, &cfg.api_key, &cfg.model)
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

/// Check index freshness and print status.
pub fn index_check() -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    match check_freshness(&context_dir, &specs_dir) {
        IndexFreshness::Fresh => {
            // Try to load metadata for details
            if let Ok(index) = ContextIndex::load(&context_dir) {
                println!(
                    "Index: fresh (built {}, {} specs, {} chunks, model: {})",
                    index.metadata.build_timestamp,
                    index.metadata.spec_count,
                    index.metadata.chunk_count,
                    index.metadata.model,
                );
            } else {
                println!("Index: fresh (details unavailable)");
            }
        }
        IndexFreshness::Stale { .. } => {
            println!("Index: stale (current specs differ from index)");
            println!("Hint: rebuild with `llman sdd index rebuild`");
        }
        IndexFreshness::Missing => {
            // Check for rebuild lock
            match check_rebuild_lock(&context_dir) {
                Ok(Some(lock)) => {
                    println!(
                        "Index: building (PID {}, {:.1}% done, {}/{} chunks)",
                        lock.pid, lock.progress_pct, lock.chunks_done, lock.chunks_total
                    );
                }
                _ => {
                    println!("Index: missing (no embedding index found)");
                    println!("Hint: run `llman sdd index rebuild --api-url <URL>`");
                }
            }
        }
        IndexFreshness::Corrupted(msg) => {
            println!("Index: corrupted ({})", msg);
            println!("Hint: rebuild with `llman sdd index rebuild`");
        }
    }

    Ok(())
}

/// Rebuild the embedding index.
pub fn index_rebuild(
    api_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    _run_async: bool,
) -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    // Resolve API config: CLI args > env vars > defaults
    let cfg = resolve_api_config(ResolveCtx {
        cli_api_host: api_url,
        cli_api_key: api_key,
        cli_model: model,
    });
    let api_host = &cfg.api_host;
    let api_key = &cfg.api_key;
    let model = &cfg.model;

    // Ensure context directory exists
    std::fs::create_dir_all(&context_dir)?;

    // 1. Scan specs and extract chunks
    eprintln!("Scanning specs...");
    let mut specs_meta: Vec<serde_json::Value> = Vec::new();
    let mut chunks: Vec<Chunk> = Vec::new();

    let mut entries: Vec<PathBuf> = fs::read_dir(&specs_dir)?
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

    // 2. Call embedding API via native Rust HTTP
    eprintln!("Embedding {} chunks via {}...", chunks.len(), api_host);

    let chunk_texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
    let all_embeddings = embed::embed_texts(&chunk_texts, api_host, api_key, model)
        .context("Failed to embed chunks")?;

    let all_vectors: Vec<f32> = all_embeddings.iter().flatten().copied().collect();

    let dim = if !chunks.is_empty() {
        all_vectors.len() / chunks.len()
    } else {
        1024
    };

    // 3. Write index files
    eprintln!("Writing index to {}...", context_dir.display());

    // specs.json
    let specs_json = serde_json::to_string_pretty(&specs_meta)?;
    fs::write(context_dir.join("specs.json"), &specs_json)?;

    // chunks.json
    let chunks_json = serde_json::to_string_pretty(&chunks)?;
    fs::write(context_dir.join("chunks.json"), &chunks_json)?;

    // vectors.bin (flat f32)
    let vec_bytes: Vec<u8> = all_vectors.iter().flat_map(|f| f.to_le_bytes()).collect();
    fs::write(context_dir.join("vectors.bin"), &vec_bytes)?;

    // metadata.toml
    let metadata = ContextMetadata {
        version: 1,
        spec_hash: compute_spec_hash(&specs_dir)?,
        spec_count: specs_meta.len(),
        chunk_count: chunks.len(),
        build_timestamp: chrono::Utc::now().to_rfc3339(),
        model: model.clone(),
        embedding_dim: dim,
    };
    let toml_str = toml::to_string(&metadata)?;
    fs::write(context_dir.join("metadata.toml"), &toml_str)?;

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
