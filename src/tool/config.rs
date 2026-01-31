use crate::config_schema::{ConfigSchemaKind, validate_yaml_value};
use anyhow::{Result, anyhow};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(
    title = "llman Tool Config",
    description = "Tool configuration section for llman."
)]
pub struct Config {
    #[schemars(description = "Configuration version for tool settings.")]
    pub version: String,
    #[schemars(description = "Tool-specific configuration.")]
    pub tools: ToolsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "Configuration for individual tools.")]
pub struct ToolsConfig {
    #[serde(rename = "clean-useless-comments")]
    #[schemars(description = "Settings for the clean-useless-comments tool.")]
    pub clean_useless_comments: Option<CleanUselessCommentsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "Settings for cleaning unnecessary comments.")]
pub struct CleanUselessCommentsConfig {
    #[schemars(description = "File scope configuration.")]
    pub scope: ScopeConfig,
    #[serde(rename = "lang-rules")]
    #[schemars(description = "Language-specific rules.")]
    pub lang_rules: LanguageRules,
    #[serde(rename = "global-rules")]
    #[schemars(description = "Global rules applied across languages.")]
    pub global_rules: Option<GlobalRules>,
    #[schemars(description = "Safety controls for running the tool.")]
    pub safety: Option<SafetyConfig>,
    #[schemars(description = "Output reporting configuration.")]
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "File include/exclude patterns for tool scope.")]
pub struct ScopeConfig {
    #[serde(default = "default_include")]
    #[schemars(description = "Glob patterns to include.")]
    pub include: Vec<String>,
    #[serde(default = "default_exclude")]
    #[schemars(description = "Glob patterns to exclude.")]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "Language-specific rule sets.")]
pub struct LanguageRules {
    #[schemars(description = "Python rule overrides.")]
    pub python: Option<LanguageSpecificRules>,
    #[schemars(description = "JavaScript rule overrides.")]
    pub javascript: Option<LanguageSpecificRules>,
    #[schemars(description = "TypeScript rule overrides.")]
    pub typescript: Option<LanguageSpecificRules>,
    #[schemars(description = "Rust rule overrides.")]
    pub rust: Option<LanguageSpecificRules>,
    #[schemars(description = "Go rule overrides.")]
    pub go: Option<LanguageSpecificRules>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[schemars(description = "Rules for a specific language.")]
pub struct LanguageSpecificRules {
    #[serde(rename = "single-line-comments")]
    pub single_line_comments: Option<bool>,
    #[serde(rename = "multi-line-comments")]
    pub multi_line_comments: Option<bool>,
    #[serde(rename = "docstrings")]
    pub docstrings: Option<bool>,
    #[serde(rename = "jsdoc")]
    pub jsdoc: Option<bool>,
    #[serde(rename = "doc-comments")]
    pub doc_comments: Option<bool>,
    #[serde(rename = "godoc")]
    pub godoc: Option<bool>,
    #[serde(rename = "preserve-patterns")]
    pub preserve_patterns: Option<Vec<String>>,
    #[serde(rename = "min-comment-length")]
    pub min_comment_length: Option<usize>,
    #[serde(rename = "min-code-complexity")]
    pub min_code_complexity: Option<u32>,
    #[serde(rename = "remove-duplicate-comments")]
    pub remove_duplicate_comments: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "Global rules applied across all languages.")]
pub struct GlobalRules {
    #[serde(rename = "preserve-empty-lines")]
    pub preserve_empty_lines: Option<bool>,
    #[serde(rename = "remove-consecutive-empty-lines")]
    pub remove_consecutive_empty_lines: Option<bool>,
    #[serde(rename = "remove-duplicate-comments")]
    pub remove_duplicate_comments: Option<bool>,
    #[serde(rename = "max-comment-density")]
    pub max_comment_density: Option<f64>,
    #[serde(rename = "min-comment-length")]
    pub min_comment_length: Option<usize>,
    #[serde(rename = "min-code-complexity")]
    pub min_code_complexity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "Safety guardrails for running the tool.")]
pub struct SafetyConfig {
    #[serde(rename = "dry-run-first")]
    pub dry_run_first: Option<bool>,
    #[serde(rename = "git-aware")]
    pub git_aware: Option<bool>,
    #[serde(rename = "require-git-commit")]
    pub require_git_commit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(description = "Output and reporting configuration.")]
pub struct OutputConfig {
    #[serde(rename = "show-changed-files")]
    pub show_changed_files: Option<bool>,
    #[serde(rename = "show-removed-comments")]
    pub show_removed_comments: Option<bool>,
    #[serde(rename = "show-statistics")]
    pub show_statistics: Option<bool>,
    #[serde(rename = "generate-report")]
    pub generate_report: Option<bool>,
    #[serde(rename = "report-format")]
    pub report_format: Option<String>,
}

// Default implementations
fn default_include() -> Vec<String> {
    vec![
        "**/*.py".to_string(),
        "**/*.js".to_string(),
        "**/*.ts".to_string(),
        "**/*.rs".to_string(),
        "**/*.go".to_string(),
    ]
}

fn default_exclude() -> Vec<String> {
    vec![
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/.git/**".to_string(),
        "**/dist/**".to_string(),
        "**/build/**".to_string(),
    ]
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            include: default_include(),
            exclude: default_exclude(),
        }
    }
}

/// Get the global configuration file path
fn get_global_config_path() -> Result<PathBuf> {
    let config_dir = crate::config::resolve_config_dir(None)?;
    Ok(config_dir.join("config.yaml"))
}

fn is_project_config_path(path: &Path) -> bool {
    path.file_name() == Some(OsStr::new("config.yaml"))
        && path.parent().and_then(|parent| parent.file_name()) == Some(OsStr::new(".llman"))
}

fn schema_kind_for_path(path: &Path) -> ConfigSchemaKind {
    if is_project_config_path(path) {
        return ConfigSchemaKind::Project;
    }
    if let Ok(global) = get_global_config_path()
        && path == global
    {
        return ConfigSchemaKind::Global;
    }
    ConfigSchemaKind::Global
}

impl Config {
    /// Load configuration from the specified path
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(anyhow!(t!("tool.config.not_found", path = path.display())));
        }

        let content = fs::read_to_string(path)?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| anyhow!(t!("tool.config.parse_failed", error = e)))?;
        let schema_kind = schema_kind_for_path(path);
        if let Err(error) = validate_yaml_value(schema_kind, &yaml_value) {
            return Err(anyhow!(t!(
                "tool.config.schema_invalid",
                path = path.display(),
                error = error
            )));
        }
        let config: Config = serde_yaml::from_value(yaml_value)
            .map_err(|e| anyhow!(t!("tool.config.parse_failed", error = e)))?;

        Ok(config)
    }

    /// Load configuration with local-first priority
    /// 1. If explicit config path provided, use it
    /// 2. Try local .llman/config.yaml in current directory
    /// 3. Try global config from LLMAN_CONFIG_DIR or default location
    pub fn load_with_priority(explicit_path: Option<&Path>) -> Result<Self> {
        // If explicit path provided, use it
        if let Some(path) = explicit_path {
            return Self::load(path);
        }

        // Try local config first
        let local_config = std::env::current_dir()?.join(".llman/config.yaml");
        if local_config.exists() {
            return Self::load(local_config);
        }

        // Fall back to global config
        let global_config = get_global_config_path()?;
        if global_config.exists() {
            return Self::load(global_config);
        }

        // No config found, return error
        Err(anyhow!(t!(
            "tool.config.not_found_with_priority",
            local = ".llman/config.yaml"
        )))
    }

    /// Load configuration or return default if not found
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration with local-first priority, returning default if none found
    pub fn load_with_priority_or_default(explicit_path: Option<&Path>) -> Result<Self> {
        if let Some(path) = explicit_path {
            if path.exists() {
                return Self::load(path);
            }
            return Ok(Self::default());
        }

        let local_config = std::env::current_dir()?.join(".llman/config.yaml");
        if local_config.exists() {
            return Self::load(local_config);
        }

        let global_config = get_global_config_path()?;
        if global_config.exists() {
            return Self::load(global_config);
        }

        Ok(Self::default())
    }

    pub fn get_clean_comments_config(&self) -> Option<&CleanUselessCommentsConfig> {
        self.tools.clean_useless_comments.as_ref()
    }

    pub fn generate_schema() -> Result<String> {
        let schema = schema_for!(Config);
        serde_json::to_string_pretty(&schema)
            .map_err(|e| anyhow!(t!("tool.config.schema_generate_failed", error = e)))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1".to_string(),
            tools: ToolsConfig {
                clean_useless_comments: Some(CleanUselessCommentsConfig {
                    scope: ScopeConfig::default(),
                    lang_rules: LanguageRules {
                        python: Some(LanguageSpecificRules {
                            single_line_comments: Some(false),
                            multi_line_comments: Some(false),
                            docstrings: Some(false),
                            preserve_patterns: Some(vec![
                                r"^\s*#\s*(TODO|FIXME|NOTE|HACK):\s*.*".to_string(),
                                r"^\s*#\s*(type|param|return|raises):\s*.*".to_string(),
                                r"^\s*#\s*(Copyright|License):\s*.*".to_string(),
                            ]),
                            min_comment_length: Some(10),
                            min_code_complexity: Some(3),
                            remove_duplicate_comments: Some(true),
                            ..Default::default()
                        }),
                        javascript: Some(LanguageSpecificRules {
                            single_line_comments: Some(false),
                            multi_line_comments: Some(false),
                            jsdoc: Some(false),
                            preserve_patterns: Some(vec![
                                r"^\s*//\s*(TODO|FIXME|NOTE|HACK):\s*.*".to_string(),
                                r"^\s*/\*\*.*\*/".to_string(),
                                r"^\s*//\s*(type|param|return):\s*.*".to_string(),
                            ]),
                            min_comment_length: Some(10),
                            min_code_complexity: Some(3),
                            remove_duplicate_comments: Some(true),
                            ..Default::default()
                        }),
                        rust: Some(LanguageSpecificRules {
                            single_line_comments: Some(false),
                            multi_line_comments: Some(false),
                            doc_comments: Some(false),
                            preserve_patterns: Some(vec![
                                r"^\s*///\s*(TODO|FIXME|NOTE|HACK):\s*.*".to_string(),
                                r"^\s*//!\s*(TODO|FIXME|NOTE|HACK):\s*.*".to_string(),
                                r"^\s*///\s*(Examples|Safety|Panics):\s*.*".to_string(),
                            ]),
                            min_comment_length: Some(8),
                            min_code_complexity: Some(2),
                            remove_duplicate_comments: Some(true),
                            ..Default::default()
                        }),
                        go: Some(LanguageSpecificRules {
                            single_line_comments: Some(false),
                            multi_line_comments: Some(false),
                            godoc: Some(false),
                            preserve_patterns: Some(vec![
                                r"^\s*//\s*(TODO|FIXME|NOTE|HACK):\s*.*".to_string(),
                                r"^\s*//\s*(Package|Function|Return|Parameters):\s*.*".to_string(),
                            ]),
                            min_comment_length: Some(10),
                            min_code_complexity: Some(3),
                            remove_duplicate_comments: Some(true),
                            ..Default::default()
                        }),
                        typescript: None,
                    },
                    global_rules: Some(GlobalRules {
                        preserve_empty_lines: Some(true),
                        remove_consecutive_empty_lines: Some(true),
                        remove_duplicate_comments: Some(true),
                        max_comment_density: Some(0.3),
                        min_comment_length: Some(8),
                        min_code_complexity: Some(2),
                    }),
                    safety: Some(SafetyConfig {
                        dry_run_first: Some(true),
                        git_aware: Some(true),
                        require_git_commit: Some(true),
                    }),
                    output: Some(OutputConfig {
                        show_changed_files: Some(true),
                        show_removed_comments: Some(true),
                        show_statistics: Some(true),
                        generate_report: Some(true),
                        report_format: Some("markdown".to_string()),
                    }),
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.version, "0.1");
        assert!(config.tools.clean_useless_comments.is_some());
    }

    #[test]
    fn test_config_schema_generation() {
        let schema = Config::generate_schema();
        assert!(schema.is_ok());
    }

    #[test]
    fn test_config_from_yaml() {
        let yaml = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
      exclude:
        - "**/node_modules/**"
    lang-rules:
      python:
        single-line-comments: true
        min-comment-length: 10
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml).unwrap();
        let config = Config::load(temp_file.path());
        assert!(config.is_ok());
    }

    #[test]
    fn test_default_preserve_patterns_match_todo() {
        let config = Config::default();
        let clean_config = config.tools.clean_useless_comments.unwrap();
        let patterns = clean_config
            .lang_rules
            .python
            .unwrap()
            .preserve_patterns
            .unwrap();

        let regex = regex::Regex::new(&patterns[0]).unwrap();
        assert!(regex.is_match("# TODO: check this"));
    }
}
