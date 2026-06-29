//! PageIndex tree index: maps the already-structured sdd spec IR into a
//! hierarchical tree that an agentic chat model can navigate via tool calls.
//!
//! Unlike the original PageIndex (which spends most of its effort having an LLM
//! extract a TOC from PDFs), sdd specs are *already* a tree
//! (`spec → requirement`), so building the index is a pure, LLM-free transform.

use crate::sdd::spec::ir::MainSpecDoc;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One requirement node (leaf of the tree).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReqNode {
    pub req_id: String,
    pub title: String,
    /// Full requirement text (contains MUST/SHALL).
    pub statement: String,
}

/// One spec document node (root of a subtree).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocNode {
    pub spec_id: String,
    /// Spec overview / purpose.
    pub purpose: String,
    pub reqs: Vec<ReqNode>,
}

/// Serialized pageindex tree index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeIndex {
    pub version: u32,
    /// Hash of the source specs for freshness checks.
    pub spec_hash: String,
    pub build_timestamp: String,
    /// Chat model recorded at build time (informational only; building is LLM-free).
    pub chat_model: String,
    pub docs: Vec<DocNode>,
}

const TREE_VERSION: u32 = 1;

/// Build the tree documents from parsed spec IR.
///
/// Takes `(spec_id, MainSpecDoc)` pairs (spec_id is the spec directory name, kept
/// consistent with the old spec_id scheme so retrieval IDs are comparable
/// across backends). The parser-level `Spec` type is intentionally avoided: it
/// drops `req_id`/`title` during conversion, which the tree needs.
pub fn build_docs(parsed: &[(String, MainSpecDoc)]) -> Vec<DocNode> {
    let mut docs: Vec<DocNode> = parsed
        .iter()
        .map(|(spec_id, doc)| DocNode {
            spec_id: spec_id.clone(),
            purpose: doc.purpose.clone(),
            reqs: doc
                .requirements
                .iter()
                .map(|r| ReqNode {
                    req_id: r.req_id.clone(),
                    title: r.title.clone(),
                    statement: r.statement.clone(),
                })
                .collect(),
        })
        .collect();
    // Deterministic ordering independent of directory iteration order.
    docs.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));
    docs
}

impl TreeIndex {
    /// Assemble a new tree index from built docs plus build metadata.
    pub fn new(
        docs: Vec<DocNode>,
        spec_hash: String,
        build_timestamp: String,
        chat_model: String,
    ) -> Self {
        Self {
            version: TREE_VERSION,
            spec_hash,
            build_timestamp,
            chat_model,
            docs,
        }
    }

    /// Serialize to `<dir>/tree.json`.
    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create pageindex dir {}", dir.display()))?;
        let json = serde_json::to_string_pretty(self).context("Failed to serialize tree.json")?;
        std::fs::write(dir.join("tree.json"), json)
            .with_context(|| format!("Failed to write {}/tree.json", dir.display()))?;
        Ok(())
    }

    /// Deserialize from `<dir>/tree.json`.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("tree.json");
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let idx: TreeIndex = serde_json::from_str(&content).context("Failed to parse tree.json")?;
        Ok(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry};

    fn sample_doc(name: &str, purpose: &str, reqs: &[(&str, &str, &str)]) -> (String, MainSpecDoc) {
        let requirements = reqs
            .iter()
            .map(|(rid, title, stmt)| RequirementEntry {
                req_id: rid.to_string(),
                title: title.to_string(),
                statement: stmt.to_string(),
            })
            .collect();
        (
            name.to_string(),
            MainSpecDoc {
                kind: "llman.sdd.spec".to_string(),
                name: name.to_string(),
                purpose: purpose.to_string(),
                valid_scope: Vec::new(),
                requirements,
                scenarios: Vec::new(),
                feature_refs: None,
            },
        )
    }

    #[test]
    fn test_build_docs_preserves_structure_and_ids() {
        let parsed = vec![
            sample_doc(
                "sdd-workflow",
                "Define the SDD workflow.",
                &[
                    ("r1", "Init scaffold", "`llman sdd init` MUST create dirs."),
                    ("r2", "Update", "`llman sdd update` MUST refresh AGENTS.md."),
                ],
            ),
            sample_doc(
                "cli",
                "CLI surface.",
                &[("r1", "Commands", "MUST expose subcommands.")],
            ),
        ];

        let docs = build_docs(&parsed);

        // Deterministic ordering by spec_id.
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].spec_id, "cli");
        assert_eq!(docs[1].spec_id, "sdd-workflow");

        let wf = &docs[1];
        assert_eq!(wf.purpose, "Define the SDD workflow.");
        assert_eq!(wf.reqs.len(), 2);
        assert_eq!(wf.reqs[0].req_id, "r1");
        assert_eq!(wf.reqs[0].title, "Init scaffold");
        assert!(wf.reqs[0].statement.contains("MUST"));
    }

    #[test]
    fn test_tree_index_save_load_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let parsed = vec![sample_doc(
            "demo",
            "demo purpose",
            &[("r1", "title", "stmt MUST x")],
        )];
        let docs = build_docs(&parsed);
        let tree = TreeIndex::new(
            docs,
            "deadbeef".to_string(),
            "2026-06-28T00:00:00Z".to_string(),
            "chat-model-x".to_string(),
        );
        tree.save(tmp.path()).unwrap();

        // tree.json was created where freshness expects it.
        assert!(tmp.path().join("tree.json").exists());

        let loaded = TreeIndex::load(tmp.path()).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.spec_hash, "deadbeef");
        assert_eq!(loaded.chat_model, "chat-model-x");
        assert_eq!(loaded.docs.len(), 1);
        assert_eq!(loaded.docs[0].spec_id, "demo");
        assert_eq!(loaded.docs[0].reqs[0].req_id, "r1");
    }
}
