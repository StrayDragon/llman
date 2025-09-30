use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub fn create_test_file_with_content(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
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
    pub temp_dir: TempDir,
    pub work_dir: std::path::PathBuf,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_dir = temp_dir.path().to_path_buf();

        // Initialize git repo for testing
        std::process::Command::new("git")
            .args(&["init", "--quiet"])
            .current_dir(&work_dir)
            .output()
            .expect("Failed to initialize git repo");

        Self { temp_dir, work_dir }
    }

    pub fn path(&self) -> &Path {
        &self.work_dir
    }

    pub fn create_file(&self, filename: &str, content: &str) -> std::path::PathBuf {
        create_test_file_with_content(&self.work_dir, filename, content)
    }

    pub fn create_config(&self, config_content: &str) -> std::path::PathBuf {
        create_test_config(&self.work_dir, config_content)
    }
}