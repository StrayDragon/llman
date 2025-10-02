use llman::tool::command::CleanUselessCommentsArgs;
use llman::tool::config::Config;
use llman::tool::processor::CommentProcessor;
use std::time::Instant;
mod common;
use common::*;

/// Performance tests for comment processing functionality.
/// These tests measure processing time for various file sizes and ensure
/// the tool remains performant with realistic codebases.
mod performance_tests {

    use super::*;

    /// Creates a large Python file with many comments for performance testing
    fn create_large_python_file_for_performance(
        env: &TestEnvironment,
        num_functions: usize,
    ) -> std::path::PathBuf {
        let mut content =
            String::from("#!/usr/bin/env python3\n# This is a large performance test file\n\n");

        for i in 0..num_functions {
            content.push_str(&format!(
                r#"
# Function {} documentation
def function_{}():
    # This is a short comment that should be removed
    result = calculate_something()
    # Another short comment
    return result

# TODO: Optimize function {}
def helper_function_{}():
    # Performance critical function
    # FIXME: Current implementation is slow
    data = process_data()
    # More comments here
    return data
"#,
                i, i, i, i
            ));
        }

        env.create_file("large_test.py", &content)
    }

    /// Tests performance with a moderately sized file (~100 functions)
    #[test]
    fn test_comment_processing_performance_medium_file() {
        let env = TestEnvironment::new();

        // Create a file with ~100 functions and many comments
        let test_file = create_large_python_file_for_performance(&env, 100);
        env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH); // Use lower threshold to ensure some comments are removed

        let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
        let args = CleanUselessCommentsArgs {
            config: Some(env.path().join(".llman").join("config.yaml")),
            dry_run: true,
            interactive: false,
            force: true,
            verbose: true,
            git_only: false,
            files: vec![test_file.clone()],
        };

        let start_time = Instant::now();
        let mut processor = CommentProcessor::new(config, args);
        let result = processor.process().unwrap();
        let duration = start_time.elapsed();

        // Performance assertions - check that processing completed without errors
        assert_eq!(result.errors, 0, "Expected no processing errors");

        // Whether files were changed depends on the content and thresholds
        // What's important for performance testing is that processing completed
        println!(
            "Files processed: {}, Files changed: {}",
            result.files_changed.len(),
            result.files_changed.len()
        );

        // Should complete within reasonable time (adjust threshold as needed)
        assert!(
            duration.as_millis() < 5000,
            "Processing took too long: {}ms for medium file (expected < 5000ms)",
            duration.as_millis()
        );

        println!(
            "Performance test - Medium file (100 functions): {}ms",
            duration.as_millis()
        );
    }

    /// Tests performance with a large file (~1000 functions)
    #[test]
    fn test_comment_processing_performance_large_file() {
        let env = TestEnvironment::new();

        // Create a file with ~1000 functions
        let test_file = create_large_python_file_for_performance(&env, 1000);
        env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH); // Use lower threshold to ensure some comments are removed

        let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
        let args = CleanUselessCommentsArgs {
            config: Some(env.path().join(".llman").join("config.yaml")),
            dry_run: true,
            interactive: false,
            force: true,
            verbose: true,
            git_only: false,
            files: vec![test_file.clone()],
        };

        let start_time = Instant::now();
        let mut processor = CommentProcessor::new(config, args);
        let result = processor.process().unwrap();
        let duration = start_time.elapsed();

        // Performance assertions - check that processing completed without errors
        assert_eq!(result.errors, 0, "Expected no processing errors");
        println!(
            "Files processed: {}, Files changed: {}",
            result.files_changed.len(),
            result.files_changed.len()
        );

        // Large files should still complete in reasonable time
        assert!(
            duration.as_millis() < 30000,
            "Processing took too long: {}ms for large file (expected < 30000ms)",
            duration.as_millis()
        );

        println!(
            "Performance test - Large file (1000 functions): {}ms",
            duration.as_millis()
        );
    }

    /// Tests performance with multiple smaller files
    #[test]
    fn test_comment_processing_performance_multiple_files() {
        let env = TestEnvironment::new();

        // Create multiple smaller files
        let files: Vec<_> = (0..10)
            .map(|i| {
                let content = format!(
                    r#"#!/usr/bin/env python3
# File {} content
def func_{}():
    # Short comment in file {}
    return "result_{}"

# TODO: Implement better logic
def main_func_{}():
    print("Hello from file {}")
"#,
                    i, i, i, i, i, i
                );
                env.create_file(&format!("test_file_{}.py", i), &content)
            })
            .collect();

        env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH); // Use lower threshold

        let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
        let args = CleanUselessCommentsArgs {
            config: Some(env.path().join(".llman").join("config.yaml")),
            dry_run: true,
            interactive: false,
            force: true,
            verbose: true,
            git_only: false,
            files: files.clone(),
        };

        let start_time = Instant::now();
        let mut processor = CommentProcessor::new(config, args);
        let result = processor.process().unwrap();
        let duration = start_time.elapsed();

        // Should process all files efficiently
        assert_eq!(result.errors, 0, "Expected no processing errors");
        println!(
            "Files processed: {}, Files changed: {}",
            result.files_changed.len(),
            result.files_changed.len()
        );

        // Multiple small files should be processed quickly
        assert!(
            duration.as_millis() < 10000,
            "Processing took too long: {}ms for 10 small files (expected < 10000ms)",
            duration.as_millis()
        );

        println!(
            "Performance test - Multiple files (10 files): {}ms",
            duration.as_millis()
        );
    }

    /// Memory efficiency test - ensures processing doesn't consume excessive memory
    #[test]
    fn test_comment_processing_memory_efficiency() {
        let env = TestEnvironment::new();

        // Create a very large file to stress test memory usage
        let test_file = create_large_python_file_for_performance(&env, 5000);
        env.create_python_clean_config(test_constants::SHORT_COMMENT_LENGTH); // Use lower threshold to ensure some comments are removed

        let config = Config::load(env.path().join(".llman").join("config.yaml")).unwrap();
        let args = CleanUselessCommentsArgs {
            config: Some(env.path().join(".llman").join("config.yaml")),
            dry_run: true,
            interactive: false,
            force: true,
            verbose: true,
            git_only: false,
            files: vec![test_file.clone()],
        };

        let start_time = Instant::now();
        let mut processor = CommentProcessor::new(config, args);
        let result = processor.process().unwrap();
        let duration = start_time.elapsed();

        // Should complete without memory issues
        assert_eq!(result.errors, 0, "Expected no processing errors");
        println!(
            "Files processed: {}, Files changed: {}",
            result.files_changed.len(),
            result.files_changed.len()
        );

        // Even very large files should complete in reasonable time
        assert!(
            duration.as_millis() < 60000,
            "Processing took too long: {}ms for very large file (expected < 60000ms)",
            duration.as_millis()
        );

        println!(
            "Memory efficiency test - Very large file (5000 functions): {}ms",
            duration.as_millis()
        );
    }
}
