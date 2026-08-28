use crate::fs_utils::atomic_write_with_mode;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub const LLMAN_PROMPTS_MARKER_START: &str = "<!-- LLMAN-PROMPTS:START -->";
pub const LLMAN_PROMPTS_MARKER_END: &str = "<!-- LLMAN-PROMPTS:END -->";

pub fn has_markers(content: &str, start_marker: &str, end_marker: &str) -> bool {
    content.lines().any(|line| line.trim() == start_marker)
        && content.lines().any(|line| line.trim() == end_marker)
}

pub fn has_llman_prompt_markers(content: &str) -> bool {
    has_markers(
        content,
        LLMAN_PROMPTS_MARKER_START,
        LLMAN_PROMPTS_MARKER_END,
    )
}

fn is_marker_on_own_line(content: &str, marker_index: usize, marker_len: usize) -> bool {
    let bytes = content.as_bytes();
    let mut left = marker_index as isize - 1;
    while left >= 0 {
        let ch = bytes[left as usize] as char;
        if ch == '\n' {
            break;
        }
        if ch != ' ' && ch != '\t' && ch != '\r' {
            return false;
        }
        left -= 1;
    }

    let mut right = marker_index + marker_len;
    while right < bytes.len() {
        let ch = bytes[right] as char;
        if ch == '\n' {
            break;
        }
        if ch != ' ' && ch != '\t' && ch != '\r' {
            return false;
        }
        right += 1;
    }

    true
}

fn find_marker_index(content: &str, marker: &str, from_index: usize) -> Option<usize> {
    let mut search_index = from_index;
    while let Some(pos) = content[search_index..].find(marker) {
        let idx = search_index + pos;
        if is_marker_on_own_line(content, idx, marker.len()) {
            return Some(idx);
        }
        search_index = idx + marker.len();
        if search_index >= content.len() {
            break;
        }
    }
    None
}

pub fn update_file_with_markers(
    path: &Path,
    body: &str,
    start_marker: &str,
    end_marker: &str,
) -> Result<()> {
    let mut content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if !content.is_empty() {
        let start_index = find_marker_index(&content, start_marker, 0);
        let end_index = start_index
            .and_then(|start| find_marker_index(&content, end_marker, start + start_marker.len()))
            .or_else(|| find_marker_index(&content, end_marker, 0));

        match (start_index, end_index) {
            (Some(start), Some(end)) => {
                if end < start {
                    return Err(anyhow!(
                        "Invalid marker state in {}. End marker appears before start marker.",
                        path.display()
                    ));
                }
                let before = &content[..start];
                let after = &content[end + end_marker.len()..];
                content = format!("{before}{start_marker}\n{body}\n{end_marker}{after}");
            }
            (None, None) => {
                content = format!("{start_marker}\n{body}\n{end_marker}\n\n{content}");
            }
            _ => {
                return Err(anyhow!(
                    "Invalid marker state in {}. Found start: {}, Found end: {}",
                    path.display(),
                    start_index.is_some(),
                    end_index.is_some()
                ));
            }
        }
    } else {
        content = format!("{start_marker}\n{body}\n{end_marker}");
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    atomic_write_with_mode(path, content.as_bytes(), None)?;
    Ok(())
}

pub fn update_text_with_markers(
    existing: &str,
    body: &str,
    append_when_missing: bool,
    start_marker: &str,
    end_marker: &str,
) -> String {
    let body = body.trim_end();

    let mut start_idx: Option<usize> = None;
    let mut end_idx: Option<usize> = None;

    // Find markers on their own line (trimmed match), tracking byte indices.
    let mut cursor = 0usize;
    for line in existing.split_inclusive('\n') {
        let line_start = cursor;
        let line_end = cursor + line.len();
        let trimmed = line.trim_matches(['\r', '\n']);
        if trimmed.trim() == start_marker {
            start_idx = Some(line_start);
        } else if trimmed.trim() == end_marker {
            end_idx = Some(line_end);
            break;
        }
        cursor = line_end;
    }

    match (start_idx, end_idx) {
        (Some(start), Some(end)) => {
            let before = &existing[..start];
            let after = &existing[end..];
            let mut out = String::new();
            out.push_str(before);
            out.push_str(start_marker);
            out.push('\n');
            out.push_str(body);
            out.push('\n');
            out.push_str(end_marker);
            out.push('\n');
            out.push_str(after);
            out
        }
        _ if append_when_missing => {
            let mut out = existing.to_string();
            if !out.ends_with('\n') && !out.is_empty() {
                out.push('\n');
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(start_marker);
            out.push('\n');
            out.push_str(body);
            out.push('\n');
            out.push_str(end_marker);
            out.push('\n');
            out
        }
        _ => existing.to_string(),
    }
}
