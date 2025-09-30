use crate::tool::config::Config;
use crate::tool::command::CleanUselessCommentsArgs;
use crate::tool::tree_sitter_processor::TreeSitterProcessor;
use anyhow::{Result, anyhow};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;

pub struct CommentProcessor {
    config: Config,
    args: CleanUselessCommentsArgs,
    tree_sitter_processor: Option<TreeSitterProcessor>,
}

impl CommentProcessor {
    pub fn new(config: Config, args: CleanUselessCommentsArgs) -> Self {
        let tree_sitter_processor = match TreeSitterProcessor::new() {
            Ok(processor) => Some(processor),
            Err(e) => {
                eprintln!("Warning: Failed to initialize TreeSitter processor: {}", e);
                None
            }
        };

        Self { config, args, tree_sitter_processor }
    }

    pub fn process(&mut self) -> Result<ProcessingResult> {
        let clean_config = self.config.get_clean_comments_config()
            .ok_or_else(|| anyhow!("No clean useless comments configuration found"))?;

        println!("Processing files...");

        // Clone the clean_config to avoid borrow issues
        let clean_config_clone = clean_config.clone();
        let files_to_process = self.find_files(&clean_config_clone.scope)?;
        let mut results = ProcessingResult::default();

        for file_path in files_to_process {
            if self.args.verbose {
                println!("Processing: {}", file_path.display());
            }

            match self.process_file(&file_path, &clean_config_clone) {
                Ok(file_result) => {
                    if file_result.has_changes() {
                        results.files_changed.push(file_path.clone());
                        results.comments_removed += file_result.comments_removed;

                        if self.args.verbose || !self.args.dry_run {
                            println!("File: {} - Removed {} comments",
                                file_path.display(),
                                file_result.comments_removed);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error processing {}: {}", file_path.display(), e);
                    results.errors += 1;
                }
            }
        }

        Ok(results)
    }

    fn find_files(&self, scope: &crate::tool::config::ScopeConfig) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // If specific files are provided, use them
        if !self.args.files.is_empty() {
            for file in &self.args.files {
                if file.exists() {
                    files.push(file.clone());
                } else {
                    eprintln!("Warning: File not found: {}", file.display());
                }
            }
            return Ok(files);
        }

        // Otherwise, walk the directory tree
        let mut walker = WalkBuilder::new(".");

        // Add include patterns
        for pattern in &scope.include {
            walker.add_custom_ignore_filename(pattern);
        }

        for result in walker.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        // Check if file matches include patterns
                        if self.matches_patterns(path, &scope.include, &scope.exclude) {
                            files.push(path.to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error walking directory: {}", e);
                }
            }
        }

        Ok(files)
    }

    fn matches_patterns(&self, path: &Path, include: &[String], exclude: &[String]) -> bool {
        let _path_str = path.to_string_lossy();

        // Check exclude patterns first
        for pattern in exclude {
            if glob::Pattern::new(pattern).map(|p| p.matches_path(path)).unwrap_or(false) {
                return false;
            }
        }

        // Check include patterns
        for pattern in include {
            if glob::Pattern::new(pattern).map(|p| p.matches_path(path)).unwrap_or(false) {
                return true;
            }
        }

        false
    }

    fn process_file(&mut self, file_path: &Path, clean_config: &crate::tool::config::CleanUselessCommentsConfig) -> Result<FileProcessingResult> {
        let content = fs::read_to_string(file_path)?;
        let language = self.detect_language(file_path);

        if let Some(lang_rules) = self.get_language_rules(language, &clean_config.lang_rules) {
            let (new_content, comments_removed) = self.remove_comments_with_tree_sitter(&content, file_path, lang_rules)?;

            let has_changes = new_content != content;

            if !self.args.dry_run && has_changes {
                fs::write(file_path, new_content)?;
            }

            Ok(FileProcessingResult {
                comments_removed,
                has_changes,
            })
        } else {
            // No rules for this language, skip
            Ok(FileProcessingResult {
                comments_removed: 0,
                has_changes: false,
            })
        }
    }

    fn detect_language(&self, path: &Path) -> Option<&'static str> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("py") => Some("python"),
            Some("js") => Some("javascript"),
            Some("ts") => Some("typescript"),
            Some("rs") => Some("rust"),
            Some("go") => Some("go"),
            _ => None,
        }
    }

    fn get_language_rules<'a>(&self, language: Option<&str>, lang_rules: &'a crate::tool::config::LanguageRules) -> Option<&'a crate::tool::config::LanguageSpecificRules> {
        match language {
            Some("python") => lang_rules.python.as_ref(),
            Some("javascript") | Some("typescript") => lang_rules.javascript.as_ref(),
            Some("rust") => lang_rules.rust.as_ref(),
            Some("go") => lang_rules.go.as_ref(),
            _ => None,
        }
    }

    fn remove_comments(&self, content: &str, rules: &crate::tool::config::LanguageSpecificRules) -> Result<String> {
        let mut result = content.to_string();

        // Remove single-line comments if enabled
        if rules.single_line_comments.unwrap_or(false) {
            result = self.remove_single_line_comments(&result, rules)?;
        }

        // Remove multi-line comments if enabled
        if rules.multi_line_comments.unwrap_or(false) {
            result = self.remove_multi_line_comments(&result, rules)?;
        }

        Ok(result)
    }

    fn remove_single_line_comments(&self, content: &str, rules: &crate::tool::config::LanguageSpecificRules) -> Result<String> {
        let empty_patterns = vec![];
        let preserve_patterns = rules.preserve_patterns.as_ref().unwrap_or(&empty_patterns);
        let preserve_regexes: Vec<_> = preserve_patterns.iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        if self.args.verbose {
            println!("Found {} preserve patterns", preserve_regexes.len());
        }

        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        for line in lines {
            let mut should_remove = false;

            // Check if line contains a comment
            if let Some(comment_start) = line.find('#').or_else(|| line.find("//")) {
                let before_comment = &line[..comment_start];
                let comment = &line[comment_start..];

                // Check if we should preserve this comment
                let should_preserve = preserve_regexes.iter().any(|regex| regex.is_match(comment.trim()));

                // Check minimum length requirement
                let min_length = rules.min_comment_length.unwrap_or(0);
                let comment_too_short = comment.trim().len() < min_length;

                // Check if the line is mostly comment (simple heuristic)
                let _mostly_comment = before_comment.trim().len() <= 10; // Simple threshold

                should_remove = !should_preserve && comment_too_short;

                if self.args.verbose {
                    println!("Processing line: '{}'", line);
                    println!("  - comment: '{}'", comment.trim());
                    println!("  - should_preserve: {}", should_preserve);
                    println!("  - comment_too_short: {} (len: {}, min: {})", comment_too_short, comment.trim().len(), min_length);
                    println!("  - would remove: {}", should_remove);
                }
            }

            if should_remove {
                // Remove the comment part
                if let Some(comment_start) = line.find('#').or_else(|| line.find("//")) {
                    result.push(&line[..comment_start]);
                }
            } else {
                result.push(line);
            }
        }

        Ok(result.join("\n"))
    }

    fn remove_multi_line_comments(&self, content: &str, _rules: &crate::tool::config::LanguageSpecificRules) -> Result<String> {
        // Simple multi-line comment removal (basic implementation)
        // This is a placeholder - a proper implementation would need more sophisticated parsing
        let mut result = content.to_string();

        // Remove C-style multi-line comments /* ... */
        let re = Regex::new(r"/\*.*?\*/").map_err(|e| anyhow!("Failed to build regex: {}", e))?;
        result = re.replace_all(&result, "").to_string();

        Ok(result)
    }

    fn remove_comments_with_tree_sitter(&mut self, content: &str, file_path: &Path, rules: &crate::tool::config::LanguageSpecificRules) -> Result<(String, u32)> {
        if let Some(ref mut tree_sitter_processor) = self.tree_sitter_processor {
            match tree_sitter_processor.remove_comments_from_content(content, file_path, rules) {
                Ok((new_content, removed_comments)) => {
                    let comments_removed = removed_comments.len() as u32;
                    Ok((new_content, comments_removed))
                }
                Err(e) => {
                    if self.args.verbose {
                        eprintln!("Tree-sitter processing failed for {}: {}, falling back to regex", file_path.display(), e);
                    }
                    // Fallback to regex-based processing
                    let new_content = self.remove_comments(content, rules)?;
                    let comments_removed = if new_content != content {
                        self.count_removed_comments(content, &new_content)
                    } else {
                        0
                    };
                    Ok((new_content, comments_removed))
                }
            }
        } else {
            // No tree-sitter processor available, use regex fallback
            let new_content = self.remove_comments(content, rules)?;
            let comments_removed = if new_content != content {
                self.count_removed_comments(content, &new_content)
            } else {
                0
            };
            Ok((new_content, comments_removed))
        }
    }

    fn count_removed_comments(&self, original: &str, modified: &str) -> u32 {
        // Simple heuristic: count lines that were removed
        let original_lines = original.lines().count();
        let modified_lines = modified.lines().count();

        (original_lines.saturating_sub(modified_lines)) as u32
    }
}

#[derive(Debug, Default)]
pub struct ProcessingResult {
    pub files_changed: Vec<PathBuf>,
    pub comments_removed: u32,
    pub errors: u32,
}

#[derive(Debug)]
pub struct FileProcessingResult {
    pub comments_removed: u32,
    pub has_changes: bool,
}

impl FileProcessingResult {
    pub fn has_changes(&self) -> bool {
        self.has_changes
    }
}