use anyhow::{Result, anyhow};
use ison_rs::{Block, Document, ISONError, Row, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsonFence {
    pub payload: String,
    /// 1-based line number of the opening ```ison fence.
    pub start_line: usize,
}

pub fn extract_all_ison_fences(content: &str, context: &str) -> Result<Vec<IsonFence>> {
    let normalized = normalize_newlines(content);
    let lines: Vec<&str> = normalized.lines().collect();

    let mut fences: Vec<IsonFence> = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if !trimmed.starts_with("```") {
            i += 1;
            continue;
        }

        let lang = trimmed.trim_start_matches('`').trim().to_ascii_lowercase();
        if lang != "ison" {
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
        let end = end.ok_or_else(|| anyhow!("{context}: unterminated ```ison code block"))?;

        let payload = lines[payload_start..end].join("\n").trim().to_string();
        if payload.is_empty() {
            return Err(anyhow!(
                "{context}: empty ISON payload in ```ison code block starting at line {}",
                i + 1
            ));
        }

        fences.push(IsonFence {
            payload,
            start_line: i + 1,
        });
        i = end + 1;
    }

    if fences.is_empty() {
        return Err(anyhow!("{context}: missing ```ison code block"));
    }

    Ok(fences)
}

#[derive(Debug, Clone, Default)]
pub struct MergedIsonDocument {
    blocks: std::collections::HashMap<String, Block>,
}

impl MergedIsonDocument {
    pub fn get(&self, kind: &str, name: &str) -> Option<&Block> {
        self.blocks.get(&format!("{kind}.{name}"))
    }

    pub fn blocks(&self) -> &std::collections::HashMap<String, Block> {
        &self.blocks
    }

    pub fn into_blocks(self) -> std::collections::HashMap<String, Block> {
        self.blocks
    }
}

pub fn parse_and_merge_all_fences(content: &str, context: &str) -> Result<MergedIsonDocument> {
    let fences = extract_all_ison_fences(content, context)?;
    parse_and_merge_fences(&fences, context)
}

pub fn parse_and_merge_fences(fences: &[IsonFence], context: &str) -> Result<MergedIsonDocument> {
    let mut merged = MergedIsonDocument::default();

    for (idx, fence) in fences.iter().enumerate() {
        let doc = parse_ison_document(&fence.payload).map_err(|err| {
            anyhow!(
                "{context}: failed to parse ISON payload (block #{}) starting at line {}: {}",
                idx + 1,
                fence.start_line,
                format_ison_error(&err)
            )
        })?;

        for block in doc.blocks {
            let key = format!("{}.{}", block.kind, block.name);
            if merged.blocks.contains_key(&key) {
                return Err(anyhow!(
                    "{context}: duplicate canonical block `{}` (found more than once across ```ison fences)",
                    key
                ));
            }
            merged.blocks.insert(key, block);
        }
    }

    Ok(merged)
}

pub fn dumps_canonical(doc: &Document, align_columns: bool) -> String {
    let dumped = ison_rs::dumps(doc, align_columns);
    normalize_null_tokens(&dumped)
}

pub fn render_ison_fence(payload: &str) -> String {
    format!("```ison\n{}\n```\n", payload.trim_end())
}

pub fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn expect_fields(block: &Block, expected: &[&str], context: &str) -> Result<()> {
    let expected: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
    if block.fields != expected {
        return Err(anyhow!(
            "{context}: expected fields [{}], got [{}]",
            expected.join(" "),
            block.fields.join(" ")
        ));
    }
    Ok(())
}

pub fn get_optional_string(row: &Row, field: &str, context: &str) -> Result<Option<String>> {
    let Some(value) = row.get(field) else {
        return Ok(None);
    };
    match value {
        Value::Null => Ok(None),
        Value::String(value) => Ok(Some(value.to_string())),
        other => Err(anyhow!(
            "{context}: field `{}` must be a string or null, got {:?}",
            field,
            other
        )),
    }
}

pub fn get_required_string(
    row: &Row,
    field: &str,
    context: &str,
    allow_empty: bool,
) -> Result<String> {
    let value = get_optional_string(row, field, context)?
        .ok_or_else(|| anyhow!("{context}: missing required field `{}`", field))?;
    let trimmed = value.trim().to_string();
    if !allow_empty && trimmed.is_empty() {
        return Err(anyhow!(
            "{context}: required field `{}` must not be empty",
            field
        ));
    }
    Ok(value)
}

fn parse_ison_document(payload: &str) -> std::result::Result<Document, ISONError> {
    ison_rs::parse(payload)
}

fn format_ison_error(err: &ISONError) -> String {
    err.to_string()
}

fn normalize_null_tokens(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut escape = false;

    while i < bytes.len() {
        let ch = bytes[i] as char;

        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if input[i..].starts_with("null") {
            let prev_is_boundary = i == 0 || (bytes[i - 1] as char).is_whitespace();
            let next_i = i + 4;
            let next_is_boundary =
                next_i >= bytes.len() || (bytes[next_i] as char).is_whitespace();

            if prev_is_boundary && next_is_boundary {
                out.push('~');
                i += 4;
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_all_ison_fences_collects_multiple_blocks() {
        let content = r#"
## A
```ison
object.spec
version kind name purpose
"1.0.0" "llman.sdd.spec" sample "x"
```

## B
```ison
table.requirements
req_id title statement
foo "Foo" "System MUST foo."
```
"#;

        let fences = extract_all_ison_fences(content, "spec").expect("fences");
        assert_eq!(fences.len(), 2);
        assert!(fences[0].payload.contains("object.spec"));
        assert!(fences[1].payload.contains("table.requirements"));
    }

    #[test]
    fn dumps_canonical_normalizes_null_to_tilde() {
        let doc = ison_rs::parse(
            "table.ops\nop req_id name\nadd_requirement a null\n",
        )
        .expect("parse");
        let dumped = dumps_canonical(&doc, false);
        assert_eq!(dumped, "table.ops\nop req_id name\nadd_requirement a ~");
    }

    #[test]
    fn normalize_null_tokens_does_not_touch_strings() {
        let dumped = "table.ops\nop req_id name\nadd_requirement a \"null\"\n";
        let normalized = normalize_null_tokens(dumped);
        assert_eq!(normalized, dumped);
    }
}

