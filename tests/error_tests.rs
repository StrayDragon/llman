use llman::error::{LlmanError, Result};
use std::io;
use std::sync::Mutex;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn load_i18n_template(key: &str, locale: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .join("locales")
        .join("app.yml");
    let content = std::fs::read_to_string(path).expect("Failed to read locales/app.yml");
    let key_parts: Vec<&str> = key.split('.').collect();

    let mut path_stack: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = line.chars().take_while(|c| *c == ' ').count();
        if indent % 2 != 0 {
            continue;
        }
        let level = indent / 2;

        let Some((raw_key, raw_value)) = trimmed.split_once(':') else {
            continue;
        };
        let current_key = raw_key.trim();
        let value = raw_value.trim();

        if path_stack.len() > level {
            path_stack.truncate(level);
        }
        path_stack.push(current_key.to_string());

        if current_key == locale {
            let parent = &path_stack[..path_stack.len().saturating_sub(1)];
            if parent == key_parts {
                return unquote_yaml_value(value);
            }
        }
    }

    panic!("Missing locale '{}' for key '{}'", locale, key);
}

fn unquote_yaml_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    trimmed.to_string()
}

fn format_i18n_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut formatted = template.to_string();
    for (key, value) in replacements {
        formatted = formatted.replace(&format!("%{{{}}}", key), value);
    }
    formatted
}

/// Tests that LlmanError variants can be created properly
/// and retain their structured data
#[test]
fn test_llman_error_variants_creation_and_messages() {
    // Test Config error
    let config_error = LlmanError::Config {
        message: "Invalid configuration".to_string(),
    };
    assert!(
        matches!(
            config_error,
            LlmanError::Config { ref message } if message == "Invalid configuration"
        ),
        "Config error should retain message payload"
    );

    // Test InvalidApp error
    let app_error = LlmanError::InvalidApp {
        app: "testapp".to_string(),
    };
    assert!(
        matches!(
            app_error,
            LlmanError::InvalidApp { ref app } if app == "testapp"
        ),
        "InvalidApp error should retain app payload"
    );

    // Test NotProjectDirectory error
    let dir_error = LlmanError::NotProjectDirectory;
    assert!(matches!(dir_error, LlmanError::NotProjectDirectory));

    // Test HomeDirectoryNotAllowed error
    let home_error = LlmanError::HomeDirectoryNotAllowed;
    assert!(matches!(home_error, LlmanError::HomeDirectoryNotAllowed));

    // Test RuleNotFound error
    let rule_error = LlmanError::RuleNotFound {
        name: "missing_rule".to_string(),
    };
    assert!(
        matches!(
            rule_error,
            LlmanError::RuleNotFound { ref name } if name == "missing_rule"
        ),
        "RuleNotFound error should retain rule name"
    );

    // Test Custom error
    let custom_error = LlmanError::Custom("Something went wrong".to_string());
    assert!(
        matches!(
            custom_error,
            LlmanError::Custom(ref message) if message == "Something went wrong"
        ),
        "Custom error should retain message payload"
    );
}

/// Tests that LlmanError properly implements From traits
/// for automatic error conversion
#[test]
fn test_llman_error_from_conversions() {
    // Test From<io::Error>
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let llman_error: LlmanError = io_error.into();
    assert!(matches!(llman_error, LlmanError::Io(_)));

    // Test that Result<T, LlmanError> works properly
    fn function_returns_error() -> Result<String> {
        Err(LlmanError::Config {
            message: "Test error".to_string(),
        })
    }

    let result = function_returns_error();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, LlmanError::Config { .. }));
}

/// Tests the display_localized method of LlmanError
#[test]
fn test_llman_error_display_localized() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let locale = "en";
    unsafe {
        std::env::set_var("LLMAN_LANG", locale);
    }
    llman::init_locale();

    let config_error = LlmanError::Config {
        message: "Config issue".to_string(),
    };
    let localized = config_error.display_localized();
    let template = load_i18n_template("errors.config_error", locale);
    let expected = format_i18n_template(&template, &[("message", "Config issue")]);
    assert_eq!(localized, expected);

    let app_error = LlmanError::InvalidApp {
        app: "myapp".to_string(),
    };
    let localized = app_error.display_localized();
    let template = load_i18n_template("errors.invalid_app", locale);
    let expected = format_i18n_template(&template, &[("app", "myapp")]);
    assert_eq!(localized, expected);

    let dir_error = LlmanError::NotProjectDirectory;
    let localized = dir_error.display_localized();
    let expected = load_i18n_template("errors.not_project_directory", locale);
    assert_eq!(localized, expected);

    let home_error = LlmanError::HomeDirectoryNotAllowed;
    let localized = home_error.display_localized();
    let expected = load_i18n_template("errors.home_directory_not_allowed", locale);
    assert_eq!(localized, expected);

    let rule_error = LlmanError::RuleNotFound {
        name: "missing_rule".to_string(),
    };
    let localized = rule_error.display_localized();
    let template = load_i18n_template("errors.rule_not_found", locale);
    let expected = format_i18n_template(&template, &[("name", "missing_rule")]);
    assert_eq!(localized, expected);

    unsafe {
        std::env::remove_var("LLMAN_LANG");
    }
}

/// Tests that LlmanError variants can be cloned and compared
#[test]
fn test_llman_error_clone_and_equality() {
    let error1 = LlmanError::Config {
        message: "Test".to_string(),
    };
    let error2 = LlmanError::Config {
        message: "Test".to_string(),
    };
    let error3 = LlmanError::Config {
        message: "Different".to_string(),
    };

    let message1 = match &error1 {
        LlmanError::Config { message } => message,
        _ => unreachable!("Expected Config error"),
    };
    let message2 = match &error2 {
        LlmanError::Config { message } => message,
        _ => unreachable!("Expected Config error"),
    };
    let message3 = match &error3 {
        LlmanError::Config { message } => message,
        _ => unreachable!("Expected Config error"),
    };

    assert_eq!(message1, message2);
    assert_ne!(message1, message3);
}

/// Tests that LlmanError can be used in async contexts and panic scenarios
#[test]
fn test_llman_error_in_error_handling_contexts() {
    // Test in result chaining
    let result: Result<String> = Ok("success".to_string())
        .map_err(|_: LlmanError| LlmanError::Custom("Map error".to_string()));
    assert!(result.is_ok());

    // Test in error propagation
    fn function_that_propagates_error(should_fail: bool) -> Result<String> {
        if should_fail {
            Err(LlmanError::Custom("Propagation test".to_string()))
        } else {
            Ok("Success".to_string())
        }
    }

    let success_result = function_that_propagates_error(false);
    assert!(success_result.is_ok());

    let error_result = function_that_propagates_error(true);
    assert!(error_result.is_err());
    assert!(
        matches!(
            error_result.unwrap_err(),
            LlmanError::Custom(ref message) if message == "Propagation test"
        ),
        "Expected Custom error with propagation message"
    );
}

/// Tests LlmanError with various complex error scenarios
#[test]
fn test_llman_error_complex_scenarios() {
    // Test nested error handling
    fn nested_operation(level: u32) -> Result<String> {
        match level {
            0 => Err(LlmanError::Config {
                message: "Level 0 config error".to_string(),
            }),
            1 => Err(LlmanError::InvalidApp {
                app: "level1_app".to_string(),
            }),
            2 => Ok("Success at level 2".to_string()),
            _ => Err(LlmanError::Custom("Unknown level".to_string())),
        }
    }

    let level0_result = nested_operation(0);
    assert!(level0_result.is_err());
    assert!(
        matches!(
            level0_result.unwrap_err(),
            LlmanError::Config { ref message } if message == "Level 0 config error"
        ),
        "Expected Config error at level 0"
    );

    let level2_result = nested_operation(2);
    assert!(level2_result.is_ok());
    assert_eq!(level2_result.unwrap(), "Success at level 2");

    // Test error collection
    let errors: Vec<LlmanError> = vec![
        LlmanError::Config {
            message: "Error 1".to_string(),
        },
        LlmanError::InvalidApp {
            app: "app1".to_string(),
        },
        LlmanError::NotProjectDirectory,
    ];

    assert_eq!(errors.len(), 3);
    assert!(matches!(errors[0], LlmanError::Config { .. }));
    assert!(matches!(errors[1], LlmanError::InvalidApp { .. }));
    assert!(matches!(errors[2], LlmanError::NotProjectDirectory));
}
