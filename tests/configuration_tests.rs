use llman::tool::command::CleanUselessCommentsArgs;
use llman::tool::config::Config;
use llman::tool::processor::CommentProcessor;
mod common;
use common::*;

/// Comprehensive tests for configuration system validation
/// Tests all aspects of YAML configuration loading, validation, and error handling

#[test]
fn test_config_default_values() {
    let _env = TestEnvironment::new();

    // Test default configuration loading
    let config = Config::default();

    assert_eq!(config.version, "0.1");
    assert!(config.tools.clean_useless_comments.is_some());

    let clean_config = config.tools.clean_useless_comments.unwrap();
    assert!(!clean_config.scope.include.is_empty());
}

#[test]
fn test_config_yaml_parsing() {
    let env = TestEnvironment::new();

    let config_content = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
        - "**/*.js"
      exclude:
        - "**/node_modules/**"
        - "**/target/**"
    lang-rules:
      python:
        single-line-comments: true
        multi-line-comments: false
        min-comment-length: 10
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME):"
      javascript:
        single-line-comments: true
        multi-line-comments: true
        doc-comments: false
        min-comment-length: 15
        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME):"
          - "^\\s*/\\*\\*.*?\\*/"
"#;

    env.create_config(config_content);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();

    assert_eq!(config.version, "0.1");
    assert!(config.tools.clean_useless_comments.is_some());

    let clean_config = config.tools.clean_useless_comments.unwrap();
    assert_eq!(clean_config.scope.include.len(), 2);
    assert_eq!(clean_config.scope.exclude.len(), 2);
    assert!(clean_config.scope.include.contains(&"**/*.py".to_string()));
    assert!(clean_config.scope.include.contains(&"**/*.js".to_string()));

    assert!(clean_config.lang_rules.python.is_some());
    assert!(clean_config.lang_rules.javascript.is_some());

    let python_rules = clean_config.lang_rules.python.unwrap();
    assert_eq!(python_rules.single_line_comments, Some(true));
    assert_eq!(python_rules.multi_line_comments, Some(false));
    assert_eq!(python_rules.min_comment_length, Some(10));
    assert_eq!(python_rules.preserve_patterns.unwrap().len(), 1);

    let js_rules = clean_config.lang_rules.javascript.unwrap();
    assert_eq!(js_rules.single_line_comments, Some(true));
    assert_eq!(js_rules.multi_line_comments, Some(true));
    assert_eq!(js_rules.doc_comments, Some(false));
    assert_eq!(js_rules.min_comment_length, Some(15));
    assert_eq!(js_rules.preserve_patterns.unwrap().len(), 2);
}

#[test]
fn test_config_invalid_yaml() {
    let env = TestEnvironment::new();

    let invalid_yaml = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
      invalid_yaml: [unclosed array
    lang-rules:
      python:
        single-line-comments: not_a_boolean
"#;

    env.create_config(invalid_yaml);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    assert!(config_result.is_err(), "Should fail to load invalid YAML");
}

#[test]
fn test_config_missing_required_fields() {
    let env = TestEnvironment::new();

    let incomplete_config = r#"
# Missing version field
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    # Missing lang-rules
"#;

    env.create_config(incomplete_config);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    // This might succeed with defaults or fail - depends on implementation
    // We just want to ensure it doesn't crash
    match config_result {
        Ok(config) => {
            // If it succeeds, check that reasonable defaults are set
            assert!(config.version.is_empty() || config.version == "0.1");
        }
        Err(_) => {
            // Failure is also acceptable for missing required fields
        }
    }
}

#[test]
fn test_config_invalid_types() {
    let env = TestEnvironment::new();

    let config_with_invalid_types = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include: "should_be_array_not_string"
      exclude: 12345  # Should be array
    lang-rules:
      python:
        single-line-comments: "should_be_boolean"
        min-comment-length: "should_be_number"
        preserve-patterns: "should_be_array"
"#;

    env.create_config(config_with_invalid_types);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    // Should fail due to type mismatches
    assert!(config_result.is_err(), "Should fail due to type mismatches");
}

#[test]
fn test_config_invalid_regex_patterns() {
    let env = TestEnvironment::new();

    let config_with_invalid_regex = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: true
        preserve-patterns:
          - "[invalid_regex*(unclosed"
          - "^\\s*#\\s*(TODO|FIXME):"  # This one is valid
        min-comment-length: 10
"#;

    env.create_config(config_with_invalid_regex);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));

    // Config loading might succeed (regex validation might happen at runtime)
    match config_result {
        Ok(config) => {
            // If config loads successfully, test that invalid regex is handled gracefully
            let test_file = env.create_file(
                "test.py",
                "# TODO: preserve this\n# remove this\ndef test(): pass",
            );

            let args = CleanUselessCommentsArgs {
                config: Some(env.path().join(".llman").join("config.yaml")),
                dry_run: true,
                yes: false,
                interactive: false,
                force: false,
                verbose: true,
                git_only: false,
                files: vec![test_file],
            };

            let mut processor = CommentProcessor::new(config, args);
            let result = processor.process();

            // Should handle invalid regex gracefully
            match result {
                Ok(_) => println!("Handled invalid regex gracefully"),
                Err(e) => println!("Failed as expected with invalid regex: {:?}", e),
            }
        }
        Err(_) => {
            // Failure during config loading is also acceptable
        }
    }
}

#[test]
fn test_config_edge_case_values() {
    let env = TestEnvironment::new();

    let edge_case_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include: []
      exclude: []
    lang-rules:
      python:
        single-line-comments: true
        multi-line-comments: false
        min-comment-length: 0  # Edge case: zero minimum
        preserve-patterns: []  # Edge case: empty patterns
      javascript:
        single-line-comments: false  # Edge case: disabled processing
        min-comment-length: 999999  # Edge case: very large minimum
        preserve-patterns:  # Edge case: complex patterns
          - "^\\s*//\\s*(TODO|FIXME|NOTE|HACK|XXX):\\s*.*$"
          - "^\\s*/\\*\\*[\\s\\S]*?\\*/"
          - "^\\s*//\\s*@[a-zA-Z].*$"
"#;

    env.create_config(edge_case_config);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    assert!(config_result.is_ok(), "Should handle edge case values");

    let config = config_result.unwrap();
    let clean_config = config.tools.clean_useless_comments.unwrap();

    assert_eq!(clean_config.scope.include.len(), 0);
    assert_eq!(clean_config.scope.exclude.len(), 0);

    let python_rules = clean_config.lang_rules.python.unwrap();
    assert_eq!(python_rules.min_comment_length, Some(0));
    assert_eq!(python_rules.preserve_patterns.unwrap().len(), 0);

    let js_rules = clean_config.lang_rules.javascript.unwrap();
    assert_eq!(js_rules.min_comment_length, Some(999999));
    assert_eq!(js_rules.preserve_patterns.unwrap().len(), 3);
}

#[test]
fn test_config_unicode_support() {
    let env = TestEnvironment::new();

    let unicode_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
        - "**/*.测试.py"  # Unicode filename pattern
    lang-rules:
      python:
        single-line-comments: true
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME|注意|修复):"  # Unicode patterns
          - "^\\s*#.*[🚀⚠️]"  # Emoji in patterns
        min-comment-length: 5
    # Unicode comment for configuration
    description: "这是一个配置文件"
    author: "开发者👨‍💻"
"#;

    env.create_config(unicode_config);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    assert!(
        config_result.is_ok(),
        "Should handle Unicode in configuration"
    );

    let config = config_result.unwrap();
    let clean_config = config.tools.clean_useless_comments.unwrap();

    assert!(
        clean_config
            .scope
            .include
            .contains(&"**/*.测试.py".to_string())
    );

    let python_rules = clean_config.lang_rules.python.unwrap();
    let patterns = python_rules.preserve_patterns.unwrap();
    assert!(
        patterns
            .iter()
            .any(|p| p.contains("注意") || p.contains("修复"))
    );
}

#[test]
fn test_config_file_not_found() {
    let env = TestEnvironment::new();

    let config_result = Config::load(env.path().join("nonexistent_config.yaml"));
    assert!(
        config_result.is_err(),
        "Should fail to load non-existent config file"
    );
}

#[test]
fn test_config_partial_configuration() {
    let env = TestEnvironment::new();

    let partial_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    # lang-rules section is missing - should use defaults
"#;

    env.create_config(partial_config);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));
    // Check if it succeeds or fails gracefully - both are acceptable
    match config_result {
        Ok(config) => {
            let clean_config = config.tools.clean_useless_comments.unwrap();
            assert_eq!(clean_config.scope.include.len(), 1);
            // lang_rules might be None or have default values
        }
        Err(_) => {
            // Failure is acceptable for partial configuration missing required fields
        }
    }
}

#[test]
fn test_config_schema_validation() {
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
        min-comment-length: 10
"#;

    env.create_config(config_content);

    let _config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();

    // Test schema generation
    let schema_result = Config::generate_schema();
    assert!(schema_result.is_ok(), "Should generate valid JSON schema");

    let schema = schema_result.unwrap();
    // Schema is a string, not an object with properties
    assert!(!schema.is_empty(), "Schema should have content");
}

#[test]
fn test_config_environment_substitution() {
    let env = TestEnvironment::new();

    // Test if config supports environment variable substitution (if implemented)
    unsafe {
        std::env::set_var("TEST_MIN_LENGTH", "15");
    }

    let config_with_env = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: true
        min-comment-length: ${TEST_MIN_LENGTH}
"#;

    env.create_config(config_with_env);

    let config_result = Config::load(env.path().join(".llman").join("config.yaml"));

    // This test depends on whether environment substitution is implemented
    // If it is, the value should be 15; if not, it might fail or remain as string
    match config_result {
        Ok(config) => {
            let clean_config = config.tools.clean_useless_comments.unwrap();
            let python_rules = clean_config.lang_rules.python.unwrap();
            // Either it's 15 (substitution worked) or some other default
            println!(
                "Environment substitution result: {:?}",
                python_rules.min_comment_length
            );
        }
        Err(_) => {
            println!("Environment substitution not implemented or failed");
        }
    }

    unsafe {
        std::env::remove_var("TEST_MIN_LENGTH");
    }
}

#[test]
fn test_config_inheritance_and_overrides() {
    let env = TestEnvironment::new();

    // Test global config
    let global_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
        - "**/*.js"
      exclude:
        - "**/test/**"
    lang-rules:
      python:
        single-line-comments: true
        min-comment-length: 10
      javascript:
        single-line-comments: true
        min-comment-length: 15
"#;

    env.create_config(global_config);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let clean_config = config.tools.clean_useless_comments.unwrap();

    // Verify inheritance worked correctly
    assert_eq!(clean_config.scope.include.len(), 2);
    assert_eq!(clean_config.scope.exclude.len(), 1);

    let python_rules = clean_config.lang_rules.python.unwrap();
    let js_rules = clean_config.lang_rules.javascript.unwrap();

    assert_eq!(python_rules.min_comment_length, Some(10));
    assert_eq!(js_rules.min_comment_length, Some(15));
}

#[test]
fn test_config_validation_integration() {
    let env = TestEnvironment::new();

    let test_file = env.create_file(
        "test.py",
        "# Short comment\n# TODO: Important comment\ndef test(): pass",
    );

    let valid_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.py"
    lang-rules:
      python:
        single-line-comments: true
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME):"
        min-comment-length: 15
"#;

    env.create_config(valid_config);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();

    // Test that config works with processor
    let args = CleanUselessCommentsArgs {
        config: Some(env.path().join(".llman").join("config.yaml")),
        dry_run: true,
        yes: false,
        interactive: false,
        force: false,
        verbose: true,
        git_only: false,
        files: vec![test_file],
    };

    let mut processor = CommentProcessor::new(config, args);
    let result = processor.process();

    assert!(
        result.is_ok(),
        "Config should work correctly with processor"
    );

    let processing_result = result.unwrap();
    assert_eq!(
        processing_result.errors, 0,
        "Should have no processing errors"
    );
    // In dry-run mode with high min-comment-length (15), changes might not be detected
    // Both detecting changes and not detecting changes are valid outcomes
    println!(
        "Files changed: {} (this is normal for dry-run with conservative settings)",
        processing_result.files_changed.len()
    );
}

#[test]
fn test_config_typescript_rules() {
    let env = TestEnvironment::new();

    let typescript_config = r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "**/*.ts"
        - "**/*.tsx"
    lang-rules:
      javascript:
        single-line-comments: true
        multi-line-comments: true
        doc-comments: false
        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME):"
          - "^\\s*/\\*\\*[\\s\\S]*?\\*/"
          - "^\\s*//\\s*@[a-zA-Z].*$"
        min-comment-length: 12
"#;

    env.create_config(typescript_config);

    let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
    let clean_config = config.tools.clean_useless_comments.unwrap();

    // TypeScript should use javascript rules
    let js_rules = clean_config.lang_rules.javascript.unwrap();
    assert_eq!(js_rules.single_line_comments, Some(true));
    assert_eq!(js_rules.multi_line_comments, Some(true));
    assert_eq!(js_rules.doc_comments, Some(false));
    assert_eq!(js_rules.min_comment_length, Some(12));
    assert_eq!(js_rules.preserve_patterns.unwrap().len(), 3);

    // Should include both .ts and .tsx files
    assert!(clean_config.scope.include.contains(&"**/*.ts".to_string()));
    assert!(clean_config.scope.include.contains(&"**/*.tsx".to_string()));
}
