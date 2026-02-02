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

#[derive(Clone, Debug)]
pub struct SkillsPaths {
    pub root: PathBuf,
    pub registry_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct SkillCandidate {
    pub skill_id: String,
    pub skill_dir: PathBuf,
}
