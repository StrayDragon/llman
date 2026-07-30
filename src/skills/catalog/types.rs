use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ConfigEntry {
    pub id: String,
    pub agent: String,
    pub scope: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub mode: TargetMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetMode {
    Link,
    Copy,
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetConflictStrategy {
    Overwrite,
    Skip,
}

#[derive(Clone, Debug)]
pub struct SkillsConfig {
    pub targets: Vec<ConfigEntry>,
}

/// A single skills repository source entry declared under `skills.repo[]`.
///
/// `id` is a stable identifier (the positional index as a string when `name` is
/// absent, e.g. `"0"`); `name` is the optional human-readable label used in the
/// TUI. `path` is the resolved local filesystem root holding skill directories.
#[derive(Clone, Debug)]
pub struct RepoSource {
    pub id: String,
    pub name: Option<String>,
    pub path: PathBuf,
}

impl RepoSource {
    /// Build a repo source from a positional index and optional name/path.
    pub fn from_index(index: usize, name: Option<String>, path: PathBuf) -> Self {
        Self {
            id: name.clone().unwrap_or_else(|| index.to_string()),
            name,
            path,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkillsPaths {
    pub root: PathBuf,
    pub config_path: PathBuf,
    /// Resolved repository sources. At least one entry is always present;
    /// `root` mirrors the first entry's path for backward compatibility with
    /// single-source callers.
    pub repos: Vec<RepoSource>,
}

#[derive(Clone, Debug)]
pub struct SkillCandidate {
    pub skill_id: String,
    pub skill_dir: PathBuf,
    /// Which repo source this skill was discovered under (None when the
    /// discovery path was unaware of repo metadata, e.g. single-source scan).
    pub repo_id: Option<String>,
    /// Human-readable repo label for TUI display (None when single-source).
    pub repo_name: Option<String>,
}

impl Default for SkillCandidate {
    fn default() -> Self {
        Self {
            skill_id: String::new(),
            skill_dir: PathBuf::new(),
            repo_id: None,
            repo_name: None,
        }
    }
}
