use crate::config_schema::ConfigSchemaKind;
use crate::config_schema::{LLMANSPEC_SCHEMA_URL, prepend_schema_header, validate_yaml_value};
use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::LLMANSPEC_CONFIG_FILE;
use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_SCHEMA: &str = "spec-driven";

pub(crate) const OPTIONAL_SKILL_NAMES: &[&str] = &[
    "llman-sdd-new-change",
    "llman-sdd-continue",
    "llman-sdd-ff",
    "llman-sdd-sync",
    "llman-sdd-validate",
    "llman-sdd-arch-review",
    "llman-sdd-wayfinder",
    "llman-sdd-research",
];

const DEFAULT_CONFIG_EN: &str = r#"schema: spec-driven
locale: en

# Optional extra skills (disabled by default, uncomment to enable).
# On `llman sdd init --update` / `update-skills`, managed candidates are:
#   default workflow skills + entries listed here (extra_skills extend).
# Then `.agents/skills/llman-sdd-*` is scanned: anything not in that candidate
# set is removed first, then candidates are written/updated. Only the
# `llman-sdd-` prefix is touched (custom skills without that prefix are kept).
# Deprecated shipped skills (e.g. removed from defaults) are cleaned this way.
# extra_skills:
#   - llman-sdd-new-change
#   - llman-sdd-continue
#   - llman-sdd-ff
#   - llman-sdd-sync
#   - llman-sdd-validate
#   - llman-sdd-arch-review
#   - llman-sdd-wayfinder
#   - llman-sdd-research

# BDD integration (optional, uncomment to enable)
# bdd:
#   framework: pytest-bdd
#   feature_dir: tests/features/
#   # default_language: en
#   # Filtered runners: include {feature_*} so validate --all/--specs runs per capability.
#   # run_command: "pytest {feature_dir} -k {feature_name} -v"
#   # Project-wide runners (no placeholders): validate --all/--specs runs the suite once (batch-once).
#   # run_command: "cargo test --features bdd"
#   # verify_prompt: |
#   #   Map test failures to requirement IDs.
"#;

const DEFAULT_CONFIG_ZH_HANS: &str = r#"schema: spec-driven
locale: zh-Hans

# 可选额外技能（默认禁用，取消注释以启用）。
# 运行 `llman sdd init --update` / `update-skills` 时，管理候选集为：
#   默认 workflow 技能 + 本列表（extra_skills 扩展）。
# 然后扫描 `.agents/skills/llman-sdd-*`：不在候选集中的先删除，再写入/更新候选。
# 仅处理 `llman-sdd-` 前缀（无此前缀的自定义技能不会被删）。
# 已废弃的内置技能（从默认集移除后）会因此被正确清理。
# extra_skills:
#   - llman-sdd-new-change
#   - llman-sdd-continue
#   - llman-sdd-ff
#   - llman-sdd-sync
#   - llman-sdd-validate
#   - llman-sdd-arch-review
#   - llman-sdd-wayfinder
#   - llman-sdd-research

# BDD 集成（可选，取消注释以启用）
# bdd:
#   framework: pytest-bdd
#   feature_dir: tests/features/
#   # default_language: zh-CN
#   # 过滤型 runner：写 {feature_*}，validate --all/--specs 按 capability 分别执行。
#   # run_command: "pytest {feature_dir} -k {feature_name} -v"
#   # 项目级 runner（无占位符）：validate --all/--specs 整批只跑一次（batch-once）。
#   # run_command: "cargo test --features bdd"
#   # verify_prompt: |
#   #   将测试失败映射到对应的 requirement ID。
"#;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[schemars(description = "Archive behaviour configuration.")]
pub struct ArchiveConfig {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "When true, unchecked tasks without a defer link are errors (not just warnings). Default: false (transition period)."
    )]
    pub strict_defer: Option<bool>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Minimum task completion ratio (0.0–1.0) required for archiving. Default: none (disabled)."
    )]
    pub min_completion_ratio: Option<f64>,
}

impl ArchiveConfig {
    pub fn strict_defer(&self) -> bool {
        self.strict_defer.unwrap_or(false)
    }

    pub fn min_completion_ratio(&self) -> Option<f64> {
        self.min_completion_ratio
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[schemars(
    description = "BDD integration settings. When defined, enables feature-as-spec mode (directory-based .feature validation) and BDD-aware verify prompts."
)]
pub struct BddConfig {
    /// BDD framework identifier (optional, only used to derive a default run_command
    /// when run_command is not set: pytest-bdd, rstest-bdd, cucumber-js, behave).
    #[serde(default)]
    #[schemars(
        description = "BDD framework identifier (optional). Only used to derive a default run_command when run_command is unset."
    )]
    pub framework: String,

    /// Root directory for .feature files, relative to project root
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Root directory for .feature files, relative to project root.")]
    pub feature_dir: Option<String>,

    /// Gherkin parsing language (default: "en")
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Gherkin parsing language code (e.g. 'en', 'zh-CN'). Default: 'en'.")]
    pub default_language: Option<String>,

    /// Custom test run command with placeholders: {feature_dir}, {feature_name}, {feature_path}
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Custom test run command. Placeholders: {feature_dir}, {feature_name}, {feature_path}. Without placeholders, validate --all/--specs runs the command at most once per batch (batch-once)."
    )]
    pub run_command: Option<String>,

    /// Extra prompt injected during verify phase
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Extra prompt text injected during verify phase.")]
    pub verify_prompt: Option<String>,
}

impl BddConfig {
    pub fn effective_run_command(&self) -> String {
        if let Some(cmd) = &self.run_command {
            return cmd.clone();
        }
        match self.framework.as_str() {
            "pytest-bdd" => "pytest {feature_dir} -k {feature_name} -v".into(),
            "rstest-bdd" => "cargo test --features bdd".into(),
            "cucumber-js" => "npx cucumber-js {feature_path}".into(),
            "behave" => "behave {feature_path}".into(),
            _ => "echo 'No run_command configured. Set bdd.run_command in config.yaml'".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "SDD project configuration for llmanspec.")]
pub struct SddConfig {
    #[schemars(description = "Schema identifier. Must be \"spec-driven\".")]
    pub schema: String,

    #[serde(default = "default_locale")]
    #[schemars(description = "Locale used for SDD templates and skills.")]
    pub locale: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Additional optional SDD skills to enable (extend candidates). \
        On init --update / update-skills, candidates = default workflow skills + this list; \
        then `.agents/skills/llman-sdd-*` not in candidates are removed before rewrite. \
        Valid values: llman-sdd-new-change, llman-sdd-continue, llman-sdd-ff, \
        llman-sdd-sync, llman-sdd-validate, llman-sdd-arch-review, \
        llman-sdd-wayfinder, llman-sdd-research."
    )]
    pub extra_skills: Option<Vec<String>>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Archive behaviour settings (defer tracking, completion gates).")]
    pub archive: Option<ArchiveConfig>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "BDD integration settings. When defined, enables feature-as-spec mode (directory-based .feature validation) and BDD-aware verify prompts."
    )]
    pub bdd: Option<BddConfig>,
}

impl Default for SddConfig {
    fn default() -> Self {
        Self {
            schema: EXPECTED_SCHEMA.to_string(),
            locale: default_locale(),
            extra_skills: None,
            archive: None,
            bdd: None,
        }
    }
}

impl SddConfig {
    pub fn archive_config(&self) -> ArchiveConfig {
        self.archive.clone().unwrap_or_default()
    }
}

fn default_locale() -> String {
    "en".to_string()
}

pub fn config_path(llmanspec_dir: &Path) -> PathBuf {
    llmanspec_dir.join(LLMANSPEC_CONFIG_FILE)
}

pub fn config_with_locale(locale: Option<&str>) -> SddConfig {
    let mut config = SddConfig::default();
    if let Some(locale) = locale {
        config.locale = normalize_locale(locale);
    }
    config
}

pub fn load_config(llmanspec_dir: &Path) -> Result<Option<SddConfig>> {
    let path = config_path(llmanspec_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| anyhow!(t!("sdd.config.read_failed", error = err)))?;
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|err| anyhow!(t!("sdd.config.parse_failed", error = err)))?;

    // Reject old-format configs
    reject_old_format(&yaml_value, &path)?;

    if let Err(error) = validate_yaml_value(ConfigSchemaKind::Llmanspec, &yaml_value) {
        return Err(anyhow!(t!(
            "sdd.config.schema_invalid",
            path = path.display(),
            error = error
        )));
    }
    let config: SddConfig = serde_yaml::from_value(yaml_value)
        .map_err(|err| anyhow!(t!("sdd.config.parse_failed", error = err)))?;

    if config.schema.trim() != EXPECTED_SCHEMA {
        return Err(anyhow!(
            "Unsupported schema '{}'. Expected '{}'.",
            config.schema.trim(),
            EXPECTED_SCHEMA
        ));
    }

    if let Some(ref extra) = config.extra_skills {
        let valid: HashSet<&str> = OPTIONAL_SKILL_NAMES.iter().copied().collect();
        for name in extra {
            if !valid.contains(name.as_str()) {
                return Err(anyhow!(
                    "Unknown extra_skills entry '{}'. Valid options: {}",
                    name,
                    OPTIONAL_SKILL_NAMES.join(", ")
                ));
            }
        }
    }

    Ok(Some(SddConfig {
        schema: EXPECTED_SCHEMA.to_string(),
        locale: normalize_locale(&config.locale),
        extra_skills: config.extra_skills,
        archive: config.archive,
        bdd: config.bdd,
    }))
}

fn reject_old_format(value: &serde_yaml::Value, path: &Path) -> Result<()> {
    let Some(mapping) = value.as_mapping() else {
        return Ok(());
    };
    let has_old_keys = mapping
        .keys()
        .any(|k| k.as_str() == Some("spec_style") || k.as_str() == Some("version"));
    if has_old_keys {
        return Err(anyhow!(
            "Old config format detected in {}. Please run `llman sdd init` to reinitialize.",
            path.display()
        ));
    }
    Ok(())
}

pub fn load_required_config(llmanspec_dir: &Path) -> Result<SddConfig> {
    load_config(llmanspec_dir)?.ok_or_else(|| {
        let path = config_path(llmanspec_dir);
        anyhow!(t!("sdd.config.missing", path = path.display()))
    })
}

pub fn write_default_config(llmanspec_dir: &Path, locale: &str) -> Result<()> {
    let path = config_path(llmanspec_dir);
    let raw = match normalize_locale(locale).as_str() {
        "zh-Hans" => DEFAULT_CONFIG_ZH_HANS,
        _ => DEFAULT_CONFIG_EN,
    };
    let content = prepend_schema_header(raw, LLMANSPEC_SCHEMA_URL);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write_with_mode(&path, content.as_bytes(), None)?;
    Ok(())
}

pub fn load_or_create_config(llmanspec_dir: &Path) -> Result<SddConfig> {
    match load_config(llmanspec_dir)? {
        Some(config) => Ok(config),
        None => {
            let config = SddConfig::default();
            write_default_config(llmanspec_dir, &config.locale)?;
            Ok(config)
        }
    }
}

pub fn write_config(llmanspec_dir: &Path, config: &SddConfig) -> Result<()> {
    let path = config_path(llmanspec_dir);
    let content = serde_yaml::to_string(config)
        .map_err(|err| anyhow!(t!("sdd.config.serialize_failed", error = err)))?;
    let content = prepend_schema_header(&content, LLMANSPEC_SCHEMA_URL);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write_with_mode(&path, content.as_bytes(), None)
        .map_err(|err| anyhow!(t!("sdd.config.write_failed", error = err)))?;
    Ok(())
}

pub fn normalize_locale(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return default_locale();
    }
    let lower = trimmed.to_lowercase();
    if lower == "zh" || lower.starts_with("zh-hans") || lower == "zh-cn" {
        return "zh-Hans".to_string();
    }
    if lower.starts_with("en") {
        return "en".to_string();
    }
    trimmed.to_string()
}

pub fn locale_fallbacks(locale: &str) -> Vec<String> {
    let normalized = normalize_locale(locale);
    let mut seen = HashSet::new();
    let mut locales = Vec::new();

    push_unique(&mut locales, &mut seen, normalized.clone());
    if let Some((lang, _)) = normalized.split_once('-') {
        push_unique(&mut locales, &mut seen, lang.to_string());
    }
    push_unique(&mut locales, &mut seen, "en".to_string());

    locales
}

fn push_unique(locales: &mut Vec<String>, seen: &mut HashSet<String>, value: String) {
    if seen.insert(value.clone()) {
        locales.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_locale_defaults_to_en() {
        assert_eq!(normalize_locale(""), "en");
        assert_eq!(normalize_locale("en-US"), "en");
        assert_eq!(normalize_locale("zh"), "zh-Hans");
        assert_eq!(normalize_locale("zh-cn"), "zh-Hans");
    }

    #[test]
    fn locale_fallbacks_include_en() {
        let fallbacks = locale_fallbacks("zh-Hans");
        assert_eq!(fallbacks, vec!["zh-Hans", "zh", "en"]);
        let fallbacks = locale_fallbacks("en");
        assert_eq!(fallbacks, vec!["en"]);
    }

    #[test]
    fn load_config_normalizes_locale() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "schema: spec-driven\nlocale: zh-cn\n";
        fs::write(&path, content).expect("write config");
        let config = load_config(llmanspec_dir).expect("load").expect("config");
        assert_eq!(config.locale, "zh-Hans");
    }

    #[test]
    fn load_config_rejects_old_format() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "version: 1\nspec_style: ison\nlocale: en\n";
        fs::write(&path, content).expect("write config");
        let result = load_config(llmanspec_dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Old config format detected"), "got: {err}");
    }

    #[test]
    fn load_config_rejects_wrong_schema() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "schema: other\nlocale: en\n";
        fs::write(&path, content).expect("write config");
        let result = load_config(llmanspec_dir);
        assert!(result.is_err());
    }

    #[test]
    fn default_config_has_spec_driven() {
        let config = SddConfig::default();
        assert_eq!(config.schema, "spec-driven");
        assert_eq!(config.locale, "en");
        assert!(config.extra_skills.is_none());
    }

    #[test]
    fn write_default_config_writes_en_template() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        write_default_config(llmanspec_dir, "en").expect("write default config");
        let path = config_path(llmanspec_dir);
        assert!(path.exists());
        let content = fs::read_to_string(&path).expect("read config");
        assert!(
            content.contains("yaml-language-server"),
            "should have schema header"
        );
        assert!(content.contains("schema: spec-driven"));
        assert!(content.contains("locale: en"));
        assert!(
            content.contains("# extra_skills:"),
            "en template should have extra_skills comment"
        );
        assert!(
            content.contains("llman-sdd-*"),
            "en template should document managed prefix cleanup"
        );
    }

    #[test]
    fn write_default_config_writes_zh_hans_template() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        write_default_config(llmanspec_dir, "zh-Hans").expect("write default config");
        let path = config_path(llmanspec_dir);
        assert!(path.exists());
        let content = fs::read_to_string(&path).expect("read config");
        assert!(content.contains("yaml-language-server"));
        assert!(content.contains("schema: spec-driven"));
        assert!(content.contains("locale: zh-Hans"));
        assert!(
            content.contains("# extra_skills:"),
            "zh-Hans template should have extra_skills comment"
        );
        assert!(
            content.contains("llman-sdd-*"),
            "zh-Hans template should document managed prefix cleanup"
        );
    }

    #[test]
    fn write_default_config_round_trip_parses_correctly() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        write_default_config(llmanspec_dir, "en").expect("write default config");
        let config = load_config(llmanspec_dir)
            .expect("load should succeed")
            .expect("config should exist");
        assert_eq!(config.schema, "spec-driven");
        assert_eq!(config.locale, "en");
        // Comments are stripped by serde
        assert!(config.extra_skills.is_none());
    }

    #[test]
    fn write_config_still_outputs_compact_yaml() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let config = SddConfig::default();
        write_config(llmanspec_dir, &config).expect("write config");
        let path = config_path(llmanspec_dir);
        let content = fs::read_to_string(&path).expect("read config");
        // write_config uses serde — no comments, just compact YAML
        assert!(
            !content.contains("# Project context"),
            "serde output should not have comments"
        );
        assert!(content.contains("schema: spec-driven"));
    }

    #[test]
    fn load_config_rejects_unknown_extra_skills() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "schema: spec-driven\nlocale: en\nextra_skills:\n  - nonexistent-skill\n";
        fs::write(&path, content).expect("write config");
        let result = load_config(llmanspec_dir);
        assert!(result.is_err(), "expected error for unknown extra_skills");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown extra_skills entry"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_config_accepts_valid_extra_skills() {
        let dir = tempdir().expect("tempdir");
        let llmanspec_dir = dir.path();
        let path = config_path(llmanspec_dir);
        let content = "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-sync\n  - llman-sdd-new-change\n";
        fs::write(&path, content).expect("write config");
        let config = load_config(llmanspec_dir).expect("load").expect("config");
        assert_eq!(
            config.extra_skills,
            Some(vec![
                "llman-sdd-sync".to_string(),
                "llman-sdd-new-change".to_string(),
            ])
        );
    }
}
