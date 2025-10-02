use crate::tool::config::LanguageSpecificRules;
use anyhow::{Result, anyhow};
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};
use tree_sitter_highlight::HighlightConfiguration;

pub struct TreeSitterProcessor {
    parser: Parser,
    languages: Vec<SupportedLanguage>,
}

pub struct SupportedLanguage {
    name: String,
    file_extensions: Vec<String>,
    language: Language,
    comment_query: Option<Query>,
    #[allow(dead_code)]
    highlight_config: Option<HighlightConfiguration>,
}

impl TreeSitterProcessor {
    pub fn new() -> Result<Self> {
        let parser = Parser::new();
        let languages = Self::init_supported_languages()?;

        Ok(Self { parser, languages })
    }

    fn init_supported_languages() -> Result<Vec<SupportedLanguage>> {
        let languages = vec![
            // Python
            SupportedLanguage {
                name: "python".to_string(),
                file_extensions: vec!["py".to_string()],
                language: tree_sitter_python::language(),
                comment_query: Self::create_python_comment_query()?,
                highlight_config: None,
            },
            // JavaScript
            SupportedLanguage {
                name: "javascript".to_string(),
                file_extensions: vec!["js".to_string(), "jsx".to_string()],
                language: tree_sitter_javascript::language(),
                comment_query: Self::create_javascript_comment_query()?,
                highlight_config: None,
            },
            // Rust
            SupportedLanguage {
                name: "rust".to_string(),
                file_extensions: vec!["rs".to_string()],
                language: tree_sitter_rust::language(),
                comment_query: Self::create_rust_comment_query()?,
                highlight_config: None,
            },
            // Go
            SupportedLanguage {
                name: "go".to_string(),
                file_extensions: vec!["go".to_string()],
                language: tree_sitter_go::language(),
                comment_query: Self::create_go_comment_query()?,
                highlight_config: None,
            },
        ];

        Ok(languages)
    }

    fn create_python_comment_query() -> Result<Option<Query>> {
        let query_str = r#"
(comment) @comment
        "#;

        match Query::new(tree_sitter_python::language(), query_str) {
            Ok(query) => Ok(Some(query)),
            Err(_) => Ok(None),
        }
    }

    fn create_javascript_comment_query() -> Result<Option<Query>> {
        let query_str = r#"
(comment) @comment
        "#;

        match Query::new(tree_sitter_javascript::language(), query_str) {
            Ok(query) => Ok(Some(query)),
            Err(_) => Ok(None),
        }
    }

    fn create_rust_comment_query() -> Result<Option<Query>> {
        let query_str = r#"
(comment) @comment
        "#;

        match Query::new(tree_sitter_rust::language(), query_str) {
            Ok(query) => Ok(Some(query)),
            Err(_) => Ok(None),
        }
    }

    fn create_go_comment_query() -> Result<Option<Query>> {
        let query_str = r#"
(comment) @comment
        "#;

        match Query::new(tree_sitter_go::language(), query_str) {
            Ok(query) => Ok(Some(query)),
            Err(_) => Ok(None),
        }
    }

    pub fn get_language_for_file(&self, file_path: &Path) -> Option<&SupportedLanguage> {
        let extension = file_path.extension()?.to_str()?;

        self.languages
            .iter()
            .find(|&lang| lang.file_extensions.contains(&extension.to_string()))
    }

    pub fn extract_comments(
        &mut self,
        content: &str,
        file_path: &Path,
    ) -> Result<Vec<CommentInfo>> {
        let (language, has_query, lang_name) = match self.get_language_for_file(file_path) {
            Some(lang) => (
                lang.language,
                lang.comment_query.is_some(),
                lang.name.clone(),
            ),
            None => return Ok(Vec::new()),
        };

        self.parser
            .set_language(language)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;

        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse content"))?;

        let mut comments = Vec::new();

        if has_query {
            if let Some(lang) = self.get_language_for_file(file_path)
                && let Some(query) = &lang.comment_query
            {
                let mut cursor = QueryCursor::new();
                let matches = cursor.matches(query, tree.root_node(), content.as_bytes());

                for mat in matches {
                    for capture in mat.captures {
                        let node = capture.node;
                        let comment_text = &content[node.byte_range()];

                        comments.push(CommentInfo {
                            text: comment_text.to_string(),
                            start_line: node.start_position().row + 1,
                            start_col: node.start_position().column,
                            end_line: node.end_position().row + 1,
                            end_col: node.end_position().column,
                            kind: self.classify_comment(node, &lang.name),
                        });
                    }
                }
            }
        } else {
            // Fallback to manual node traversal
            self.extract_comments_fallback(tree.root_node(), content, &mut comments, &lang_name);
        }

        Ok(comments)
    }

    fn extract_comments_fallback(
        &self,
        node: Node,
        content: &str,
        comments: &mut Vec<CommentInfo>,
        lang_name: &str,
    ) {
        if node.kind().contains("comment") {
            let comment_text = &content[node.byte_range()];
            comments.push(CommentInfo {
                text: comment_text.to_string(),
                start_line: node.start_position().row + 1,
                start_col: node.start_position().column,
                end_line: node.end_position().row + 1,
                end_col: node.end_position().column,
                kind: self.classify_comment(node, lang_name),
            });
        }

        for child in node.children(&mut node.walk()) {
            self.extract_comments_fallback(child, content, comments, lang_name);
        }
    }

    fn classify_comment(&self, node: Node, lang_name: &str) -> CommentKind {
        let node_kind = node.kind();

        match lang_name {
            "python" => {
                if node_kind == "comment" {
                    CommentKind::Line
                } else {
                    CommentKind::Unknown
                }
            }
            "javascript" => match node_kind {
                "comment" => CommentKind::Line,
                "block_comment" | "multiline_comment" => CommentKind::Block,
                _ => CommentKind::Unknown,
            },
            "rust" => match node_kind {
                "line_comment" => CommentKind::Line,
                "block_comment" => CommentKind::Block,
                "doc_comment" => CommentKind::Doc,
                _ => CommentKind::Unknown,
            },
            "go" => match node_kind {
                "comment" => CommentKind::Line,
                "block_comment" => CommentKind::Block,
                _ => CommentKind::Unknown,
            },
            _ => CommentKind::Unknown,
        }
    }

    pub fn should_remove_comment(
        &self,
        comment: &CommentInfo,
        rules: &LanguageSpecificRules,
    ) -> bool {
        // Check if comments are enabled for this type
        match comment.kind {
            CommentKind::Line => {
                if rules.single_line_comments != Some(true) {
                    return false;
                }
            }
            CommentKind::Block => {
                if rules.multi_line_comments != Some(true) {
                    return false;
                }
            }
            CommentKind::Doc => {
                match rules
                    .docstrings
                    .or(rules.jsdoc.or(rules.doc_comments.or(rules.godoc)))
                {
                    Some(true) => return false,
                    Some(false) => {}
                    None => return false,
                }
            }
            CommentKind::Unknown => return false,
        }

        // Check minimum length - remove comments that are too short
        if let Some(min_length) = rules.min_comment_length {
            let text_length = comment.text.trim().len();
            if text_length >= min_length {
                return false; // Don't remove long comments
            }
        }

        // Check preservation patterns
        if let Some(patterns) = &rules.preserve_patterns {
            for pattern in patterns {
                if let Ok(regex) = regex::Regex::new(pattern)
                    && regex.is_match(&comment.text)
                {
                    return false;
                }
            }
        }

        true
    }

    pub fn remove_comments_from_content(
        &mut self,
        content: &str,
        file_path: &Path,
        rules: &LanguageSpecificRules,
    ) -> Result<(String, Vec<CommentInfo>)> {
        let comments = self.extract_comments(content, file_path)?;
        let mut removed_comments = Vec::new();
        let mut result = content.to_string();

        // Process comments in reverse order to maintain correct positions
        let mut comments_to_remove: Vec<_> = comments
            .iter()
            .filter(|c| self.should_remove_comment(c, rules))
            .collect();

        // Sort by start position (descending)
        comments_to_remove
            .sort_by(|a, b| (b.start_line, b.start_col).cmp(&(a.start_line, a.start_col)));

        for comment in comments_to_remove {
            removed_comments.push(comment.clone());

            // Find the exact text in the source and remove it
            // This is a simple implementation - in practice, we'd want to be more careful
            // about whitespace and line breaks
            if let Some(range) = self.find_comment_range(&result, comment) {
                result.replace_range(range, "");
            }
        }

        Ok((result, removed_comments))
    }

    fn find_comment_range(
        &self,
        content: &str,
        comment: &CommentInfo,
    ) -> Option<std::ops::Range<usize>> {
        // This is a simplified approach - find the comment text near the expected line
        let lines: Vec<&str> = content.lines().collect();

        if comment.start_line <= lines.len() {
            let line = lines[comment.start_line - 1];
            let start_pos = comment.start_col.min(line.len());

            let trimmed_text = comment.text.trim();
            if let Some(pos) = line[start_pos..].find(trimmed_text) {
                let line_start = content[..content
                    .lines()
                    .take(comment.start_line - 1)
                    .map(|l| l.len() + 1)
                    .sum::<usize>()]
                    .len();
                let abs_start = line_start + start_pos + pos;
                let abs_end = abs_start + trimmed_text.len();

                return Some(abs_start..abs_end);
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct CommentInfo {
    pub text: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub kind: CommentKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommentKind {
    Line,
    Block,
    Doc,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_for_file() {
        let processor = TreeSitterProcessor::new().unwrap();

        assert!(
            processor
                .get_language_for_file(Path::new("test.py"))
                .is_some()
        );
        assert!(
            processor
                .get_language_for_file(Path::new("test.js"))
                .is_some()
        );
        assert!(
            processor
                .get_language_for_file(Path::new("test.rs"))
                .is_some()
        );
        assert!(
            processor
                .get_language_for_file(Path::new("test.go"))
                .is_some()
        );
        assert!(
            processor
                .get_language_for_file(Path::new("test.unknown"))
                .is_none()
        );
    }

    #[test]
    fn test_extract_python_comments() {
        let mut processor = TreeSitterProcessor::new().unwrap();
        let content = r#"
# This is a comment
def hello():
    # Another comment
    pass
"#;

        let comments = processor
            .extract_comments(content, Path::new("test.py"))
            .unwrap();
        assert_eq!(comments.len(), 2);
        assert!(comments[0].text.contains("This is a comment"));
        assert!(comments[1].text.contains("Another comment"));
    }

    #[test]
    fn test_extract_javascript_comments() {
        let mut processor = TreeSitterProcessor::new().unwrap();
        let content = r#"
// Line comment
function hello() {
    /* Block comment */
    return "hello";
}
"#;

        let comments = processor
            .extract_comments(content, Path::new("test.js"))
            .unwrap();
        assert_eq!(comments.len(), 2);
        assert!(comments[0].text.contains("Line comment"));
        assert!(comments[1].text.contains("Block comment"));
    }
}
