use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use anyhow::{Context, Result, anyhow};
use chrono::NaiveDate;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub const FREEZE_ARCHIVE_NAME: &str = "freezed_changes.7z.archived";

#[derive(Debug, Clone)]
pub struct FreezeArgs {
    pub before: Option<String>,
    pub keep_recent: usize,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ThawArgs {
    pub change: Vec<String>,
    pub dest: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ArchivedChangeDir {
    name: String,
    path: PathBuf,
    date: NaiveDate,
}

pub fn run_freeze(args: FreezeArgs) -> Result<()> {
    run_freeze_with_root(Path::new("."), args)
}

pub fn run_thaw(args: ThawArgs) -> Result<()> {
    run_thaw_with_root(Path::new("."), args)
}

fn run_freeze_with_root(root: &Path, args: FreezeArgs) -> Result<()> {
    let archive_dir = archive_root(root);
    if !archive_dir.exists() {
        return Err(anyhow!(
            "archive directory not found: {}",
            archive_dir.display()
        ));
    }
    let before_date = parse_before_date(args.before.as_deref())?;
    let candidates = select_freeze_candidates(&archive_dir, before_date, args.keep_recent)?;
    let freeze_file = archive_dir.join(FREEZE_ARCHIVE_NAME);

    if args.dry_run {
        println!("Dry run: freeze target {}", freeze_file.display());
        if candidates.is_empty() {
            println!("No archived changes selected for freezing.");
        } else {
            println!("Would freeze {} archived changes:", candidates.len());
            for candidate in &candidates {
                println!("  - {}", candidate.name);
            }
        }
        return Ok(());
    }

    if candidates.is_empty() {
        println!("No archived changes selected for freezing.");
        return Ok(());
    }

    let tempdir = tempfile::tempdir().context("create tempdir for freeze staging")?;
    let staging_root = tempdir.path().join("staging");
    fs::create_dir_all(&staging_root).context("create staging root")?;

    if freeze_file.exists() {
        sevenz_rust2::decompress_file(&freeze_file, &staging_root)
            .map_err(|e| anyhow!("read existing freeze archive failed: {e}"))?;
    }

    for candidate in &candidates {
        let target = staging_root.join(&candidate.name);
        if target.exists() {
            fs::remove_dir_all(&target)
                .with_context(|| format!("remove existing staged dir {}", target.display()))?;
        }
        copy_dir_recursive(&candidate.path, &target)?;
    }

    let temp_archive = archive_dir.join(format!(".{}.tmp", FREEZE_ARCHIVE_NAME));
    if temp_archive.exists() {
        if temp_archive.is_dir() {
            fs::remove_dir_all(&temp_archive)?;
        } else {
            fs::remove_file(&temp_archive)?;
        }
    }

    sevenz_rust2::compress_to_path(&staging_root, &temp_archive)
        .map_err(|e| anyhow!("write freeze archive failed: {e}"))?;
    replace_file_atomically(&temp_archive, &freeze_file)?;

    for candidate in &candidates {
        fs::remove_dir_all(&candidate.path).with_context(|| {
            format!(
                "remove frozen source directory {}",
                candidate.path.display()
            )
        })?;
    }

    println!(
        "Froze {} archived changes into {}",
        candidates.len(),
        freeze_file.display()
    );
    Ok(())
}

fn run_thaw_with_root(root: &Path, args: ThawArgs) -> Result<()> {
    let archive_dir = archive_root(root);
    let freeze_file = archive_dir.join(FREEZE_ARCHIVE_NAME);
    if !freeze_file.exists() {
        return Err(anyhow!(
            "freeze archive not found: {}",
            freeze_file.display()
        ));
    }

    for change in &args.change {
        validate_sdd_id(change, "change")?;
    }

    let dest = args.dest.unwrap_or_else(|| archive_dir.join(".thawed"));
    fs::create_dir_all(&dest)
        .with_context(|| format!("create thaw destination {}", dest.display()))?;

    if args.change.is_empty() {
        sevenz_rust2::decompress_file(&freeze_file, &dest)
            .map_err(|e| anyhow!("thaw archive failed: {e}"))?;
        println!("Thawed archive to {}", dest.display());
        return Ok(());
    }

    let tempdir = tempfile::tempdir().context("create tempdir for selective thaw")?;
    sevenz_rust2::decompress_file(&freeze_file, tempdir.path())
        .map_err(|e| anyhow!("thaw archive failed: {e}"))?;

    for change in &args.change {
        let src = tempdir.path().join(change);
        if !src.exists() {
            return Err(anyhow!(
                "archived change '{}' not found in freeze archive",
                change
            ));
        }
        let target = dest.join(change);
        if target.exists() {
            fs::remove_dir_all(&target)
                .with_context(|| format!("remove existing thaw target {}", target.display()))?;
        }
        copy_dir_recursive(&src, &target)?;
    }

    println!(
        "Thawed {} selected archived changes to {}",
        args.change.len(),
        dest.display()
    );
    Ok(())
}

fn parse_before_date(value: Option<&str>) -> Result<Option<NaiveDate>> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .map_err(|_| anyhow!("invalid --before date '{}', expected YYYY-MM-DD", raw))?;
            Ok(Some(date))
        }
    }
}

fn select_freeze_candidates(
    archive_dir: &Path,
    before: Option<NaiveDate>,
    keep_recent: usize,
) -> Result<Vec<ArchivedChangeDir>> {
    let mut candidates = collect_archived_change_dirs(archive_dir)?;
    if let Some(limit) = before {
        candidates.retain(|entry| entry.date < limit);
    }
    candidates.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.name.cmp(&b.name)));
    if keep_recent >= candidates.len() {
        return Ok(Vec::new());
    }
    let keep_until = candidates.len() - keep_recent;
    Ok(candidates.into_iter().take(keep_until).collect())
}

fn collect_archived_change_dirs(archive_dir: &Path) -> Result<Vec<ArchivedChangeDir>> {
    let mut entries = Vec::new();
    let date_dir_re = Regex::new(r"^\d{4}-\d{2}-\d{2}-.+$").expect("compile regex");
    for entry in fs::read_dir(archive_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !date_dir_re.is_match(&name) {
            continue;
        }
        let date = parse_dir_date(&name)
            .ok_or_else(|| anyhow!("invalid archive directory date prefix '{}'", name))?;
        entries.push(ArchivedChangeDir {
            name,
            path: entry.path(),
            date,
        });
    }
    Ok(entries)
}

fn parse_dir_date(name: &str) -> Option<NaiveDate> {
    let prefix = name.get(..10)?;
    NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok()
}

fn replace_file_atomically(temp_file: &Path, final_file: &Path) -> Result<()> {
    let backup_file = final_file.with_extension("archived.bak");
    if backup_file.exists() {
        fs::remove_file(&backup_file).with_context(|| {
            format!(
                "remove stale backup file before replace {}",
                backup_file.display()
            )
        })?;
    }

    let had_existing = final_file.exists();
    if had_existing {
        fs::rename(final_file, &backup_file).with_context(|| {
            format!(
                "move existing freeze archive to backup {}",
                backup_file.display()
            )
        })?;
    }

    if let Err(err) = fs::rename(temp_file, final_file) {
        if had_existing {
            let _ = fs::rename(&backup_file, final_file);
        }
        return Err(anyhow!(
            "replace freeze archive failed (target: {}): {}",
            final_file.display(),
            err
        ));
    }

    if had_existing && backup_file.exists() {
        fs::remove_file(&backup_file).with_context(|| {
            format!(
                "remove freeze archive backup after successful replace {}",
                backup_file.display()
            )
        })?;
    }

    Ok(())
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("create directory {}", to.display()))?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else if file_type.is_file() {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst)
                .with_context(|| format!("copy file {} -> {}", src.display(), dst.display()))?;
        }
    }
    Ok(())
}

fn archive_root(root: &Path) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join("archive")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn collect_archived_changes_ignores_non_date_dirs() {
        let dir = tempdir().expect("tempdir");
        let archive = dir.path().join("archive");
        fs::create_dir_all(archive.join("2026-01-01-a")).expect("mkdir");
        fs::create_dir_all(archive.join(".thawed")).expect("mkdir");
        fs::create_dir_all(archive.join("notes")).expect("mkdir");
        fs::write(archive.join("freezed_changes.7z.archived"), b"x").expect("write");

        let entries = collect_archived_change_dirs(&archive).expect("collect");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "2026-01-01-a");
    }

    #[test]
    fn select_candidates_respects_before_and_keep_recent() {
        let dir = tempdir().expect("tempdir");
        let archive = dir.path().join("archive");
        fs::create_dir_all(archive.join("2026-01-01-a")).expect("mkdir");
        fs::create_dir_all(archive.join("2026-01-02-b")).expect("mkdir");
        fs::create_dir_all(archive.join("2026-01-03-c")).expect("mkdir");

        let before = NaiveDate::from_ymd_opt(2026, 1, 4).expect("valid date");
        let selected = select_freeze_candidates(&archive, Some(before), 1).expect("select");
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].name, "2026-01-01-a");
        assert_eq!(selected[1].name, "2026-01-02-b");
    }
}
