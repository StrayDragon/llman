pub mod chat;
pub mod index;
pub mod retrieve;
pub mod tree;

pub use index::*;

use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::Result;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

/// Retrieval/index backend (pageindex agentic tree retrieval).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// PageIndex-style agentic tree retrieval (default, sole backend).
    Pageindex,
}

impl Backend {
    /// Parse a backend name (only `pageindex` is supported).
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim().to_ascii_lowercase();
        match s.as_str() {
            "pageindex" | "page-index" => Ok(Backend::Pageindex),
            "rag" => anyhow::bail!(
                "Backend `rag` is no longer supported. Use the default pageindex backend instead:\n\
                 Set `LLMAN_SDD_INDEX_CHAT_MODEL` to a tool-calling chat model, then\n\
                 run `llman sdd index rebuild`."
            ),
            other => anyhow::bail!(
                "invalid backend {:?} (only `pageindex` is supported)",
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

/// Find the llmanspec directory by walking up from start_dir.
///
/// The start dir is canonicalized first: all real callers pass `Path::new(".")`,
/// and `Path::new(".").parent()` returns `Some("")` (an empty path) rather than
/// the real parent. Without canonicalization the loop would exit on the first
/// iteration and never find a `llmanspec/` in an ancestor directory — so the
/// command only worked when run from a directory that *directly* contained
/// `llmanspec/`.
fn find_llmanspec_dir(start_dir: &Path) -> Result<PathBuf> {
    let start = start_dir
        .canonicalize()
        .unwrap_or_else(|_| start_dir.to_path_buf());
    let mut dir = Some(start);
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
/// Uses the pageindex agentic tree retrieval backend.
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
    // Only pageindex is supported.
    let _ = backend;
    context_run_pageindex(&context_dir, &specs_dir, task, paths, top).await
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
            print_err("index_stale", "index stale; run `llman sdd index rebuild`");
            Ok(())
        }
        IndexFreshness::Missing => {
            print_err(
                "index_missing",
                "index missing; run `llman sdd index rebuild`",
            );
            Ok(())
        }
        IndexFreshness::Corrupted(msg) => {
            print_err(
                "index_corrupted",
                &format!("index corrupted ({msg}); run `llman sdd index rebuild`"),
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
            print_err("index_corrupted", &format!("Failed to load index: {e}"));
            return Ok(());
        }
    };

    let chat_cfg = match chat::ChatConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            print_err(
                "api_error",
                &format!("LLMAN_SDD_INDEX_CHAT_MODEL unset; set a tool-calling chat model: {e}"),
            );
            return Ok(());
        }
    };
    let invoker = chat::OpenAiInvoker::new(&chat_cfg);

    let task_str = task.clone().unwrap_or_default();
    let out = match retrieve::retrieve(&invoker, &tree, &task_str, &paths).await {
        Ok(o) => o,
        Err(e) => {
            print_err("api_error", &format!("retrieval failed: {e}"));
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

/// Check index freshness for the pageindex backend and print status.
pub fn index_check() -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    print_index_status(&context_dir, &specs_dir);

    Ok(())
}

fn backend_label() -> &'static str {
    "pageindex"
}

fn print_index_status(context_dir: &Path, specs_dir: &Path) {
    let label = backend_label();
    match check_freshness(context_dir, specs_dir, Backend::Pageindex) {
        IndexFreshness::Fresh => {
            let backend_dir = resolve_backend_dir(context_dir, Backend::Pageindex);
            match pageindex_summary(&backend_dir) {
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
            }
        }
        IndexFreshness::Stale { .. } => println!(
            "[{}] stale (current specs differ from index). Rebuild: `llman sdd index rebuild`",
            label,
        ),
        IndexFreshness::Missing => {
            println!("[{}] missing. Build: `llman sdd index rebuild`", label,);
        }
        IndexFreshness::Corrupted(msg) => println!(
            "[{}] corrupted ({}). Rebuild: `llman sdd index rebuild`",
            label, msg,
        ),
    }
}

/// Rebuild the pageindex tree index.
pub async fn index_rebuild(
    _api_url: Option<String>,
    _model: Option<String>,
    _api_key: Option<String>,
    _run_async: bool,
    _backend: Backend,
) -> Result<()> {
    let llmanspec_dir = find_llmanspec_dir(Path::new("."))?;
    let context_dir = llmanspec_dir.join(".context");
    let specs_dir = llmanspec_dir.join("specs");

    index_rebuild_pageindex(&context_dir, &specs_dir).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestProcess;

    /// Walking up from an absolute nested path finds an ancestor `llmanspec/`.
    #[test]
    fn find_llmanspec_dir_walks_up_from_absolute_subdir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("llmanspec")).unwrap();
        let nested = root.join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();

        let found = find_llmanspec_dir(&nested).unwrap();
        assert_eq!(found, root.join("llmanspec"));
    }

    /// Regression: the real-world start is `Path::new(".")` (every caller in
    /// this module passes it). `Path::new(".").parent()` is `Some("")`, which
    /// used to stop traversal immediately, so `sdd context/index` only worked
    /// when cwd *directly* contained `llmanspec/`. Canonicalization fixes it.
    /// Uses `TestProcess` to chdir safely (serialized + restored on drop).
    #[test]
    fn find_llmanspec_dir_walks_up_from_relative_dot() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        std::fs::create_dir_all(root.join("llmanspec")).unwrap();
        let nested = root.join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();

        let mut proc = TestProcess::new();
        proc.chdir(&nested).unwrap();

        let found = find_llmanspec_dir(Path::new(".")).unwrap();
        assert_eq!(found.canonicalize().unwrap(), root.join("llmanspec"));
        // proc restores cwd on drop
    }
}
