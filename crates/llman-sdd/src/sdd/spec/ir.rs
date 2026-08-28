use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct MainSpecDoc {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) purpose: String,
    /// Validation scope (formerly the YAML frontmatter `valid_scope`). Drives the
    /// staleness check. Required and non-empty for main specs. `valid_commands` and
    /// `evidence` were dropped — only `valid_scope` is functionally consumed.
    #[serde(default)]
    pub(crate) valid_scope: Vec<String>,
    #[serde(default)]
    pub(crate) requirements: Vec<RequirementEntry>,
    #[serde(default)]
    pub(crate) scenarios: Vec<ScenarioEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequirementEntry {
    pub(crate) req_id: String,
    pub(crate) title: String,
    pub(crate) statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScenarioEntry {
    pub(crate) req_id: String,
    pub(crate) id: String,
    pub(crate) given: String,
    #[serde(rename = "when")]
    pub(crate) when_: String,
    #[serde(rename = "then")]
    pub(crate) then_: String,
    /// When `true` (default), the scenario is treated as executable for Partitioned
    /// morphology / dual-write checks. `feature: false` keeps the scenario in
    /// the constraints layer only (non-executable documentation in toon).
    #[serde(default = "default_feature_true")]
    pub(crate) feature: bool,
}

fn default_feature_true() -> bool {
    true
}
