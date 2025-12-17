use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test constants for consistent test values
#[allow(dead_code)]
pub mod test_constants {
    /// Default minimum comment length for testing
    pub const DEFAULT_MIN_COMMENT_LENGTH: u32 = 20;
    /// Short comment length (should be removed)
    pub const SHORT_COMMENT_LENGTH: u32 = 5;
    /// Comment length that should be preserved
    pub const PRESERVED_COMMENT_LENGTH: u32 = 25;

    /// Default preserve patterns for TODO/FIXME
    pub const DEFAULT_PRESERVE_PATTERNS: &[&str] = &["^\\s*#\\s*(TODO|FIXME):"];

    /// Default file patterns for Python files
    pub const PYTHON_FILE_PATTERN: &str = "**/*.py";
    /// Default file patterns for JavaScript files
    pub const JS_FILE_PATTERN: &str = "**/*.js";
    /// Default file patterns for Rust files
    pub const RUST_FILE_PATTERN: &str = "**/*.rs";
}

/// Test content samples
#[allow(dead_code)]
pub mod test_content {
    /// Sample Python code with various comment types
    pub const PYTHON_CODE_WITH_COMMENTS: &str = r#"#!/usr/bin/env python3
# This is a short comment that should be removed
def hello():
    # Another short comment
    print("Hello")  # Inline comment
    # TODO: This should be preserved
    # FIXME: This should also be preserved
    return "done"
"#;

    /// Sample JavaScript code with comments
    pub const JAVASCRIPT_CODE_WITH_COMMENTS: &str = r#"// Short comment
function hello() {
    console.log("Hello"); // Inline comment
    // TODO: This should be preserved
    return "done";
}
"#;

    /// Sample Rust code with comments
    pub const RUST_CODE_WITH_COMMENTS: &str = r#"// Short comment
fn main() {
    println!("Hello"); // Inline comment
    /// This is a doc comment and should be preserved
    // TODO: This should be preserved
}
"#;

    /// Python code with only important comments
    pub const PYTHON_CODE_IMPORTANT_COMMENTS: &str = r#"#!/usr/bin/env python3
# x
def important_function():
    # TODO: This is a TODO item
    # FIXME: This needs to be fixed
    # NOTE: This is an important note
    # y
    print("Hello")
"#;
}

#[allow(dead_code)]
pub fn create_test_file_with_content(
    dir: &Path,
    filename: &str,
    content: &str,
) -> std::path::PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("Failed to write test file");
    file_path
}

pub fn create_test_config(dir: &Path, config_content: &str) -> std::path::PathBuf {
    let config_dir = dir.join(".llman");
    fs::create_dir_all(&config_dir).expect("Failed to create .llman directory");
    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, config_content).expect("Failed to write config file");
    config_path
}

pub struct TestEnvironment {
    // TempDir is kept to ensure cleanup happens when TestEnvironment is dropped
    #[allow(dead_code)]
    pub(crate) temp_dir: TempDir,
    pub work_dir: std::path::PathBuf,
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_dir = temp_dir.path().to_path_buf();

        // Initialize git repo for testing
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&work_dir)
            .output()
            .expect("Failed to initialize git repo");

        Self { temp_dir, work_dir }
    }

    pub fn path(&self) -> &Path {
        &self.work_dir
    }

    #[allow(dead_code)]
    pub fn create_file(&self, filename: &str, content: &str) -> std::path::PathBuf {
        create_test_file_with_content(&self.work_dir, filename, content)
    }

    pub fn create_config(&self, config_content: &str) -> std::path::PathBuf {
        create_test_config(&self.work_dir, config_content)
    }

    /// Create a test configuration for Python comment cleaning
    #[allow(dead_code)]
    pub fn create_python_clean_config(&self, min_comment_length: u32) -> std::path::PathBuf {
        let config = format!(
            r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "{}"
    lang-rules:
      python:
        single-line-comments: true
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME):"
        min-comment-length: {}
"#,
            test_constants::PYTHON_FILE_PATTERN,
            min_comment_length
        );
        self.create_config(&config)
    }

    /// Create a test configuration for JavaScript comment cleaning
    #[allow(dead_code)]
    pub fn create_javascript_clean_config(&self, min_comment_length: u32) -> std::path::PathBuf {
        let config = format!(
            r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "{}"
    lang-rules:
      javascript:
        single-line-comments: true
        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME):"
        min-comment-length: {}
"#,
            test_constants::JS_FILE_PATTERN,
            min_comment_length
        );
        self.create_config(&config)
    }

    /// Create a test configuration for Rust comment cleaning
    #[allow(dead_code)]
    pub fn create_rust_clean_config(&self, min_comment_length: u32) -> std::path::PathBuf {
        let config = format!(
            r#"
version: "0.1"
tools:
  clean-useless-comments:
    scope:
      include:
        - "{}"
    lang-rules:
      rust:
        single-line-comments: true
        doc-comments: false
        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME):"
          - "^\\s*///"
        min-comment-length: {}
"#,
            test_constants::RUST_FILE_PATTERN,
            min_comment_length
        );
        self.create_config(&config)
    }
}
