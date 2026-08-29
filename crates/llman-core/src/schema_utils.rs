//! Generic schema utilities shared by all config layers (global / project /
//! llmanspec). Pure logic only: no i18n, no filesystem writes — the
//! facade-side file handling stays in `config_schema`.
//!
//! This module is a member of the top-level utility layer (future
//! `llman-core`); it MUST NOT import feature modules (sdd/skills/tool/x).

use jsonschema::validator_for;
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;

const SCHEMA_ERROR_LIMIT: usize = 5;

/// Generate a draft-07 root schema with subschemas inlined (the project-wide
/// schema style, previously private in `config_schema`).
pub fn generate_schema<T: JsonSchema>() -> schemars::Schema {
    let mut settings = SchemaSettings::draft07();
    settings.inline_subschemas = true;
    settings.into_generator().into_root_schema_for::<T>()
}

/// Validate a YAML-derived value against the JSON schema of `T` (draft-07,
/// inlined).
///
/// Generic over the config type so feature crates (e.g. sdd owning
/// `SddConfig`) can validate without importing the facade's schema registry —
/// this is what breaks the former `config_schema <-> sdd::project` ring.
///
/// Callers parse YAML into `serde_json::Value` (via `serde-saphyr`) before
/// validation; the dynamic value tree is JSON-schema-shaped by construction.
pub fn validate_yaml_value_against<T: JsonSchema>(value: &serde_json::Value) -> Result<(), String> {
    let schema_value = serde_json::to_value(generate_schema::<T>()).map_err(|e| e.to_string())?;
    validate_schema_value(&schema_value, value)
}

/// Validate a value against an already-materialized JSON schema value.
pub fn validate_schema_value(
    schema_value: &serde_json::Value,
    value: &serde_json::Value,
) -> Result<(), String> {
    let validator = validator_for(schema_value).map_err(|e| e.to_string())?;
    if !validator.is_valid(value) {
        return Err(format_schema_errors(
            validator.iter_errors(value).map(|err| err.to_string()),
        ));
    }
    Ok(())
}

pub fn format_schema_errors<I>(errors: I) -> String
where
    I: IntoIterator<Item = String>,
{
    let mut iter = errors.into_iter();
    let mut items = Vec::new();
    for _ in 0..SCHEMA_ERROR_LIMIT {
        if let Some(err) = iter.next() {
            items.push(err);
        } else {
            break;
        }
    }
    let remaining = iter.count();
    if items.is_empty() {
        return "unknown".to_string();
    }
    let mut message = items.join("; ");
    if remaining > 0 {
        message.push_str(&format!("; ... (+{remaining} more)"));
    }
    message
}

pub fn schema_header_line(schema_url: &str) -> String {
    format!("# yaml-language-server: $schema={schema_url}")
}

pub fn prepend_schema_header(content: &str, schema_url: &str) -> String {
    let header = schema_header_line(schema_url);
    if content.is_empty() {
        return format!("{header}\n");
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    format!("{header}{newline}{content}")
}

pub fn apply_schema_header_to_content(content: &str, schema_url: &str) -> (String, bool) {
    let header = schema_header_line(schema_url);
    if content.is_empty() {
        return (format!("{header}\n"), true);
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let has_trailing = content.ends_with('\n') || content.ends_with("\r\n");
    let all_lines = content.lines().collect::<Vec<_>>();

    // Only normalize the leading header/comment region. Do not delete schema headers that
    // appear later in the file.
    let mut header_end = 0;
    while header_end < all_lines.len() {
        let line = all_lines[header_end];
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            header_end += 1;
            continue;
        }
        break;
    }

    let mut normalized_header_lines = Vec::new();
    for line in &all_lines[..header_end] {
        if line
            .trim_start()
            .starts_with("# yaml-language-server: $schema=")
        {
            continue;
        }
        normalized_header_lines.push((*line).to_string());
    }

    let mut out_lines = Vec::with_capacity(all_lines.len() + 1);
    out_lines.push(header);
    out_lines.extend(normalized_header_lines);
    out_lines.extend(
        all_lines[header_end..]
            .iter()
            .map(|line| (*line).to_string()),
    );
    let mut updated = out_lines.join(newline);
    if has_trailing {
        updated.push_str(newline);
    }
    let changed = updated != content;
    (updated, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_schema_header_inserts_before_doc_start() {
        let content = "---\nversion: \"0.1\"\n";
        let (updated, changed) =
            apply_schema_header_to_content(content, "https://example.com/g.json");
        assert!(changed);
        assert!(updated.starts_with("# yaml-language-server: $schema="));
        assert!(updated.contains("\n---\n"));
    }

    #[test]
    fn apply_schema_header_replaces_existing() {
        let content =
            "# yaml-language-server: $schema=https://example.com/old.json\nversion: \"0.1\"\n";
        let (updated, changed) =
            apply_schema_header_to_content(content, "https://example.com/g.json");
        assert!(changed);
        assert!(updated.starts_with(&schema_header_line("https://example.com/g.json")));
        assert!(!updated.contains("old.json"));
    }

    #[test]
    fn apply_schema_header_does_not_delete_late_schema_headers() {
        let content = "# comment\n# yaml-language-server: $schema=https://example.com/old.json\nkey: value\n# yaml-language-server: $schema=https://example.com/keep.json\n".to_string();
        let (updated, changed) =
            apply_schema_header_to_content(&content, "https://example.com/g.json");
        assert!(changed);
        assert!(updated.starts_with(&schema_header_line("https://example.com/g.json")));
        assert!(updated.contains("https://example.com/keep.json"));
    }
}
