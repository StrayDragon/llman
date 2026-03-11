use anyhow::{Context, Result};
use std::io::Write;
use std::io::{self, ErrorKind};
use std::path::Path;
use tempfile::NamedTempFile;

pub(crate) fn atomic_write_with_mode(path: &Path, content: &[u8], mode: Option<u32>) -> Result<()> {
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
