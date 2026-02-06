use crate::tool::command::CleanUselessCommentsArgs;
use crate::tool::config::Config;
use crate::tool::tree_sitter_processor::TreeSitterProcessor;
use anyhow::{Result, anyhow};
use glob::Pattern;
use ignore::WalkBuilder;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct CompiledScopePatterns {
    include: Vec<Pattern>,
    exclude: Vec<Pattern>,
}

impl CompiledScopePatterns {
    fn new(scope: &crate::tool::config::ScopeConfig) -> Self {
        Self {
            include: scope
                .include
                .iter()
                .filter_map(|pattern| Pattern::new(pattern).ok())
                .collect(),
            exclude: scope
                .exclude
                .iter()
                .filter_map(|pattern| Pattern::new(pattern).ok())
                .collect(),
        }
    }

    fn matches(&self, path: &Path) -> bool {
        if self
            .exclude
            .iter()
            .any(|pattern| pattern.matches_path(path))
        {
            return false;
        }

        self.include
            .iter()
            .any(|pattern| pattern.matches_path(path))
    }
}

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
                eprintln!(
                    "{}",
                    t!(
                        "tool.clean_comments.processor.init_tree_sitter_failed",
                        error = e
                    )
                );
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
            .ok_or_else(|| anyhow!(t!("tool.clean_comments.processor.config_missing")))?;

        println!("{}", t!("tool.clean_comments.processor.processing_files"));
        let effective_dry_run = self.effective_dry_run();

        // Clone the clean_config to avoid borrow issues
        let clean_config_clone = clean_config.clone();
        let compiled_scope = CompiledScopePatterns::new(&clean_config_clone.scope);
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
        let files_to_process =
            self.find_files(&compiled_scope, git_only, tracked_files.as_ref(), &cwd)?;
        let mut results = ProcessingResult::default();

        for file_path in files_to_process {
            if self.args.verbose {
                println!(
                    "{}",
                    t!(
                        "tool.clean_comments.processor.processing_file",
                        path = file_path.display()
                    )
                );
            }

            match self.process_file(&file_path, &clean_config_clone) {
                Ok(file_result) => {
                    if file_result.has_changes() {
                        results.files_changed.push(file_path.clone());
                        results.comments_removed += file_result.comments_removed;

                        if self.args.verbose || !effective_dry_run {
                            println!(
                                "{}",
                                t!(
                                    "tool.clean_comments.processor.file_removed_comments",
                                    path = file_path.display(),
                                    count = file_result.comments_removed
                                )
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        t!(
                            "tool.clean_comments.processor.error_processing",
                            path = file_path.display(),
                            error = e
                        )
                    );
                    results.errors += 1;
                }
            }
        }

        Ok(results)
    }

    fn find_files(
        &self,
        scope: &CompiledScopePatterns,
        git_only: bool,
        tracked_files: Option<&HashSet<PathBuf>>,
        cwd: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if git_only && tracked_files.is_none() {
            eprintln!("{}", t!("tool.clean_comments.processor.git_only_no_repo"));
            return Ok(files);
        }

        // If specific files are provided, use them
        if !self.args.files.is_empty() {
            for file in &self.args.files {
                if file.exists() {
                    if file.is_file() {
                        if !git_only
                            || tracked_files
                                .map(|tracked| self.is_tracked(file, cwd, tracked))
                                .unwrap_or(false)
                        {
                            files.push(file.clone());
                        } else {
                            eprintln!(
                                "{}",
                                t!(
                                    "tool.clean_comments.processor.file_not_tracked",
                                    path = file.display()
                                )
                            );
                        }
                    } else {
                        eprintln!(
                            "{}",
                            t!(
                                "tool.clean_comments.processor.path_skipped_not_file",
                                path = file.display()
                            )
                        );
                    }
                } else {
                    eprintln!(
                        "{}",
                        t!(
                            "tool.clean_comments.processor.file_not_found",
                            path = file.display()
                        )
                    );
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
                        if scope.matches(path)
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
                    eprintln!(
                        "{}",
                        t!("tool.clean_comments.processor.walk_error", error = e)
                    );
                }
            }
        }

        Ok(files)
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
            Some("tsx") => Some("typescript"),
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
            println!(
                "{}",
                t!(
                    "tool.clean_comments.processor.preserve_patterns_found",
                    count = preserve_regexes.len()
                )
            );
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
                    println!(
                        "{}",
                        t!("tool.clean_comments.processor.debug_line", line = line)
                    );
                    println!(
                        "{}",
                        t!(
                            "tool.clean_comments.processor.debug_comment",
                            comment = comment.trim()
                        )
                    );
                    println!(
                        "{}",
                        t!(
                            "tool.clean_comments.processor.debug_should_preserve",
                            preserve = should_preserve
                        )
                    );
                    println!(
                        "{}",
                        t!(
                            "tool.clean_comments.processor.debug_comment_too_short",
                            too_short = comment_too_short,
                            len = comment.trim().len(),
                            min = min_length
                        )
                    );
                    println!(
                        "{}",
                        t!(
                            "tool.clean_comments.processor.debug_would_remove",
                            remove = should_remove
                        )
                    );
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

    #[allow(dead_code)]
    fn remove_multi_line_comments(
        &self,
        content: &str,
        _rules: &crate::tool::config::LanguageSpecificRules,
    ) -> Result<String> {
        // Simple multi-line comment removal (basic implementation)
        // This is a placeholder - a proper implementation would need more sophisticated parsing
        let mut result = content.to_string();

        // Remove C-style multi-line comments /* ... */
        let re = Regex::new(r"/\*.*?\*/").map_err(|e| {
            anyhow!(t!(
                "tool.clean_comments.processor.regex_build_failed",
                error = e
            ))
        })?;
        result = re.replace_all(&result, "").to_string();

        Ok(result)
    }

    fn remove_comments_with_tree_sitter(
        &mut self,
        content: &str,
        file_path: &Path,
        rules: &crate::tool::config::LanguageSpecificRules,
    ) -> Result<(String, u32)> {
        let tree_sitter_processor = self.tree_sitter_processor.as_mut().ok_or_else(|| {
            anyhow!(t!(
                "tool.clean_comments.processor.tree_sitter_unavailable",
                path = file_path.display()
            ))
        })?;

        match tree_sitter_processor.remove_comments_from_content(content, file_path, rules) {
            Ok((new_content, removed_comments)) => {
                let comments_removed = removed_comments.len() as u32;
                Ok((new_content, comments_removed))
            }
            Err(e) => Err(anyhow!(t!(
                "tool.clean_comments.processor.tree_sitter_failed",
                path = file_path.display(),
                error = e
            ))),
        }
    }

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::CommentProcessor;
    use crate::tool::command::CleanUselessCommentsArgs;
    use crate::tool::config::{
        CleanUselessCommentsConfig, Config, LanguageRules, LanguageSpecificRules, ScopeConfig,
        ToolsConfig,
    };
    use tempfile::TempDir;

    #[test]
    fn test_tree_sitter_unavailable_does_not_modify_files() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path();
        let file_path = work_dir.join("test.py");
        let original = "# short\n\ndef test():\n    pass\n";
        std::fs::write(&file_path, original).unwrap();

        let config = Config {
            version: "0.1".to_string(),
            tools: ToolsConfig {
                rm_useless_dirs: None,
                clean_useless_comments: Some(CleanUselessCommentsConfig {
                    scope: ScopeConfig {
                        include: vec!["**/*.py".to_string()],
                        exclude: Vec::new(),
                    },
                    lang_rules: LanguageRules {
                        python: Some(LanguageSpecificRules {
                            single_line_comments: Some(true),
                            min_comment_length: Some(100),
                            ..Default::default()
                        }),
                        javascript: None,
                        typescript: None,
                        rust: None,
                        go: None,
                    },
                    global_rules: None,
                    safety: None,
                    output: None,
                }),
            },
        };

        let args = CleanUselessCommentsArgs {
            config: None,
            dry_run: false,
            yes: true,
            interactive: false,
            force: true,
            verbose: false,
            git_only: false,
            files: vec![file_path.clone()],
        };

        let mut processor = CommentProcessor {
            config,
            args,
            tree_sitter_processor: None,
        };
        let result = processor.process().unwrap();
        assert_eq!(result.errors, 1);
        assert!(result.files_changed.is_empty());

        let actual = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(actual, original);
    }
}
