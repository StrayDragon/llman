use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

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
    fs::write(path, content)?;
    Ok(())
}
