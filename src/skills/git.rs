use std::fs;
use std::path::{Path, PathBuf};

pub fn find_git_root(start: &Path) -> Option<PathBuf> {
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
    use tempfile::TempDir;

    #[test]
    fn test_find_git_root() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).expect("create nested dirs");
        fs::create_dir_all(root.join(".git")).expect("create git dir");

        let found = find_git_root(&nested).expect("git root");
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_git_root_none() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        fs::create_dir_all(&root).expect("create dir");
        let found = find_git_root(&root);
        assert!(found.is_none());
    }
}
