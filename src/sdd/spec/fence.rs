use anyhow::{Result, anyhow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeFence {
    pub lang: String,
    pub payload: String,
    /// 1-based line number of the opening ``` fence.
    pub start_line: usize,
}

pub fn render_code_fence(lang: &str, payload: &str) -> String {
    let lang = lang.trim();
    let payload = payload.trim_end();
    format!("```{lang}\n{payload}\n```\n")
}

pub fn extract_all_code_fences(content: &str) -> Result<Vec<CodeFence>> {
    let normalized = normalize_newlines(content);
    let lines: Vec<&str> = normalized.lines().collect();

    let mut fences: Vec<CodeFence> = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if !trimmed.starts_with("```") {
            i += 1;
            continue;
        }

        let info = trimmed.trim_start_matches('`').trim();
        let lang = info
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if lang.is_empty() {
            i += 1;
            continue;
        }

        let payload_start = i + 1;
        let mut end: Option<usize> = None;
        let mut j = i + 1;
        while j < lines.len() {
            if lines[j].trim().starts_with("```") {
                end = Some(j);
                break;
            }
            j += 1;
        }
        let end = end.ok_or_else(|| {
            anyhow!(
                "unterminated ```{} code block (starting at line {})",
                lang,
                i + 1
            )
        })?;

        let payload = lines[payload_start..end].join("\n");
        fences.push(CodeFence {
            lang,
            payload,
            start_line: i + 1,
        });
        i = end + 1;
    }

    Ok(fences)
}

pub fn extract_single_fence_payload(content: &str, lang: &str) -> Result<CodeFence> {
    let lang = lang.trim().to_ascii_lowercase();
    let fences = extract_all_code_fences(content)?;
    let matches = fences
        .into_iter()
        .filter(|f| f.lang == lang)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(anyhow!("missing ```{} code block", lang));
    }
    if matches.len() != 1 {
        return Err(anyhow!(
            "expected exactly 1 ```{} code block, got {}",
            lang,
            matches.len()
        ));
    }
    Ok(matches
        .into_iter()
        .next()
        .expect("matches has exactly 1 element"))
}

pub fn replace_single_fence_payload(
    content: &str,
    lang: &str,
    new_payload: &str,
) -> Result<String> {
    let lang = lang.trim().to_ascii_lowercase();
    let normalized = normalize_newlines(content);
    let lines = normalized.lines().collect::<Vec<_>>();

    let mut open_idx: Option<usize> = None;
    let mut close_idx: Option<usize> = None;

    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if !trimmed.starts_with("```") {
            i += 1;
            continue;
        }

        let info = trimmed.trim_start_matches('`').trim();
        let fence_lang = info
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if fence_lang != lang {
            i += 1;
            continue;
        }

        if open_idx.is_some() {
            return Err(anyhow!(
                "expected exactly 1 ```{} code block, found another starting at line {}",
                lang,
                i + 1
            ));
        }

        open_idx = Some(i);
        let mut j = i + 1;
        while j < lines.len() {
            if lines[j].trim().starts_with("```") {
                close_idx = Some(j);
                break;
            }
            j += 1;
        }
        break;
    }

    let Some(open_idx) = open_idx else {
        return Err(anyhow!("missing ```{} code block", lang));
    };
    let Some(close_idx) = close_idx else {
        return Err(anyhow!(
            "unterminated ```{} code block (starting at line {})",
            lang,
            open_idx + 1
        ));
    };

    let new_payload = new_payload.trim_end_matches('\n');
    if new_payload.trim().is_empty() {
        return Err(anyhow!("new fenced payload must not be empty"));
    }
    let mut new_payload_lines = new_payload.split('\n').collect::<Vec<_>>();
    if new_payload_lines.len() == 1 && new_payload_lines[0].is_empty() {
        new_payload_lines.clear();
    }

    let mut out: Vec<String> = Vec::new();
    out.extend(lines[..=open_idx].iter().map(|line| (*line).to_string()));
    out.extend(new_payload_lines.iter().map(|line| (*line).to_string()));
    out.extend(lines[close_idx..].iter().map(|line| (*line).to_string()));

    let mut rebuilt = out.join("\n");
    rebuilt.push('\n');
    Ok(rebuilt)
}

pub fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}
