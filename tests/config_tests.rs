use llman::tool::config::Config;
use serde_json::Value;
mod common;
use common::*;

/// Tests that the default configuration loads correctly with expected values
/// and includes the clean-useless-comments tool configuration.
#[test]
fn test_default_config_loading_contains_expected_values_and_tool_configuration() {
    let env = TestEnvironment::new();
    let config = Config::load_or_default(env.path().join(".llman").join("config.yaml")).unwrap();

    assert_eq!(config.version, "0.1");
    assert!(config.tools.clean_useless_comments.is_some());
}

/// Tests loading a custom configuration with all supported options including
/// language-specific rules, file scoping, safety settings, and output options.
#[test]
fn test_custom_config_loading_parses_all_supported_options_correctly() {
    let env = TestEnvironment::new();

    let config_content = format!(
        r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "{}"
        - "{}"
      exclude:
        - "**/node_modules/**"
    lang-rules:
      python:
        single-line-comments: true
        min-comment-length: {}
      javascript:
        single-line-comments: true
        min-comment-length: {}
    global-rules:
      min-comment-length: {}
    safety:
      dry-run-first: false
    output:
      show-statistics: true
"#,
        test_constants::PYTHON_FILE_PATTERN,
        test_constants::JS_FILE_PATTERN,
        test_constants::SHORT_COMMENT_LENGTH * 2, // 10
        test_constants::SHORT_COMMENT_LENGTH * 3, // 15
        test_constants::SHORT_COMMENT_LENGTH + 3  // 8
    );

    env.create_config(&config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let clean_config = config.get_clean_comments_config().unwrap();

    assert_eq!(
        clean_config.scope.include,
        vec![
            test_constants::PYTHON_FILE_PATTERN,
            test_constants::JS_FILE_PATTERN
        ]
    );
    assert_eq!(clean_config.scope.exclude, vec!["**/node_modules/**"]);

    assert_eq!(
        clean_config
            .lang_rules
            .python
            .as_ref()
            .unwrap()
            .single_line_comments,
        Some(true)
    );
    assert_eq!(
        clean_config
            .lang_rules
            .python
            .as_ref()
            .unwrap()
            .min_comment_length,
        Some((test_constants::SHORT_COMMENT_LENGTH * 2) as usize)
    );

    assert_eq!(
        clean_config
            .lang_rules
            .javascript
            .as_ref()
            .unwrap()
            .min_comment_length,
        Some((test_constants::SHORT_COMMENT_LENGTH * 3) as usize)
    );

    // Check if global_rules exists and has the right value
    assert!(
        clean_config.global_rules.is_some(),
        "global_rules should not be None"
    );
    assert_eq!(
        clean_config
            .global_rules
            .as_ref()
            .unwrap()
            .min_comment_length,
        Some((test_constants::SHORT_COMMENT_LENGTH + 3) as usize)
    );

    // Check if safety exists and has the right value
    assert!(clean_config.safety.is_some(), "safety should not be None");
    assert_eq!(
        clean_config.safety.as_ref().unwrap().dry_run_first,
        Some(false)
    );

    // Check if output exists and has the right value
    assert!(clean_config.output.is_some(), "output should not be None");
    assert_eq!(
        clean_config.output.as_ref().unwrap().show_statistics,
        Some(true)
    );
}

#[test]
fn test_config_with_missing_optional_fields() {
    let env = TestEnvironment::new();

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: true
"#;

    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let clean_config = config.get_clean_comments_config().unwrap();

    // Check defaults for missing fields
    assert_eq!(
        clean_config
            .lang_rules
            .python
            .as_ref()
            .unwrap()
            .min_comment_length,
        None
    );
    assert_eq!(clean_config.global_rules, None);
    assert_eq!(clean_config.safety, None);
    assert_eq!(clean_config.output, None);
}

#[test]
fn test_invalid_config_yaml() {
    let env = TestEnvironment::new();

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: "true"  # Should be boolean, not string
"#;

    env.create_config(config_content);

    let result = Config::load(env.path().join(".llman").join("config.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_config_schema_generation() {
    let schema = Config::generate_schema();
    assert!(schema.is_ok());
    let schema_str = schema.unwrap();

    // Check that it's valid JSON and contains expected fields
    let schema_value: Value = serde_json::from_str(&schema_str).expect("Schema should be JSON");
    assert_eq!(
        schema_value.get("type").and_then(|value| value.as_str()),
        Some("object")
    );
    let properties = schema_value
        .get("properties")
        .and_then(|value| value.as_object())
        .expect("Schema should define properties");
    assert!(properties.contains_key("version"));
    assert!(properties.contains_key("tools"));
}

#[test]
fn test_language_specific_rules_config() {
    let env = TestEnvironment::new();

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
        - "**/*.rs"
    lang-rules:
      python:
        single-line-comments: true
        multi-line-comments: false
        docstrings: false
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME):"
        min-comment-length: 12
        remove-duplicate-comments: true
      rust:
        single-line-comments: true
        doc-comments: false
        preserve-patterns:
          - "^\\s*///\\s*(TODO|FIXME):"
        min-comment-length: 8
"#;

    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let clean_config = config.get_clean_comments_config().unwrap();

    let python_rules = clean_config.lang_rules.python.as_ref().unwrap();
    assert_eq!(python_rules.single_line_comments, Some(true));
    assert_eq!(python_rules.multi_line_comments, Some(false));
    assert_eq!(python_rules.docstrings, Some(false));
    assert_eq!(
        python_rules.preserve_patterns.as_ref().unwrap(),
        &vec!["^\\s*#\\s*(TODO|FIXME):"]
    );
    assert_eq!(python_rules.min_comment_length, Some(12));
    assert_eq!(python_rules.remove_duplicate_comments, Some(true));

    let rust_rules = clean_config.lang_rules.rust.as_ref().unwrap();
    assert_eq!(rust_rules.doc_comments, Some(false));
    assert_eq!(rust_rules.min_comment_length, Some(8));
}
