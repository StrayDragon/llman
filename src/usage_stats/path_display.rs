use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn display_path(path: &Path, current_dir: &Path, verbose: bool) -> String {
    if verbose {
        return path.display().to_string();
    }

    if let Some(repo_root) = find_git_root(current_dir)
        && let Ok(rel) = path.strip_prefix(&repo_root)
        && !rel.as_os_str().is_empty()
    {
        return rel.display().to_string();
    }

    last_two_segments(path)
}

fn last_two_segments(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(os) => Some(os.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    match parts.len() {
        0 => path.display().to_string(),
        1 => parts[0].clone(),
        _ => format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1]),
    }
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if is_git_root(&current) {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn is_git_root(path: &Path) -> bool {
    let git = path.join(".git");
    if let Ok(metadata) = fs::symlink_metadata(&git) {
        return metadata.is_dir() || metadata.is_file();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn default_display_is_repo_relative_when_possible() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).expect("create nested dirs");
        fs::create_dir_all(root.join(".git")).expect("create .git dir");

        let path = root.join("a").join("b").join("c");
        let shown = display_path(&path, &nested, false);
        assert_eq!(shown, "a/b/c");
    }

    #[test]
    fn default_display_falls_back_to_last_two_segments() {
        let current = PathBuf::from("/not/a/repo");
        let path = PathBuf::from("/Users/alice/projects/x/y/z");
        let shown = display_path(&path, &current, false);
        assert_eq!(shown, "y/z");
    }

    #[test]
    fn verbose_display_shows_full_path() {
        let current = PathBuf::from("/not/a/repo");
        let path = PathBuf::from("/Users/alice/projects/x/y/z");
        let shown = display_path(&path, &current, true);
        assert_eq!(shown, path.display().to_string());
    }
}
