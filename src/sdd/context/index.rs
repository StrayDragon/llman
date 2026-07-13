use super::Backend;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
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

/// Freshness status of the pageindex tree index
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

/// Subdirectory name under `.context/` for the pageindex backend's index storage.
pub fn backend_subdir(backend: Backend) -> &'static str {
    let _ = backend;
    "pageindex"
}

/// Resolve the directory holding the pageindex backend's index.
pub fn resolve_backend_dir(context_dir: &Path, backend: Backend) -> PathBuf {
    let _ = backend;
    context_dir.join(backend_subdir(backend))
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

/// Read the stored spec_hash for the pageindex backend from `tree.json`.
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

/// Check the freshness of the pageindex index.
pub fn check_freshness(context_dir: &Path, specs_dir: &Path, backend: Backend) -> IndexFreshness {
    let _ = backend;
    let backend_dir = resolve_backend_dir(context_dir, backend);
    let stored_hash = read_pageindex_spec_hash(&backend_dir);
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

/// Rebuild lock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildLock {
    pub pid: u32,
    pub started_at: String,
    pub chunks_total: usize,
    pub chunks_done: usize,
    pub progress_pct: f64,
}

/// RAII guard that removes `.rebuild.lock` on drop.
pub struct RebuildLockGuard {
    path: PathBuf,
}

impl Drop for RebuildLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

const REBUILD_LOCK_MAX_AGE_SECS: i64 = 6 * 60 * 60;

/// Acquire an exclusive rebuild lock via `create_new`, clearing stale locks first.
pub fn acquire_rebuild_lock(context_dir: &Path) -> Result<RebuildLockGuard> {
    fs::create_dir_all(context_dir).context("create context dir for rebuild lock")?;
    let lock_path = context_dir.join(".rebuild.lock");

    for _ in 0..2 {
        match try_create_rebuild_lock(&lock_path) {
            Ok(guard) => return Ok(guard),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                if clear_stale_rebuild_lock(context_dir)? {
                    continue;
                }
                let holder = read_rebuild_lock(&lock_path)
                    .map(|l| l.pid.to_string())
                    .unwrap_or_else(|_| "?".to_string());
                anyhow::bail!("index rebuild already in progress (pid={holder})");
            }
            Err(err) => {
                return Err(err).context(format!("create rebuild lock {}", lock_path.display()));
            }
        }
    }
    anyhow::bail!("failed to acquire rebuild lock at {}", lock_path.display());
}

fn try_create_rebuild_lock(lock_path: &Path) -> std::io::Result<RebuildLockGuard> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)?;
    let lock = RebuildLock {
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        chunks_total: 0,
        chunks_done: 0,
        progress_pct: 0.0,
    };
    let content = toml::to_string(&lock).map_err(std::io::Error::other)?;
    file.write_all(content.as_bytes())?;
    Ok(RebuildLockGuard {
        path: lock_path.to_path_buf(),
    })
}

fn read_rebuild_lock(lock_path: &Path) -> Result<RebuildLock> {
    let content = fs::read_to_string(lock_path)?;
    Ok(toml::from_str(&content)?)
}

fn clear_stale_rebuild_lock(context_dir: &Path) -> Result<bool> {
    let lock_path = context_dir.join(".rebuild.lock");
    if !lock_path.exists() {
        return Ok(true);
    }
    match read_rebuild_lock(&lock_path) {
        Ok(lock) if is_rebuild_lock_stale(&lock) => {
            let _ = fs::remove_file(&lock_path);
            Ok(true)
        }
        Ok(_) => Ok(false),
        Err(_) => {
            let _ = fs::remove_file(&lock_path);
            Ok(true)
        }
    }
}

fn is_rebuild_lock_stale(lock: &RebuildLock) -> bool {
    if lock_age_secs(lock).is_some_and(|age| age > REBUILD_LOCK_MAX_AGE_SECS) {
        return true;
    }
    #[cfg(unix)]
    {
        if !is_pid_alive(lock.pid) {
            return true;
        }
        if let Some(comm) = read_proc_comm(lock.pid) {
            let comm = comm.trim();
            if !(comm == "llman" || comm.starts_with("llman")) {
                return true;
            }
        }
        false
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn lock_age_secs(lock: &RebuildLock) -> Option<i64> {
    let started = chrono::DateTime::parse_from_rfc3339(&lock.started_at).ok()?;
    Some((chrono::Utc::now() - started.with_timezone(&chrono::Utc)).num_seconds())
}

#[cfg(unix)]
fn read_proc_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm")).ok()
}

/// Check if a rebuild lock file exists and if the holder is still alive.
pub fn check_rebuild_lock(context_dir: &Path) -> Result<Option<RebuildLock>> {
    let lock_path = context_dir.join(".rebuild.lock");
    if !lock_path.exists() {
        return Ok(None);
    }

    let lock = match read_rebuild_lock(&lock_path) {
        Ok(lock) => lock,
        Err(_) => {
            let _ = fs::remove_file(&lock_path);
            return Ok(None);
        }
    };

    if is_rebuild_lock_stale(&lock) {
        let _ = fs::remove_file(&lock_path);
        return Ok(None);
    }

    Ok(Some(lock))
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
        let _ = pid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pageindex_freshness_isolation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let context_dir = root.join(".context");
        let specs_dir = root.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        // Simulate an old rag directory — must not satisfy pageindex freshness.
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
        // resolve_backend_dir for pageindex points at pageindex/, never rag/.
        assert_eq!(
            resolve_backend_dir(&context_dir, Backend::Pageindex),
            context_dir.join("pageindex")
        );
    }

    #[test]
    fn test_backend_parse_rejects_rag() {
        use super::super::Backend;
        assert!(Backend::parse("rag").is_err());
        assert_eq!(Backend::parse("pageindex").unwrap(), Backend::Pageindex);
        assert_eq!(Backend::parse(" PageIndex ").unwrap(), Backend::Pageindex);
        assert!(Backend::parse("nope").is_err());
    }

    #[test]
    fn test_resolve_backend_default() {
        use super::super::Backend;
        use super::super::resolve_backend;
        assert_eq!(resolve_backend(None).unwrap(), Backend::Pageindex);
        assert_eq!(
            resolve_backend(Some("pageindex".to_string())).unwrap(),
            Backend::Pageindex
        );
        assert_eq!(
            resolve_backend(Some(String::new())).unwrap(),
            Backend::Pageindex
        );
    }

    #[test]
    fn test_acquire_rebuild_lock_is_exclusive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let context_dir = tmp.path().join(".context");
        std::fs::create_dir_all(&context_dir).unwrap();

        let first = acquire_rebuild_lock(&context_dir).expect("first lock");
        let second = acquire_rebuild_lock(&context_dir);
        assert!(second.is_err(), "second acquire must fail while held");
        drop(first);
        acquire_rebuild_lock(&context_dir).expect("lock after release");
    }

    #[test]
    fn test_stale_rebuild_lock_cleared_by_age() {
        let tmp = tempfile::TempDir::new().unwrap();
        let context_dir = tmp.path().join(".context");
        std::fs::create_dir_all(&context_dir).unwrap();
        let lock_path = context_dir.join(".rebuild.lock");
        let stale = RebuildLock {
            pid: std::process::id(),
            started_at: (chrono::Utc::now() - chrono::Duration::hours(7)).to_rfc3339(),
            chunks_total: 0,
            chunks_done: 0,
            progress_pct: 0.0,
        };
        std::fs::write(&lock_path, toml::to_string(&stale).unwrap()).unwrap();
        assert!(check_rebuild_lock(&context_dir).unwrap().is_none());
        assert!(!lock_path.exists());
    }
}
