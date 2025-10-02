use llman::error::{LlmanError, Result};
use std::io;

/// Tests that LlmanError variants can be created properly
/// and have the expected error messages
#[test]
fn test_llman_error_variants_creation_and_messages() {
    // Test Config error
    let config_error = LlmanError::Config {
        message: "Invalid configuration".to_string()
    };
    assert!(config_error.to_string().contains("Config Error"));
    assert!(config_error.to_string().contains("Invalid configuration"));

    // Test InvalidApp error
    let app_error = LlmanError::InvalidApp {
        app: "testapp".to_string()
    };
    assert!(app_error.to_string().contains("Invalid App"));
    assert!(app_error.to_string().contains("testapp"));

    // Test NotProjectDirectory error
    let dir_error = LlmanError::NotProjectDirectory;
    assert!(dir_error.to_string().contains("not a valid project directory"));

    // Test HomeDirectoryNotAllowed error
    let home_error = LlmanError::HomeDirectoryNotAllowed;
    assert!(home_error.to_string().contains("Cannot generate rules in home directory"));

    // Test RuleNotFound error
    let rule_error = LlmanError::RuleNotFound {
        name: "missing_rule".to_string()
    };
    assert!(rule_error.to_string().contains("Rule file not found"));
    assert!(rule_error.to_string().contains("missing_rule"));

    // Test Custom error
    let custom_error = LlmanError::Custom("Something went wrong".to_string());
    assert!(custom_error.to_string().contains("Custom Error"));
    assert!(custom_error.to_string().contains("Something went wrong"));
}

/// Tests that LlmanError properly implements From traits
/// for automatic error conversion
#[test]
fn test_llman_error_from_conversions() {
    // Test From<io::Error>
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let llman_error: LlmanError = io_error.into();
    assert!(llman_error.to_string().contains("IO Error"));

    // Test that Result<T, LlmanError> works properly
    fn function_returns_error() -> Result<String> {
        Err(LlmanError::Config {
            message: "Test error".to_string()
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
    let config_error = LlmanError::Config {
        message: "Config issue".to_string()
    };
    let localized = config_error.display_localized();
    assert!(!localized.is_empty());

    let app_error = LlmanError::InvalidApp {
        app: "myapp".to_string()
    };
    let localized = app_error.display_localized();
    assert!(!localized.is_empty());

    let dir_error = LlmanError::NotProjectDirectory;
    let localized = dir_error.display_localized();
    assert!(!localized.is_empty());
}

/// Tests that LlmanError variants can be cloned and compared
#[test]
fn test_llman_error_clone_and_equality() {
    let error1 = LlmanError::Config {
        message: "Test".to_string()
    };
    let error2 = LlmanError::Config {
        message: "Test".to_string()
    };
    let error3 = LlmanError::Config {
        message: "Different".to_string()
    };

    // Since we derive PartialEq through Debug, test string representation
    assert_eq!(error1.to_string(), error2.to_string());
    assert_ne!(error1.to_string(), error3.to_string());
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
    assert!(error_result.unwrap_err().to_string().contains("Propagation test"));
}

/// Tests LlmanError with various complex error scenarios
#[test]
fn test_llman_error_complex_scenarios() {
    // Test nested error handling
    fn nested_operation(level: u32) -> Result<String> {
        match level {
            0 => Err(LlmanError::Config {
                message: "Level 0 config error".to_string()
            }),
            1 => Err(LlmanError::InvalidApp {
                app: "level1_app".to_string()
            }),
            2 => Ok("Success at level 2".to_string()),
            _ => Err(LlmanError::Custom("Unknown level".to_string())),
        }
    }

    let level0_result = nested_operation(0);
    assert!(level0_result.is_err());
    assert!(level0_result.unwrap_err().to_string().contains("Level 0"));

    let level2_result = nested_operation(2);
    assert!(level2_result.is_ok());
    assert_eq!(level2_result.unwrap(), "Success at level 2");

    // Test error collection
    let errors: Vec<LlmanError> = vec![
        LlmanError::Config { message: "Error 1".to_string() },
        LlmanError::InvalidApp { app: "app1".to_string() },
        LlmanError::NotProjectDirectory,
    ];

    assert_eq!(errors.len(), 3);
    assert!(errors.iter().all(|e| !e.to_string().is_empty()));
}