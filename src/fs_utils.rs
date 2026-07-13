use anyhow::{Context, Result, bail};
use std::io::Write;
use std::io::{self, ErrorKind};
use std::path::Path;
use tempfile::NamedTempFile;

/// Default maximum file size for structured config / data files (10 MiB).
#[allow(dead_code)]
pub(crate) const DEFAULT_MAX_READ_BYTES: u64 = 10 * 1024 * 1024;

/// Read a file whose size is known to be at most `max_bytes`.
#[allow(dead_code)]
///
/// Checks `metadata().len()` before reading so that a huge file is rejected
/// without allocating a large buffer.  Returns an error when the file exceeds
/// the limit.
pub(crate) fn read_with_max_size(path: &Path, max_bytes: u64) -> Result<String> {
    let metadata =
        std::fs::metadata(path).with_context(|| format!("read metadata of {}", path.display()))?;
    if metadata.len() > max_bytes {
        bail!(
            "file {} is {} bytes (max allowed: {} bytes)",
            path.display(),
            metadata.len(),
            max_bytes,
        );
    }
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

pub(crate) fn atomic_write_with_mode(path: &Path, content: &[u8], mode: Option<u32>) -> Result<()> {
    // Do not follow symlinks: if path is a symlink, replace it with a regular file.
    if path.is_symlink() {
        std::fs::remove_file(path)
            .with_context(|| format!("remove symlink before write: {}", path.display()))?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    let mut tmp = NamedTempFile::new_in(parent)
        .with_context(|| format!("create temp file under {}", parent.display()))?;
    tmp.write_all(content)
        .with_context(|| format!("write temp file for {}", path.display()))?;
    tmp.flush()
        .with_context(|| format!("flush temp file for {}", path.display()))?;

    #[cfg(unix)]
    if let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(tmp.path(), perms)
            .with_context(|| format!("set temp file permissions for {}", path.display()))?;
    }

    tmp.persist(path)
        .map(|_| ())
        .with_context(|| format!("persist {}", path.display()))
}

pub(crate) fn atomic_write_new_with_mode(
    path: &Path,
    content: &[u8],
    mode: Option<u32>,
) -> Result<bool> {
    // `persist_noclobber` already refuses to overwrite an existing file,
    // but if `path` is a dangling symlink the OS may follow it into nowhere
    // or create a new file at the target.  Remove the symlink first so that
    // `persist_noclobber` sees a clean non-existing target.
    if path.is_symlink() {
        std::fs::remove_file(path).with_context(|| {
            format!("remove symlink before noclobber write: {}", path.display())
        })?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    let mut tmp = NamedTempFile::new_in(parent)
        .with_context(|| format!("create temp file under {}", parent.display()))?;
    tmp.write_all(content)
        .with_context(|| format!("write temp file for {}", path.display()))?;
    tmp.flush()
        .with_context(|| format!("flush temp file for {}", path.display()))?;

    #[cfg(unix)]
    if let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(tmp.path(), perms)
            .with_context(|| format!("set temp file permissions for {}", path.display()))?;
    }

    match tmp.persist_noclobber(path) {
        Ok(_) => Ok(true),
        Err(err) if err.error.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(io::Error::new(err.error.kind(), err.error))
            .with_context(|| format!("persist {}", path.display())),
    }
}
