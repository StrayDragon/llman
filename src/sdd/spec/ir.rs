use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MainSpecDoc {
    pub kind: String,
    pub name: String,
    pub purpose: String,
    pub requirements: Vec<RequirementEntry>,
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
