//! `llman tool comments overview` — leading file-head comment TSV.

use crate::path_utils::relative_path_from_dir;
use crate::tool::command::CommentsOverviewArgs;
use anyhow::{Result, anyhow, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use rust_i18n::t;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const HEADER_READ_LIMIT: usize = 64 * 1024;

enum HeaderMode {
    Line { prefix: &'static str },
    BlockC { opened: bool },
    Docstring { quotes: &'static str, opened: bool },
}

/// Extract the first paragraph of a leading file-head comment.
pub fn extract_header(source: &str, basename: &str) -> Option<String> {
    let mut mode: Option<HeaderMode> = None;
    let mut header = String::new();
    let mut first = true;

    for raw in source.lines() {
        let line = raw.trim_start_matches([' ', '\t']);
        if first {
            first = false;
            if line.starts_with("#!") {
                continue;
            }
        }

        if mode.is_none() {
            if line.is_empty() {
                break;
            }
            mode = detect_mode(line);
            if mode.is_none() {
                break;
            }
        }

        match process_line(mode.as_mut().expect("mode set"), line, &mut header) {
            LineAction::Continue => {}
            LineAction::Stop => break,
        }
    }

    finalize_header(header, basename)
}

fn detect_mode(line: &str) -> Option<HeaderMode> {
    if line.starts_with('#') {
        Some(HeaderMode::Line { prefix: "#" })
    } else if line.starts_with("//") {
        Some(HeaderMode::Line { prefix: "//" })
    } else if line.starts_with("--") {
        Some(HeaderMode::Line { prefix: "--" })
    } else if line.starts_with("/*") {
        Some(HeaderMode::BlockC { opened: false })
    } else if line.starts_with("\"\"\"") {
        Some(HeaderMode::Docstring {
            quotes: "\"\"\"",
            opened: false,
        })
    } else if line.starts_with("'''") {
        Some(HeaderMode::Docstring {
            quotes: "'''",
            opened: false,
        })
    } else {
        None
    }
}

enum LineAction {
    Continue,
    Stop,
}

fn process_line(mode: &mut HeaderMode, line: &str, header: &mut String) -> LineAction {
    match mode {
        HeaderMode::Line { prefix } => {
            if line.is_empty() || !line.starts_with(*prefix) {
                return LineAction::Stop;
            }
            let rest = line[prefix.len()..].trim_start_matches([' ', '\t']);
            if is_slash_only(rest) {
                return LineAction::Continue;
            }
            let rest = rest.trim_start_matches('/').trim_start_matches([' ', '\t']);
            push_fragment(header, rest);
            LineAction::Continue
        }
        HeaderMode::BlockC { opened } => {
            let mut rest = if !*opened {
                *opened = true;
                line.trim_start_matches("/*")
                    .trim_start_matches([' ', '\t'])
                    .to_string()
            } else {
                line.to_string()
            };
            if let Some(idx) = rest.find("*/") {
                rest.truncate(idx);
                let rest = strip_block_decor(&rest);
                push_fragment(header, rest);
                return LineAction::Stop;
            }
            let rest = strip_block_decor(&rest);
            if rest.is_empty() {
                return LineAction::Continue;
            }
            push_fragment(header, rest);
            LineAction::Continue
        }
        HeaderMode::Docstring { quotes, opened } => {
            if line.is_empty() {
                return LineAction::Stop;
            }
            let rest = if !*opened {
                *opened = true;
                line.get(quotes.len()..)
                    .unwrap_or("")
                    .trim_start_matches([' ', '\t'])
            } else {
                line
            };
            if let Some(idx) = rest.find(*quotes) {
                push_fragment(header, &rest[..idx]);
                return LineAction::Stop;
            }
            push_fragment(header, rest);
            LineAction::Continue
        }
    }
}

fn is_slash_only(s: &str) -> bool {
    let trimmed = s.trim_end_matches([' ', '\t']);
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '/')
}

fn strip_block_decor(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
        .trim_start_matches(['*', '!'])
        .trim_start_matches([' ', '\t'])
}

fn push_fragment(header: &mut String, fragment: &str) {
    let fragment = fragment.trim_end_matches([' ', '\t']);
    if fragment.is_empty() {
        return;
    }
    if !header.is_empty() {
        header.push(' ');
    }
    header.push_str(fragment);
}

fn finalize_header(header: String, basename: &str) -> Option<String> {
    if header.is_empty() {
        return None;
    }
    let mut text = header;
    if let Some(rest) = text.strip_prefix(basename) {
        let rest = rest.trim_start_matches([' ', '\t']);
        let rest = rest.trim_start_matches(['-', ':', '=', '|']);
        text = rest.trim_start_matches([' ', '\t']).to_string();
    }
    let text = text.trim_end_matches([' ', '\t']);
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub fn run_overview(args: &CommentsOverviewArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut files: BTreeMap<String, PathBuf> = BTreeMap::new();

    for target in &args.targets {
        collect_target(&cwd, target, &mut files)?;
    }

    for (rel, path) in files {
        if let Some(desc) = header_for_path(&path) {
            println!("{rel}\t{desc}");
        }
    }

    Ok(())
}

fn collect_target(cwd: &Path, target: &str, files: &mut BTreeMap<String, PathBuf>) -> Result<()> {
    let candidate = {
        let p = Path::new(target);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };

    if candidate.is_file() {
        insert_file(cwd, candidate, files);
        return Ok(());
    }
    if candidate.is_dir() {
        for path in walk_files(&candidate) {
            insert_file(cwd, path, files);
        }
        return Ok(());
    }
    if looks_like_glob(target) {
        let matched = glob_files(cwd, target)?;
        if matched.is_empty() {
            bail!("{}", t!("tool.comments.overview.no_match", target = target));
        }
        for path in matched {
            insert_file(cwd, path, files);
        }
        return Ok(());
    }

    Err(anyhow!(
        "{}",
        t!("tool.comments.overview.no_match", target = target)
    ))
}

fn looks_like_glob(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '*' | '?' | '[' | '{'))
}

fn insert_file(cwd: &Path, path: PathBuf, files: &mut BTreeMap<String, PathBuf>) {
    let rel = display_rel(cwd, &path);
    files.entry(rel).or_insert(path);
}

fn display_rel(cwd: &Path, path: &Path) -> String {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let cwd_abs = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    relative_path_from_dir(&cwd_abs, &abs)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| path.display().to_string())
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .follow_links(false)
        .require_git(false)
        .build();
    let mut out = Vec::new();
    for entry in walker {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            out.push(entry.into_path());
        }
    }
    out
}

fn glob_files(cwd: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let set = compile_glob(pattern)?;
    let mut out = Vec::new();
    for path in walk_files(cwd) {
        let rel = display_rel(cwd, &path);
        if set.is_match(&rel) || path.file_name().is_some_and(|name| set.is_match(name)) {
            out.push(path);
        }
    }
    Ok(out)
}

fn compile_glob(pattern: &str) -> Result<GlobSet> {
    let glob = Glob::new(pattern).map_err(|error| {
        anyhow!(t!(
            "tool.comments.overview.invalid_glob",
            target = pattern,
            error = error.to_string()
        ))
    })?;
    let mut builder = GlobSetBuilder::new();
    builder.add(glob);
    builder.build().map_err(|error| {
        anyhow!(t!(
            "tool.comments.overview.invalid_glob",
            target = pattern,
            error = error.to_string()
        ))
    })
}

fn header_for_path(path: &Path) -> Option<String> {
    let bytes = read_header_bytes(path)?;
    if bytes.contains(&0) {
        return None;
    }
    let source = std::str::from_utf8(&bytes).ok()?;
    let basename = path.file_name()?.to_str()?;
    extract_header(source, basename)
}

fn read_header_bytes(path: &Path) -> Option<Vec<u8>> {
    let mut data = fs::read(path).ok()?;
    if data.len() > HEADER_READ_LIMIT {
        data.truncate(HEADER_READ_LIMIT);
    }
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_comment_first_paragraph() {
        let src = "# hello world\n# still first\n\n# second\ncode()\n";
        assert_eq!(
            extract_header(src, "a.py").as_deref(),
            Some("hello world still first")
        );
    }

    #[test]
    fn skips_shebang() {
        let src = "#!/usr/bin/env python3\n# i18n key audit.\n\nimport sys\n";
        assert_eq!(
            extract_header(src, "check.py").as_deref(),
            Some("i18n key audit.")
        );
    }

    #[test]
    fn slash_line_comments_and_extra_slashes() {
        let src = "/// module docs\n// more\nfn main() {}\n";
        assert_eq!(
            extract_header(src, "main.rs").as_deref(),
            Some("module docs more")
        );
    }

    #[test]
    fn lua_dash_comments() {
        let src = "-- pack helper\nprint(1)\n";
        assert_eq!(
            extract_header(src, "pack.lua").as_deref(),
            Some("pack helper")
        );
    }

    #[test]
    fn c_block_comment() {
        let src = "/* title\n * body\n */\nint x;\n";
        assert_eq!(extract_header(src, "a.c").as_deref(), Some("title body"));
    }

    #[test]
    fn python_docstring_first_paragraph() {
        let src = "\"\"\"Regenerate README.md.\n\nMore detail.\n\"\"\"\n";
        assert_eq!(
            extract_header(src, "gen_readme.py").as_deref(),
            Some("Regenerate README.md.")
        );
    }

    #[test]
    fn strips_basename_prefix() {
        let src = "# file.py: does a thing\n";
        assert_eq!(
            extract_header(src, "file.py").as_deref(),
            Some("does a thing")
        );
    }

    #[test]
    fn none_without_header() {
        assert_eq!(extract_header("print(1)\n", "a.py"), None);
        assert_eq!(extract_header("", "a.py"), None);
    }
}
