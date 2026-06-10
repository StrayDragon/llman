use super::config::{OPTIONAL_SKILL_NAMES, SddConfig, load_or_create_config};
use super::templates::skill_templates;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

// ---------------------------------------------------------------------------
// Feature registry
//
// When adding a new SDD feature (config field, skill, workflow), add an entry
// here so `llman sdd upgrade-guide` can detect and suggest it.
//
// RULE: See llmanspec/specs/upgrade-guide/spec.md — every new SDD feature
//       MUST register here before merge.
// ---------------------------------------------------------------------------

struct FeatureDef {
    name: &'static str,
    description: &'static str,
    scan: fn(&SddConfig, &Path) -> FeatureResult,
}

struct FeatureResult {
    status: &'static str,
    suggestion: String,
}

/// Canonical registry of all upgradeable SDD features.
///
/// Order matters: features are presented to the agent in this order.
const FEATURES: &[FeatureDef] = &[
    FeatureDef {
        name: "extra_skills",
        description: "Optional SDD workflow skills",
        scan: scan_extra_skills,
    },
    FeatureDef {
        name: "archive",
        description: "Archive behavior settings",
        scan: scan_archive_config,
    },
    FeatureDef {
        name: "bdd",
        description: "BDD integration",
        scan: scan_bdd_config,
    },
    FeatureDef {
        name: "templates",
        description: "Skill template freshness",
        scan: scan_template_freshness,
    },
];

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------

pub fn run() -> Result<()> {
    let root = Path::new(".");
    let llmanspec_path = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        let cmd = "llman sdd init";
        return Err(anyhow::anyhow!(t!(
            "sdd.update_skills.no_llmanspec",
            cmd = cmd
        )));
    }

    let config = load_or_create_config(&llmanspec_path)?;
    let toon = build_toon(&config, root);
    println!("{toon}");
    Ok(())
}

// ---------------------------------------------------------------------------
// TOON output builder
// ---------------------------------------------------------------------------

fn build_toon(config: &SddConfig, root: &Path) -> String {
    let mut names = Vec::new();
    let mut statuses = Vec::new();
    let mut descriptions = Vec::new();
    let mut suggestions = Vec::new();

    for def in FEATURES {
        let result = (def.scan)(config, root);
        names.push(def.name);
        statuses.push(result.status);
        descriptions.push(def.description);
        if !result.suggestion.is_empty() {
            suggestions.push((def.name, result.suggestion));
        }
    }

    let n = names.len();
    let mut out = String::new();

    // Header
    out.push_str("kind: llman.sdd.upgrade_guide\n");
    out.push_str(&format!("locale: {}\n", config.locale));

    // Features table
    out.push_str(&format!("features[{n}]{{name,status,description}}:\n"));
    for i in 0..n {
        out.push_str(&format!(
            "  {},{},{}\n",
            names[i], statuses[i], descriptions[i]
        ));
    }

    // Suggestions block (only actionable items)
    if !suggestions.is_empty() {
        out.push_str(&format!(
            "suggestions[{}]{{name,yaml}}:\n",
            suggestions.len()
        ));
        for (name, yaml) in &suggestions {
            // Encode multi-line YAML as single-line with \n escapes.
            // Agents parse this structurally; the escaped newlines are
            // restored when applying the suggestion.
            let escaped = yaml.replace('\n', "\\n");
            // Quote values that contain commas (TOON tabular quoting rule)
            if escaped.contains(',') {
                out.push_str(&format!("  {name},\"{escaped}\"\n"));
            } else {
                out.push_str(&format!("  {name},{escaped}\n"));
            }
        }
    }

    // Footer hint
    out.push_str(
        "---\nTo apply: merge suggestions into llmanspec/config.yaml, then run `llman sdd init --update`.",
    );

    out
}

// ---------------------------------------------------------------------------
// Per-feature scanners
// ---------------------------------------------------------------------------

fn scan_extra_skills(config: &SddConfig, _root: &Path) -> FeatureResult {
    let enabled: HashSet<&str> = config
        .extra_skills
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let available: Vec<&str> = OPTIONAL_SKILL_NAMES
        .iter()
        .filter(|name| !enabled.contains(**name))
        .copied()
        .collect();

    let status = if available.is_empty() {
        "enabled"
    } else if enabled.is_empty() {
        "disabled"
    } else {
        "partial"
    };

    let suggestion = if available.is_empty() {
        String::new()
    } else {
        format!(
            "extra_skills:\n{}",
            available
                .iter()
                .map(|s| format!("  - {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    FeatureResult { status, suggestion }
}

fn scan_archive_config(config: &SddConfig, _root: &Path) -> FeatureResult {
    let archive = config.archive.as_ref();

    let has_strict = archive.and_then(|a| a.strict_defer).is_some();
    let has_ratio = archive.and_then(|a| a.min_completion_ratio).is_some();

    let status = if has_strict && has_ratio {
        "enabled"
    } else if archive.is_none() {
        "disabled"
    } else {
        "partial"
    };

    let suggestion = if has_strict && has_ratio {
        String::new()
    } else {
        let mut lines = vec!["archive:".to_string()];
        if !has_strict {
            lines.push("  # When true, unchecked tasks without a defer link are errors.".into());
            lines.push("  strict_defer: false".into());
        }
        if !has_ratio {
            lines
                .push("  # Minimum task completion ratio (0.0-1.0) required for archiving.".into());
            lines.push("  # min_completion_ratio: 0.8".into());
        }
        lines.join("\n")
    };

    FeatureResult { status, suggestion }
}

fn scan_bdd_config(config: &SddConfig, _root: &Path) -> FeatureResult {
    let status = if config.bdd.is_some() {
        "enabled"
    } else {
        "disabled"
    };

    let suggestion = if config.bdd.is_some() {
        String::new()
    } else {
        "\
bdd:
  # Supported: pytest-bdd, rstest-bdd, cucumber-js, behave, custom
  framework: pytest-bdd
  feature_dir: tests/features/
  # default_language: en
  # run_command: \"pytest {feature_dir} -k {feature_name} -v\"
  # verify_prompt: |
  #   Map test failures to requirement IDs."
            .into()
    };

    FeatureResult { status, suggestion }
}

fn scan_template_freshness(config: &SddConfig, root: &Path) -> FeatureResult {
    let templates = skill_templates(config, root);
    let installed_dir = root.join(".agents").join("skills");

    let mut outdated = Vec::new();

    if let Ok(templates) = templates {
        for template in &templates {
            let skill_dir = installed_dir.join(template.name.trim_end_matches(".md"));
            let skill_path = skill_dir.join("SKILL.md");
            let stale = if skill_path.exists() {
                match std::fs::read_to_string(&skill_path) {
                    Ok(installed) => installed.trim() != template.content.trim(),
                    Err(_) => true,
                }
            } else {
                true
            };
            if stale {
                outdated.push(template.name);
            }
        }
    }

    let status = if outdated.is_empty() {
        "enabled"
    } else {
        "partial"
    };

    let suggestion = if outdated.is_empty() {
        String::new()
    } else {
        format!(
            "Run `llman sdd init --update` to refresh:\\n{}",
            outdated
                .iter()
                .map(|s| format!("  - {s}"))
                .collect::<Vec<_>>()
                .join("\\n")
        )
    };

    FeatureResult { status, suggestion }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn toon_output_has_kind_and_features() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");
        let config = SddConfig::default();
        let toon = build_toon(&config, root);
        assert!(toon.contains("kind: llman.sdd.upgrade_guide"));
        assert!(toon.contains("features[4]{name,status,description}:"));
        assert!(toon.contains("extra_skills,disabled,"));
        assert!(toon.contains("suggestions["));
    }

    #[test]
    fn scan_extra_skills_all_disabled() {
        let config = SddConfig::default();
        let result = scan_extra_skills(&config, Path::new("."));
        assert_eq!(result.status, "disabled");
        assert!(result.suggestion.contains("extra_skills:"));
        for name in OPTIONAL_SKILL_NAMES {
            assert!(result.suggestion.contains(name), "missing {name}");
        }
    }

    #[test]
    fn scan_extra_skills_all_enabled() {
        let mut config = SddConfig::default();
        config.extra_skills = Some(OPTIONAL_SKILL_NAMES.iter().map(|s| s.to_string()).collect());
        let result = scan_extra_skills(&config, Path::new("."));
        assert_eq!(result.status, "enabled");
        assert!(result.suggestion.is_empty());
    }

    #[test]
    fn scan_extra_skills_partial() {
        let mut config = SddConfig::default();
        config.extra_skills = Some(vec!["llman-sdd-verify".to_string()]);
        let result = scan_extra_skills(&config, Path::new("."));
        assert_eq!(result.status, "partial");
        // Enabled skill should NOT appear in suggestions
        assert!(!result.suggestion.contains("- llman-sdd-verify\n"));
        // Available skills should appear
        assert!(result.suggestion.contains("- llman-sdd-new-change"));
    }

    #[test]
    fn scan_bdd_disabled() {
        let config = SddConfig::default();
        let result = scan_bdd_config(&config, Path::new("."));
        assert_eq!(result.status, "disabled");
        assert!(result.suggestion.contains("framework:"));
    }

    #[test]
    fn scan_archive_disabled() {
        let config = SddConfig::default();
        let result = scan_archive_config(&config, Path::new("."));
        assert_eq!(result.status, "disabled");
        assert!(result.suggestion.contains("strict_defer"));
    }

    #[test]
    fn scan_template_freshness_with_no_installed() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");
        let config = SddConfig::default();
        let result = scan_template_freshness(&config, root);
        assert_eq!(result.status, "partial");
        assert!(!result.suggestion.is_empty());
    }
}
