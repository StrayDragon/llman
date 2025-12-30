use crate::tool::command::CleanUselessCommentsArgs;
use crate::tool::config::Config;
use crate::tool::tree_sitter_processor::TreeSitterProcessor;
use anyhow::{Result, anyhow};
use ignore::WalkBuilder;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
                eprintln!("Warning: Failed to initialize TreeSitter processor: {e}");
                None
            }
        };

        Self {
            config,
            args,
            tree_sitter_processor,
        }
    }

    pub fn process(&mut self) -> Result<ProcessingResult> {
        let clean_config = self
            .config
            .get_clean_comments_config()
            .ok_or_else(|| anyhow!("No clean useless comments configuration found"))?;

        println!("Processing files...");
        let effective_dry_run = self.effective_dry_run();

        // Clone the clean_config to avoid borrow issues
        let clean_config_clone = clean_config.clone();
        let git_only = self.args.git_only
            || clean_config_clone
                .safety
                .as_ref()
                .and_then(|safety| safety.git_aware)
                .unwrap_or(false);
        let tracked_files = if git_only {
            self.load_git_tracked_files()?
        } else {
            None
        };
        let cwd = std::env::current_dir()?;
        let files_to_process = self.find_files(
            &clean_config_clone.scope,
            git_only,
            tracked_files.as_ref(),
            &cwd,
        )?;
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

                        if self.args.verbose || !effective_dry_run {
                            println!(
                                "File: {} - Removed {} comments",
                                file_path.display(),
                                file_result.comments_removed
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error processing {}: {e}", file_path.display());
                    results.errors += 1;
                }
            }
        }

        Ok(results)
    }

    fn find_files(
        &self,
        scope: &crate::tool::config::ScopeConfig,
        git_only: bool,
        tracked_files: Option<&HashSet<PathBuf>>,
        cwd: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if git_only && tracked_files.is_none() {
            eprintln!("Warning: git-only enabled but no git repository detected.");
            return Ok(files);
        }

        // If specific files are provided, use them
        if !self.args.files.is_empty() {
            for file in &self.args.files {
                if file.exists() {
                    if !git_only
                        || tracked_files
                            .map(|tracked| self.is_tracked(file, cwd, tracked))
                            .unwrap_or(false)
                    {
                        files.push(file.clone());
                    } else {
                        eprintln!("Warning: File not tracked by git: {}", file.display());
                    }
                } else {
                    eprintln!("Warning: File not found: {}", file.display());
                }
            }
            return Ok(files);
        }

        // Otherwise, walk the directory tree
        let walker = WalkBuilder::new(".");

        for result in walker.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        // Check if file matches include patterns
                        if self.matches_patterns(path, &scope.include, &scope.exclude)
                            && (!git_only
                                || tracked_files
                                    .map(|tracked| self.is_tracked(path, cwd, tracked))
                                    .unwrap_or(false))
                        {
                            files.push(path.to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error walking directory: {e}");
                }
            }
        }

        Ok(files)
    }

    fn matches_patterns(&self, path: &Path, include: &[String], exclude: &[String]) -> bool {
        let _path_str = path.to_string_lossy();

        // Check exclude patterns first
        for pattern in exclude {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches_path(path))
                .unwrap_or(false)
            {
                return false;
            }
        }

        // Check include patterns
        for pattern in include {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches_path(path))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    fn is_tracked(&self, path: &Path, cwd: &Path, tracked_files: &HashSet<PathBuf>) -> bool {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };

        match absolute.canonicalize() {
            Ok(normalized) => tracked_files.contains(&normalized),
            Err(_) => false,
        }
    }

    fn load_git_tracked_files(&self) -> Result<Option<HashSet<PathBuf>>> {
        let repo_root = match self.git_repo_root()? {
            Some(root) => root,
            None => return Ok(None),
        };

        let output = match Command::new("git")
            .args(["ls-files", "-z"])
            .current_dir(&repo_root)
            .output()
        {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };

        if !output.status.success() {
            return Ok(None);
        }

        let mut tracked = HashSet::new();
        for entry in output.stdout.split(|byte| *byte == 0) {
            if entry.is_empty() {
                continue;
            }
            let rel = String::from_utf8_lossy(entry);
            let candidate = repo_root.join(rel.as_ref());
            if let Ok(canonical) = candidate.canonicalize() {
                tracked.insert(canonical);
            }
        }

        Ok(Some(tracked))
    }

    fn git_repo_root(&self) -> Result<Option<PathBuf>> {
        let output = match Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };

        if !output.status.success() {
            return Ok(None);
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if root.is_empty() {
            return Ok(None);
        }

        Ok(Some(PathBuf::from(root)))
    }

    fn process_file(
        &mut self,
        file_path: &Path,
        clean_config: &crate::tool::config::CleanUselessCommentsConfig,
    ) -> Result<FileProcessingResult> {
        let content = fs::read_to_string(file_path)?;
        let language = self.detect_language(file_path);

        if let Some(lang_rules) = self.get_language_rules(language, &clean_config.lang_rules) {
            let (new_content, comments_removed) =
                self.remove_comments_with_tree_sitter(&content, file_path, lang_rules)?;

            let has_changes = new_content != content;

            if !self.effective_dry_run() && has_changes {
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

    fn get_language_rules<'a>(
        &self,
        language: Option<&str>,
        lang_rules: &'a crate::tool::config::LanguageRules,
    ) -> Option<&'a crate::tool::config::LanguageSpecificRules> {
        match language {
            Some("python") => lang_rules.python.as_ref(),
            Some("javascript") => lang_rules.javascript.as_ref(),
            Some("typescript") => lang_rules
                .typescript
                .as_ref()
                .or(lang_rules.javascript.as_ref()),
            Some("rust") => lang_rules.rust.as_ref(),
            Some("go") => lang_rules.go.as_ref(),
            _ => None,
        }
    }

    fn remove_comments(
        &self,
        content: &str,
        rules: &crate::tool::config::LanguageSpecificRules,
    ) -> Result<String> {
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

    fn remove_single_line_comments(
        &self,
        content: &str,
        rules: &crate::tool::config::LanguageSpecificRules,
    ) -> Result<String> {
        let empty_patterns = vec![];
        let preserve_patterns = rules.preserve_patterns.as_ref().unwrap_or(&empty_patterns);
        let preserve_regexes: Vec<_> = preserve_patterns
            .iter()
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
                let should_preserve = preserve_regexes
                    .iter()
                    .any(|regex| regex.is_match(comment.trim()));

                // Check minimum length requirement
                let min_length = rules.min_comment_length.unwrap_or(0);
                let comment_too_short = comment.trim().len() < min_length;

                // Check if the line is mostly comment (simple heuristic)
                let _mostly_comment = before_comment.trim().len() <= 10; // Simple threshold

                should_remove = !should_preserve && comment_too_short;

                if self.args.verbose {
                    println!("Processing line: '{line}'");
                    println!("  - comment: '{}'", comment.trim());
                    println!("  - should_preserve: {should_preserve}");
                    println!(
                        "  - comment_too_short: {} (len: {}, min: {})",
                        comment_too_short,
                        comment.trim().len(),
                        min_length
                    );
                    println!("  - would remove: {should_remove}");
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

    fn remove_multi_line_comments(
        &self,
        content: &str,
        _rules: &crate::tool::config::LanguageSpecificRules,
    ) -> Result<String> {
        // Simple multi-line comment removal (basic implementation)
        // This is a placeholder - a proper implementation would need more sophisticated parsing
        let mut result = content.to_string();

        // Remove C-style multi-line comments /* ... */
        let re = Regex::new(r"/\*.*?\*/").map_err(|e| anyhow!("Failed to build regex: {}", e))?;
        result = re.replace_all(&result, "").to_string();

        Ok(result)
    }

    fn remove_comments_with_tree_sitter(
        &mut self,
        content: &str,
        file_path: &Path,
        rules: &crate::tool::config::LanguageSpecificRules,
    ) -> Result<(String, u32)> {
        if let Some(ref mut tree_sitter_processor) = self.tree_sitter_processor {
            match tree_sitter_processor.remove_comments_from_content(content, file_path, rules) {
                Ok((new_content, removed_comments)) => {
                    let comments_removed = removed_comments.len() as u32;
                    Ok((new_content, comments_removed))
                }
                Err(e) => {
                    if self.args.verbose {
                        eprintln!(
                            "Tree-sitter processing failed for {}: {}, falling back to regex",
                            file_path.display(),
                            e
                        );
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

    fn effective_dry_run(&self) -> bool {
        self.args.dry_run || !self.args.yes
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
