use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MainSpecDoc {
    pub kind: String,
    pub name: String,
    pub purpose: String,
    /// Validation scope (formerly the YAML frontmatter `valid_scope`). Drives the
    /// staleness check. Required and non-empty for main specs. `valid_commands` and
    /// `evidence` were dropped — only `valid_scope` is functionally consumed.
    #[serde(default)]
    pub valid_scope: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<RequirementEntry>,
    #[serde(default)]
    pub scenarios: Vec<ScenarioEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequirementEntry {
    pub req_id: String,
    pub title: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioEntry {
    pub req_id: String,
    pub id: String,
    pub given: String,
    #[serde(rename = "when")]
    pub when_: String,
    #[serde(rename = "then")]
    pub then_: String,
    /// When `true` (default), the scenario is treated as executable for Partitioned
    /// morphology / dual-write checks. `feature: false` keeps the scenario in
    /// the constraints layer only (non-executable documentation in toon).
    #[serde(default = "default_feature_true")]
    pub feature: bool,
}

fn default_feature_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeltaSpecDoc {
    pub kind: String,
    pub ops: Vec<DeltaOpEntry>,
    pub op_scenarios: Vec<ScenarioEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeltaOpEntry {
    pub op: String,
    pub req_id: String,
    pub title: Option<String>,
    pub statement: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub name: Option<String>,
}
