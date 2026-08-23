//! Declarable harness-binding sources (`bdd.bindings`, sdd-workflow r2).
//!
//! A "binding" is consumer-side evidence that a `.feature` scenario is
//! actually wired into a test (llman's own `scenarios!("llmanspec/specs",
//! tags = "@executable")` directory discovery, or downstream per-scenario
//! `#[scenario(path = "...", name = "...")]` attributes). llman cannot know
//! this without being told, so the criterion is declared per project:
//!
//! - [`BindingSource::Tags`] — a scenario is bound when its tags contain all
//!   listed tags (`@` prefix optional).
//! - [`BindingSource::ScenarioAttrs`] — extract `path = "..."` /
//!   `name = "..."` literal pairs from `#[scenario( ... )]` attribute blocks
//!   in glob-matched Rust files; feature paths outside the specs root are
//!   ignored.
//!
//! Multiple sources union with dedup. When no sources are declared, the
//! bound/unbound split stays inactive and legacy output is preserved (r3).

use crate::sdd::project::config::BindingSource;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const SPECS_SEGMENTS: &str = "specs";

/// Resolved binding evidence, ready for per-capability bound counting.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedBindings {
    /// Required-tag sets from `tags` sources: a scenario is bound when its
    /// tag set contains ALL tags of any entry.
    tag_sets: Vec<Vec<String>>,
    /// `(capability, feature file name, scenario name)` triples from
    /// `scenario-attrs` sources whose feature path lives under the specs root.
    attr_triples: HashSet<(String, String, String)>,
    /// Attribute blocks that named no usable scenario selector (e.g.
    /// `index = N`) or had incomplete path/name literals. Recorded, not fatal.
    pub ignored_attr_blocks: usize,
}

impl ResolvedBindings {
    /// True when at least one source was declared (even if it matched nothing).
    pub fn is_declared(&self) -> bool {
        !self.tag_sets.is_empty() || !self.attr_triples.is_empty() || self.ignored_attr_blocks > 0
    }

    /// Count scenarios of one capability that are bound by any source.
    ///
    /// The count only includes scenarios that actually exist in the spec dir,
    /// so `bound <= total` and `unbound = total - bound` always holds (r3).
    pub fn count_bound(
        &self,
        capability: &str,
        features: &[(String, crate::sdd::spec::partitioned::FeatureScenario)],
    ) -> usize {
        features
            .iter()
            .filter(|(file, sc)| self.is_bound(capability, file, sc))
            .count()
    }

    /// Whether a single scenario is bound by any declared source.
    fn is_bound(
        &self,
        capability: &str,
        feature_file: &str,
        scenario: &crate::sdd::spec::partitioned::FeatureScenario,
    ) -> bool {
        if self.attr_triples.contains(&(
            capability.to_string(),
            feature_file.to_string(),
            scenario.id.clone(),
        )) {
            return true;
        }
        self.tag_sets.iter().any(|required| {
            required.iter().all(|tag| {
                scenario
                    .tags
                    .iter()
                    .any(|t| normalize_tag(t) == tag.as_str())
            })
        })
    }
}

fn normalize_tag(tag: &str) -> &str {
    tag.trim().trim_start_matches('@')
}

/// Resolve declared sources into matchable evidence. Returns `None` when no
/// bindings are configured (split stays inactive, r2/r3).
pub fn resolve_from_config(
    root: &Path,
    config: Option<&crate::sdd::project::config::SddConfig>,
) -> Result<Option<ResolvedBindings>> {
    let Some(sources) = config
        .and_then(|c| c.bdd.as_ref())
        .and_then(|b| b.bindings.as_ref())
    else {
        return Ok(None);
    };
    if sources.is_empty() {
        return Ok(None);
    }
    resolve(root, sources).map(Some)
}

pub fn resolve(root: &Path, sources: &[BindingSource]) -> Result<ResolvedBindings> {
    let mut resolved = ResolvedBindings::default();
    for source in sources {
        match source {
            BindingSource::Tags { tags } => {
                if tags.is_empty() {
                    continue;
                }
                let set: Vec<String> = tags.iter().map(|t| normalize_tag(t).to_string()).collect();
                resolved.tag_sets.push(set);
            }
            BindingSource::ScenarioAttrs { files } => {
                for pattern in files {
                    let full = root.join(pattern.trim().replace('\\', "/"));
                    let pattern_str = full.to_string_lossy().to_string();
                    let paths = glob::glob(&pattern_str)
                        .with_context(|| format!("invalid bindings glob `{pattern}`"))?;
                    for entry in paths {
                        let path = entry.with_context(|| format!("glob walk `{pattern}`"))?;
                        collect_attr_pairs(&path, &mut resolved);
                    }
                }
            }
        }
    }
    Ok(resolved)
}

/// Extract `(path = "...", name = "...")` pairs from every
/// `#[scenario( ... )]` block of one Rust source file.
fn collect_attr_pairs(path: &Path, out: &mut ResolvedBindings) {
    let Ok(content) = fs::read_to_string(path) else {
        out.ignored_attr_blocks += 1;
        return;
    };
    let block_re = regex::Regex::new(r"(?s)#\[\s*scenario\s*\((.*?)\)\s*\]").expect("static regex");
    let kv_re = regex::Regex::new(r#"(path|name|index)\s*=\s*"([^"]*)""#).expect("static regex");
    for block in block_re.captures_iter(&content) {
        let attrs = &block[1];
        let mut feature_path: Option<String> = None;
        let mut name: Option<String> = None;
        let mut has_index = false;
        for kv in kv_re.captures_iter(attrs) {
            match &kv[1] {
                "path" => feature_path = Some(kv[2].to_string()),
                "name" => name = Some(kv[2].to_string()),
                _ => has_index = true,
            }
        }
        match (feature_path, name) {
            (Some(p), Some(n)) => {
                if let Some(triple) = split_specs_path(&p) {
                    out.attr_triples.insert((triple.0, triple.1, n));
                }
                // Paths outside the specs root are ignored (r2): they belong
                // to no capability, e.g. historical tests/features/*.feature.
            }
            _ => out.ignored_attr_blocks += 1,
        }
        let _ = has_index; // index-only selectors carry no name to record
    }
}

/// Map a feature path like `llmanspec/specs/<cap>/<file>.feature` (as written
/// in the attribute literal, `/`-separated) to `(capability, file name)`.
/// Returns `None` for anything outside `<llmanspec>/specs/**`.
fn split_specs_path(raw: &str) -> Option<(String, String)> {
    let normalized = normalize_rel(&raw.trim().replace('\\', "/"));
    let prefix = format!("{LLMANSPEC_DIR_NAME}/{SPECS_SEGMENTS}/");
    let rest = normalized.strip_prefix(&prefix)?;
    let (capability, file) = rest.split_once('/')?;
    Some((capability.to_string(), file.to_string()))
}

/// Collapse `./` segments and backslashes so prefix matching is stable.
fn normalize_rel(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::spec::partitioned::FeatureScenario;

    fn scenario(id: &str, tags: &[&str]) -> FeatureScenario {
        FeatureScenario {
            id: id.to_string(),
            given: String::new(),
            when_: String::new(),
            then_: String::new(),
            req_ids: Vec::new(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdirs");
        }
        fs::write(path, content).expect("write");
    }

    #[test]
    fn tags_source_matches_scenarios_with_all_tags() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve(
            dir.path(),
            &[BindingSource::Tags {
                tags: vec!["executable".into()],
            }],
        )
        .unwrap();

        let features = vec![(
            "cap.feature".to_string(),
            scenario("bound-one", &["@executable", "@req:r1"]),
        )];
        assert_eq!(resolved.count_bound("cap", &features), 1);

        let untagged = vec![("cap.feature".to_string(), scenario("doc-only", &[]))];
        assert_eq!(resolved.count_bound("cap", &untagged), 0);
    }

    #[test]
    fn multiple_tag_sources_union_and_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve(
            dir.path(),
            &[
                BindingSource::Tags {
                    tags: vec!["a".into()],
                },
                BindingSource::Tags {
                    tags: vec!["@a".into(), "b".into()],
                },
            ],
        )
        .unwrap();
        // Scenario tagged a+b counts once even though both sources hit it.
        let features = vec![("f.feature".to_string(), scenario("x", &["a", "b"]))];
        assert_eq!(resolved.count_bound("cap", &features), 1);

        // A scenario matching only the second (two-tag) source also counts.
        let features2 = vec![("f.feature".to_string(), scenario("y", &["a", "b"]))];
        assert_eq!(resolved.count_bound("cap", &features2), 1);
    }

    #[test]
    fn attr_source_extracts_multiline_pairs_and_ignores_outside_specs() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("tests/bdd/bindings_x.rs"),
            r#"
#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "tui-registers-ask"
)]
fn t() {}

#[scenario(
    path = "tests/features/external.feature",
    name = "outside-specs"
)]
fn e() {}

#[scenario("llmanspec/specs/no-name/f.feature", index = 2)]
fn i() {}
"#,
        );
        let resolved = resolve(
            dir.path(),
            &[BindingSource::ScenarioAttrs {
                files: vec!["tests/bdd/bindings_*.rs".into()],
            }],
        )
        .unwrap();

        let features = vec![(
            "app-tui-ask.feature".to_string(),
            scenario("tui-registers-ask", &[]),
        )];
        assert_eq!(resolved.count_bound("app-tui-ask", &features), 1);
        // The external + index-only blocks are recorded but bind nothing.
        assert_eq!(resolved.ignored_attr_blocks, 1);
        assert!(resolved.attr_triples.len() == 1);
    }

    #[test]
    fn attr_source_requires_existing_scenario_names_for_invariant() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("tests/b.rs"),
            r#"#[scenario(path = "llmanspec/specs/cap/cap.feature", name = "ghost")]"#,
        );
        let resolved = resolve(
            dir.path(),
            &[BindingSource::ScenarioAttrs {
                files: vec!["tests/b.rs".into()],
            }],
        )
        .unwrap();
        // Ghost names must not inflate bound above the real scenario count.
        let features = vec![("cap.feature".to_string(), scenario("real", &[]))];
        assert_eq!(resolved.count_bound("cap", &features), 0);
    }

    #[test]
    fn glob_without_matches_is_empty_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve(
            dir.path(),
            &[BindingSource::ScenarioAttrs {
                files: vec!["tests/none/*.rs".into()],
            }],
        )
        .unwrap();
        assert!(!resolved.is_declared());
    }

    #[test]
    fn invalid_glob_pattern_fails_loudly() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve(
            dir.path(),
            &[BindingSource::ScenarioAttrs {
                files: vec!["[".into()],
            }],
        );
        assert!(result.is_err());
    }
}
