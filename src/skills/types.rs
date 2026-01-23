use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ConfigEntry {
    pub id: String,
    pub agent: String,
    pub scope: String,
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct SkillsConfig {
    pub sources: Vec<ConfigEntry>,
    pub targets: Vec<ConfigEntry>,
    pub repo_root: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct SkillsPaths {
    pub root: PathBuf,
    pub store_dir: PathBuf,
    pub registry_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct SkillCandidate {
    pub skill_id: String,
    pub hash: String,
    pub source_id: String,
    pub source_path: PathBuf,
    pub skill_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ConflictOption {
    pub hash: String,
    pub source_id: String,
    pub source_path: PathBuf,
    pub skill_dir: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct SyncSummary {
    pub scanned_sources: usize,
    pub discovered_skills: usize,
    pub imported_versions: usize,
    pub linked_sources: usize,
    pub conflicts: usize,
}
