use anyhow::{Result, anyhow};
use ignore::WalkBuilder;
use md5::Context;
use std::fs;
use std::path::{Path, PathBuf};

pub fn hash_skill_dir(root: &Path) -> Result<String> {
    hash_skill_dir_filtered(root, &[])
}

pub fn hash_skill_dir_filtered(root: &Path, exclude_names: &[&str]) -> Result<String> {
    let mut files: Vec<PathBuf> = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_symlink())
        {
            continue;
        }
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && exclude_names.contains(&name)
            {
                continue;
            }
            files.push(path.to_path_buf());
        }
    }

    files.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));

    let mut context = Context::new();
    for file in files {
        let rel = file.strip_prefix(root).map_err(|_| {
            anyhow!(
                "{} {}",
                t!("skills.hash.strip_prefix_failed"),
                file.display()
            )
        })?;
        context.consume(rel.to_string_lossy().as_bytes());
        context.consume([0]);
        let content = fs::read(&file)?;
        context.consume(&content);
        context.consume([0]);
    }
    Ok(format!("{:x}", context.compute()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_hash_changes_on_content_update() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("skill");
        fs::create_dir_all(&root).expect("create skill dir");
        let file_path = root.join("SKILL.md");
        let mut file = fs::File::create(&file_path).expect("create file");
        writeln!(file, "hello").expect("write file");

        let first = hash_skill_dir(&root).expect("hash");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file_path)
            .expect("open file");
        writeln!(file, "hello world").expect("write file");
        let second = hash_skill_dir(&root).expect("hash");

        assert_ne!(first, second);
    }
}
