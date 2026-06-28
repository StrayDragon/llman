use super::Backend;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Metadata stored in llmanspec/.context/metadata.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub version: u32,
    pub spec_hash: String,
    pub spec_count: usize,
    pub chunk_count: usize,
    pub build_timestamp: String,
    pub model: String,
    pub embedding_dim: usize,
}

/// A per-requirement text chunk for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub spec_id: String,
    pub req_id: String,
    pub text: String,
}

/// Loaded context index (read-only, used by `context` command)
#[derive(Debug, Clone)]
pub struct ContextIndex {
    pub metadata: ContextMetadata,
    pub specs_json: serde_json::Value,
    pub chunks: Vec<Chunk>,
    /// Flat f32 vectors [n_chunks * embedding_dim]
    pub vectors: Vec<f32>,
}

impl ContextIndex {
    /// Load the full index from `.context/` directory
    pub fn load(context_dir: &Path) -> Result<Self> {
        let meta: ContextMetadata = {
            let toml_str = fs::read_to_string(context_dir.join("metadata.toml"))
                .context("Failed to read metadata.toml")?;
            toml::from_str(&toml_str).context("Failed to parse metadata.toml")?
        };

        let specs_json: serde_json::Value = {
            let json_str = fs::read_to_string(context_dir.join("specs.json"))
                .context("Failed to read specs.json")?;
            serde_json::from_str(&json_str).context("Failed to parse specs.json")?
        };

        let chunks: Vec<Chunk> = {
            let json_str = fs::read_to_string(context_dir.join("chunks.json"))
                .context("Failed to read chunks.json")?;
            serde_json::from_str(&json_str).context("Failed to parse chunks.json")?
        };

        let vectors = {
            let vec_bytes =
                fs::read(context_dir.join("vectors.bin")).context("Failed to read vectors.bin")?;
            let expected_len = meta.chunk_count * meta.embedding_dim;
            if vec_bytes.len() != expected_len * 4 {
                anyhow::bail!(
                    "vectors.bin size mismatch: expected {} bytes, got {}",
                    expected_len * 4,
                    vec_bytes.len(),
                );
            }
            vec_bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect()
        };

        Ok(Self {
            metadata: meta,
            specs_json,
            chunks,
            vectors,
        })
    }

    /// Get the embedding vector for a chunk
    pub fn chunk_vector(&self, chunk_idx: usize) -> &[f32] {
        let dim = self.metadata.embedding_dim;
        let start = chunk_idx * dim;
        &self.vectors[start..start + dim]
    }

    /// Number of chunks
    pub fn chunk_count(&self) -> usize {
        self.metadata.chunk_count
    }

    /// Embedding dimension
    pub fn embedding_dim(&self) -> usize {
        self.metadata.embedding_dim
    }
}

/// Compute sha256 hash of all spec files (sorted by path).
pub fn compute_spec_hash(specs_dir: &Path) -> Result<String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(specs_dir)
        .context("Failed to read specs directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path().join("spec.toon"))
        .filter(|p| p.exists())
        .collect();
    entries.sort();

    let mut hasher = Sha256::new();
    for path in &entries {
        let content = fs::read(path).context(format!("Failed to read {}", path.display()))?;
        hasher.update(&content);
    }
    Ok(hex_encode(&hasher.finalize()))
}

/// Freshness status of the embedding index
#[derive(Debug, Clone, PartialEq)]
pub enum IndexFreshness {
    Fresh,
    Stale {
        current_hash: String,
        stored_hash: String,
    },
    Missing,
    Corrupted(String),
}

/// Subdirectory name under `.context/` for each backend's index storage.
pub fn backend_subdir(backend: Backend) -> &'static str {
    match backend {
        Backend::Rag => "rag",
        Backend::Pageindex => "pageindex",
    }
}

/// Resolve the directory holding the given backend's index.
///
/// For the rag backend this falls back to the legacy flat layout (index files
/// directly under `context_dir`) when a `.context/rag/` subdirectory does not
/// yet exist, so existing users keep their indexes after the storage split.
/// The pageindex backend is always scoped to `.context/pageindex/`.
pub fn resolve_backend_dir(context_dir: &Path, backend: Backend) -> PathBuf {
    let sub = context_dir.join(backend_subdir(backend));
    if sub.join("metadata.toml").exists() || sub.join("tree.json").exists() {
        return sub;
    }
    if matches!(backend, Backend::Rag) && context_dir.join("metadata.toml").exists() {
        return context_dir.to_path_buf();
    }
    sub
}

/// Best-effort summary of a pageindex tree index: (doc_count, build_timestamp, chat_model).
///
/// Parsed as generic JSON so this works without depending on the `tree.rs` types.
pub fn pageindex_summary(backend_dir: &Path) -> Option<(usize, String, String)> {
    let content = fs::read_to_string(backend_dir.join("tree.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let docs = value["docs"].as_array().map(|a| a.len()).unwrap_or(0);
    let ts = value["build_timestamp"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let model = value["chat_model"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    Some((docs, ts, model))
}

enum FreshErr {
    Missing,
    Corrupted(String),
}

/// Read the stored spec_hash for the rag backend from `metadata.toml`.
fn read_rag_spec_hash(backend_dir: &Path) -> std::result::Result<String, FreshErr> {
    let meta_path = backend_dir.join("metadata.toml");
    if !meta_path.exists() {
        return Err(FreshErr::Missing);
    }
    let toml_str =
        fs::read_to_string(&meta_path).map_err(|e| FreshErr::Corrupted(e.to_string()))?;
    let meta: ContextMetadata =
        toml::from_str(&toml_str).map_err(|e| FreshErr::Corrupted(e.to_string()))?;
    Ok(meta.spec_hash)
}

/// Read the stored spec_hash for the pageindex backend from `tree.json`.
///
/// Parsed as generic JSON to avoid coupling `index.rs` to the `tree.rs` types
/// (and to keep the Phase-1 freshness check working before `tree.rs` exists).
fn read_pageindex_spec_hash(backend_dir: &Path) -> std::result::Result<String, FreshErr> {
    let tree_path = backend_dir.join("tree.json");
    if !tree_path.exists() {
        return Err(FreshErr::Missing);
    }
    let content = fs::read_to_string(&tree_path).map_err(|e| FreshErr::Corrupted(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| FreshErr::Corrupted(e.to_string()))?;
    value["spec_hash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| FreshErr::Corrupted("tree.json missing spec_hash".to_string()))
}

/// Check the freshness of the index for a specific backend.
pub fn check_freshness(context_dir: &Path, specs_dir: &Path, backend: Backend) -> IndexFreshness {
    let backend_dir = resolve_backend_dir(context_dir, backend);
    let stored_hash = match backend {
        Backend::Rag => read_rag_spec_hash(&backend_dir),
        Backend::Pageindex => read_pageindex_spec_hash(&backend_dir),
    };
    let stored_hash = match stored_hash {
        Ok(h) => h,
        Err(FreshErr::Missing) => return IndexFreshness::Missing,
        Err(FreshErr::Corrupted(msg)) => return IndexFreshness::Corrupted(msg),
    };

    let current_hash = match compute_spec_hash(specs_dir) {
        Ok(h) => h,
        Err(e) => return IndexFreshness::Corrupted(e.to_string()),
    };

    if stored_hash == current_hash {
        IndexFreshness::Fresh
    } else {
        IndexFreshness::Stale {
            current_hash,
            stored_hash,
        }
    }
}

/// Check if a rebuild lock file exists and if the process is still alive
pub fn check_rebuild_lock(context_dir: &Path) -> Result<Option<RebuildLock>> {
    let lock_path = context_dir.join(".rebuild.lock");
    if !lock_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&lock_path)?;
    let lock: RebuildLock = toml::from_str(&content)?;

    // Check if PID is still alive
    let alive = is_pid_alive(lock.pid);
    if !alive {
        // Stale lock file, clean it up
        let _ = fs::remove_file(&lock_path);
        return Ok(None);
    }

    Ok(Some(lock))
}

/// Rebuild lock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildLock {
    pub pid: u32,
    pub started_at: String,
    pub chunks_total: usize,
    pub chunks_done: usize,
    pub progress_pct: f64,
}

/// Check if a process is alive (Unix: uses `kill -0`; Windows not supported)
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Compute cosine similarity between two vectors
pub fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na * nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Z-score normalize a slice of scores
pub fn z_score_normalize(scores: &[f32]) -> Vec<f32> {
    let n = scores.len() as f32;
    if n == 0.0 {
        return Vec::new();
    }
    let mean: f32 = scores.iter().sum::<f32>() / n;
    let variance: f32 = scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / n;
    let std = variance.sqrt();
    if std == 0.0 {
        return vec![0.0; scores.len()];
    }
    scores.iter().map(|s| (s - mean) / std).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_sim_identical() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_sim(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_sim(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_z_score_normalize() {
        let scores = vec![3.0, 1.0, 2.0];
        let normalized = z_score_normalize(&scores);
        assert!((normalized.iter().sum::<f32>()).abs() < 1e-6);
    }

    /// r10: pageindex storage MUST be isolated from rag. A present rag index must
    /// never satisfy a pageindex freshness check.
    #[test]
    fn test_backend_storage_isolation_pageindex_ignores_rag() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let context_dir = root.join(".context");
        let specs_dir = root.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        // Simulate an existing rag index (new isolated layout).
        let rag_dir = context_dir.join("rag");
        std::fs::create_dir_all(&rag_dir).unwrap();
        let hash = compute_spec_hash(&specs_dir).unwrap();
        std::fs::write(
            rag_dir.join("metadata.toml"),
            format!(
                "version = 1\nspec_hash = \"{hash}\"\nspec_count = 1\nchunk_count = 1\n\
                 build_timestamp = \"2026-01-01T00:00:00Z\"\nmodel = \"m\"\nembedding_dim = 4\n"
            ),
        )
        .unwrap();

        // pageindex dir absent → pageindex freshness is Missing, NOT Fresh.
        assert_eq!(
            check_freshness(&context_dir, &specs_dir, Backend::Pageindex),
            IndexFreshness::Missing,
            "pageindex must not silently use the rag index"
        );
        // rag resolves to its own (fresh) dir.
        assert_eq!(
            check_freshness(&context_dir, &specs_dir, Backend::Rag),
            IndexFreshness::Fresh
        );
        // resolve_backend_dir for pageindex points at pageindex/, never rag/.
        assert_eq!(
            resolve_backend_dir(&context_dir, Backend::Pageindex),
            context_dir.join("pageindex")
        );
    }

    /// r10 (legacy compat): an old flat `.context/metadata.toml` is treated as the
    /// rag index, but still does not satisfy pageindex.
    #[test]
    fn test_backend_legacy_layout_treated_as_rag_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let context_dir = root.join(".context");
        let specs_dir = root.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let hash = compute_spec_hash(&specs_dir).unwrap();
        std::fs::create_dir_all(&context_dir).unwrap();
        std::fs::write(
            context_dir.join("metadata.toml"),
            format!(
                "version = 1\nspec_hash = \"{hash}\"\nspec_count = 1\nchunk_count = 1\n\
                 build_timestamp = \"2026-01-01T00:00:00Z\"\nmodel = \"m\"\nembedding_dim = 4\n"
            ),
        )
        .unwrap();

        // Legacy flat layout is recognized as rag (read fallback).
        assert_eq!(
            check_freshness(&context_dir, &specs_dir, Backend::Rag),
            IndexFreshness::Fresh
        );
        assert_eq!(resolve_backend_dir(&context_dir, Backend::Rag), context_dir);
        // ...but pageindex is still missing.
        assert_eq!(
            check_freshness(&context_dir, &specs_dir, Backend::Pageindex),
            IndexFreshness::Missing
        );
    }

    #[test]
    fn test_backend_parse_and_resolve() {
        use super::super::{Backend, resolve_backend};
        assert_eq!(Backend::parse("rag").unwrap(), Backend::Rag);
        assert_eq!(Backend::parse("pageindex").unwrap(), Backend::Pageindex);
        assert_eq!(Backend::parse(" PageIndex ").unwrap(), Backend::Pageindex);
        assert!(Backend::parse("nope").is_err());

        // A present CLI flag wins without consulting the environment.
        assert_eq!(
            resolve_backend(Some(String::from("rag"))).unwrap(),
            Backend::Rag
        );
        // An empty CLI flag falls through to the default (pageindex).
        assert_eq!(
            resolve_backend(Some(String::new())).unwrap(),
            Backend::Pageindex
        );
    }
}
