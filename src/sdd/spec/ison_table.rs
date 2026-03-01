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
        // NOTE: ison-rs v1.0.1 has a buggy "new block" heuristic: it ends a block when a line
        // starts with an alphabetic character and contains any '.' anywhere on the line, even if
        // the '.' is inside a quoted string value. This breaks the canonical table/object ISON
        // examples where statements/purpose end with '.'.
        //
        // Work around it by quoting the first token of any data row line that:
        // - starts with an alphabetic character, and
        // - contains a '.' somewhere on the line, and
        // - is not itself a block header line (kind.name).
        let payload = workaround_ison_rs_block_boundary_heuristic(&fence.payload);

        let doc = parse_ison_document(&payload).map_err(|err| {
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

fn workaround_ison_rs_block_boundary_heuristic(payload: &str) -> String {
    payload
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return line.to_string();
            }
            if trimmed.starts_with('#') {
                return line.to_string();
            }
            if is_block_header_line(trimmed) {
                return line.to_string();
            }

            let starts_alpha = trimmed
                .chars()
                .next()
                .map(|c| c.is_alphabetic())
                .unwrap_or(false);
            if !starts_alpha || !trimmed.contains('.') {
                return line.to_string();
            }

            quote_first_token(trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_block_header_line(line: &str) -> bool {
    if line.contains(' ') || line.contains('\t') {
        return false;
    }
    let Some(dot_index) = line.find('.') else {
        return false;
    };
    let kind = &line[..dot_index];
    let name = &line[dot_index + 1..];
    if kind.is_empty() || name.is_empty() {
        return false;
    }
    if !kind.chars().next().is_some_and(|c| c.is_alphabetic()) {
        return false;
    }
    kind.chars()
        .chain(name.chars())
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn quote_first_token(line: &str) -> String {
    let token_end = line
        .find(|c: char| [' ', '\t'].contains(&c))
        .unwrap_or(line.len());
    let (first, rest) = line.split_at(token_end);
    if first.starts_with('"') {
        return line.to_string();
    }
    format!("\"{first}\"{rest}")
}

pub fn dumps_canonical(doc: &Document, align_columns: bool) -> String {
    dump_document(doc, align_columns)
}

pub fn render_ison_fence(payload: &str) -> String {
    format!("```ison\n{}\n```\n", payload.trim_end())
}

fn dump_document(doc: &Document, align_columns: bool) -> String {
    let mut rendered_blocks = Vec::new();
    for block in &doc.blocks {
        rendered_blocks.push(dump_block(block, align_columns));
    }
    rendered_blocks.join("\n\n").trim_end().to_string()
}

fn dump_block(block: &Block, align_columns: bool) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{}.{}", block.kind, block.name));
    lines.push(block.fields.join(" "));

    let rows = block
        .rows
        .iter()
        .map(|row| dump_row(row, &block.fields))
        .collect::<Vec<_>>();
    let summary_rows = block
        .summary_rows
        .iter()
        .map(|row| dump_row(row, &block.fields))
        .collect::<Vec<_>>();

    if !align_columns {
        for row in rows {
            lines.push(row.join(" "));
        }
        if !summary_rows.is_empty() {
            lines.push("---".to_string());
            for row in summary_rows {
                lines.push(row.join(" "));
            }
        }
        return lines.join("\n").trim_end().to_string();
    }

    let widths = calculate_column_widths(&block.fields, &rows, &summary_rows);
    for row in rows {
        lines.push(pad_row(&row, &widths));
    }
    if !summary_rows.is_empty() {
        lines.push("---".to_string());
        for row in summary_rows {
            lines.push(pad_row(&row, &widths));
        }
    }

    lines.join("\n").trim_end().to_string()
}

fn dump_row(row: &Row, fields: &[String]) -> Vec<String> {
    fields
        .iter()
        .map(|field| {
            let value = row.get(field).unwrap_or(&Value::Null);
            dump_value(value)
        })
        .collect()
}

fn dump_value(value: &Value) -> String {
    match value {
        Value::Null => "~".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Reference(reference) => reference.to_ison(),
        Value::String(value) => dump_string(value),
    }
}

fn dump_string(value: &str) -> String {
    let needs_quotes = value.is_empty()
        || value.contains(' ')
        || value.contains('\t')
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('"')
        || value.contains('\\')
        || value.contains('.') // Avoid confusion with block headers (kind.name)
        || value == "true"
        || value == "false"
        || value == "null"
        || value.starts_with(':')
        || value == "~"
        || value.parse::<f64>().is_ok();

    if !needs_quotes {
        return value.to_string();
    }

    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r");
    format!("\"{escaped}\"")
}

fn calculate_column_widths(
    fields: &[String],
    rows: &[Vec<String>],
    summary_rows: &[Vec<String>],
) -> Vec<usize> {
    let mut widths = fields.iter().map(|f| f.len()).collect::<Vec<_>>();
    for row in rows.iter().chain(summary_rows.iter()) {
        for (idx, value) in row.iter().enumerate() {
            if idx >= widths.len() {
                continue;
            }
            widths[idx] = widths[idx].max(value.len());
        }
    }
    widths
}

fn pad_row(row: &[String], widths: &[usize]) -> String {
    let mut out = String::new();
    for (idx, value) in row.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(value);
        if idx < row.len().saturating_sub(1) {
            let width = widths.get(idx).copied().unwrap_or(value.len());
            if value.len() < width {
                out.extend(std::iter::repeat_n(' ', width - value.len()));
            }
        }
    }
    out.trim_end().to_string()
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

pub fn expect_fields_any_of(block: &Block, expected: &[&[&str]], context: &str) -> Result<()> {
    for variant in expected {
        let expected_vec: Vec<String> = variant.iter().map(|s| (*s).to_string()).collect();
        if block.fields == expected_vec {
            return Ok(());
        }
    }

    let variants = expected
        .iter()
        .map(|variant| variant.join(" "))
        .collect::<Vec<_>>()
        .join(" | ");
    Err(anyhow!(
        "{context}: expected fields [{}], got [{}]",
        variants,
        block.fields.join(" ")
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_all_ison_fences_collects_multiple_blocks() {
        let content = r#"
## A
```ison
object.spec
kind name purpose
"llman.sdd.spec" sample "x"
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
    fn parse_and_merge_fences_allows_dots_inside_quoted_strings() {
        let content = r#"
```ison
table.requirements
req_id title statement
existing "Existing behavior" "System MUST preserve existing behavior."
```
"#;

        let fences = extract_all_ison_fences(content, "spec").expect("fences");
        let merged = parse_and_merge_fences(&fences, "spec").expect("merged");
        let block = merged
            .get("table", "requirements")
            .expect("table.requirements");
        assert_eq!(block.rows.len(), 1);
        assert_eq!(
            block.rows[0].get("req_id"),
            Some(&Value::String("existing".to_string()))
        );
    }

    #[test]
    fn dumps_canonical_normalizes_null_to_tilde() {
        let doc =
            ison_rs::parse("table.ops\nop req_id name\nadd_requirement a null\n").expect("parse");
        let dumped = dumps_canonical(&doc, false);
        assert_eq!(dumped, "table.ops\nop req_id name\nadd_requirement a ~");
    }
}
