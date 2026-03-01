use anyhow::{Result, anyhow};
use llm_json::{RepairOptions, loads};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn parse_ison_document<T>(content: &str, context: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let payload = extract_ison_payload(content, context)?;
    parse_ison_payload(&payload, context)
}

pub fn parse_ison_payload<T>(payload: &str, context: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let trimmed = payload.trim_start();
    if looks_like_table_object_ison(trimmed) {
        return Err(anyhow!(
            "{context}: detected canonical table/object ISON payload. \
`llman sdd-legacy` expects legacy JSON inside ```ison```; use `llman sdd ...` for the canonical table/object ISON workflow.",
        ));
    }

    match serde_json::from_str::<T>(payload) {
        Ok(value) => Ok(value),
        Err(json_err) => {
            let repaired = loads(payload, &RepairOptions::default()).map_err(|repair_err| {
                anyhow!(
                    "{context}: failed to parse ISON payload as JSON ({json_err}); repair also failed ({repair_err})"
                )
            })?;
            serde_json::from_value(repaired)
                .map_err(|err| anyhow!("{context}: invalid ISON structure: {err}"))
        }
    }
}

fn looks_like_table_object_ison(payload: &str) -> bool {
    let first = payload.lines().find(|line| !line.trim().is_empty());
    let Some(first) = first else {
        return false;
    };
    let header = first.trim();
    header.starts_with("object.") || header.starts_with("table.")
}

pub fn extract_ison_payload(content: &str, context: &str) -> Result<String> {
    let normalized = normalize_newlines(content);
    let lines: Vec<&str> = normalized.lines().collect();

    let mut start: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("```") {
            continue;
        }

        let lang = trimmed.trim_start_matches('`').trim().to_lowercase();
        if lang == "ison" {
            start = Some(idx + 1);
            break;
        }
    }

    let start = start.ok_or_else(|| anyhow!("{context}: missing ```ison code block"))?;
    let mut end = None;
    for (idx, line) in lines.iter().enumerate().skip(start) {
        if line.trim().starts_with("```") {
            end = Some(idx);
            break;
        }
    }

    let end = end.ok_or_else(|| anyhow!("{context}: unterminated ```ison code block"))?;
    let payload = lines[start..end].join("\n").trim().to_string();
    if payload.is_empty() {
        return Err(anyhow!("{context}: empty ISON payload"));
    }

    Ok(payload)
}

pub fn render_ison_code_block<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let json = serde_json::to_string_pretty(value)?;
    Ok(format!("```ison\n{json}\n```\n"))
}

pub fn split_frontmatter(content: &str) -> (Option<String>, String) {
    let normalized = normalize_newlines(content);
    if !normalized.starts_with("---\n") {
        return (None, normalized);
    }

    let mut lines = normalized.lines();
    lines.next();

    let mut yaml_lines = Vec::new();
    let mut reached_end = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            reached_end = true;
            break;
        }
        yaml_lines.push(line.to_string());
    }

    if !reached_end {
        return (None, normalized);
    }

    let body = lines.collect::<Vec<_>>().join("\n");
    (Some(yaml_lines.join("\n")), body)
}

pub fn compose_with_frontmatter(frontmatter_yaml: Option<&str>, body: &str) -> String {
    let body = body.trim_start_matches('\n');
    match frontmatter_yaml {
        Some(yaml) => {
            let yaml = yaml.trim();
            if body.trim().is_empty() {
                format!("---\n{yaml}\n---\n")
            } else {
                format!("---\n{yaml}\n---\n\n{body}")
            }
        }
        None => body.to_string(),
    }
}

pub fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}
