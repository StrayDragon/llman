use anyhow::{Result, anyhow};
use regex::Regex;
use std::path::Path;

pub fn expand_regions<F>(template: &str, load_source: F) -> Result<String>
where
    F: Fn(&str) -> Result<String>,
{
    let re = Regex::new(r"\{\{\s*region:\s*([^#\s]+)\s*#\s*([^\s\}]+)\s*\}\}").expect("regex");
    let mut output = String::new();
    let mut last = 0;

    for caps in re.captures_iter(template) {
        let whole = caps.get(0).expect("match");
        output.push_str(&template[last..whole.start()]);
        let path = caps.get(1).expect("path").as_str();
        let name = caps.get(2).expect("name").as_str();
        let source = load_source(path)?;
        let region = extract_region(&source, path, name)?;
        output.push_str(region.trim_end());
        last = whole.end();
    }

    output.push_str(&template[last..]);
    Ok(output)
}

fn extract_region(content: &str, path: &str, name: &str) -> Result<String> {
    let syntax = RegionSyntax::for_path(path);
    let mut in_region = false;
    let mut found = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if let Some(caps) = syntax.start.captures(line) {
            let marker = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if marker == name {
                if found || in_region {
                    return Err(anyhow!(t!(
                        "sdd.templates.region_duplicate",
                        name = name,
                        path = path
                    )));
                }
                in_region = true;
                continue;
            }
        }

        if syntax.end.is_match(line) && in_region {
            found = true;
            in_region = false;
            continue;
        }

        if in_region {
            lines.push(line);
        }
    }

    if in_region {
        return Err(anyhow!(t!(
            "sdd.templates.region_unterminated",
            name = name,
            path = path
        )));
    }

    if !found {
        return Err(anyhow!(t!(
            "sdd.templates.region_missing",
            name = name,
            path = path
        )));
    }

    Ok(lines.join("\n"))
}

struct RegionSyntax {
    start: Regex,
    end: Regex,
}

impl RegionSyntax {
    fn for_path(path: &str) -> Self {
        let ext = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "md" | "markdown" | "html" | "htm" => Self::html_comment(),
            "rs" | "js" | "ts" | "tsx" | "jsx" => Self::line_comment("//"),
            "yml" | "yaml" | "toml" | "ini" | "sh" | "bash" => Self::line_comment("#"),
            _ => Self::line_comment("#"),
        }
    }

    fn html_comment() -> Self {
        let start = Regex::new(r"^\s*<!--\s*region:\s*(.+?)\s*-->\s*$").expect("regex");
        let end = Regex::new(r"^\s*<!--\s*endregion\s*-->\s*$").expect("regex");
        Self { start, end }
    }

    fn line_comment(prefix: &str) -> Self {
        let start = Regex::new(&format!(
            r"^\s*{}\s*region:\s*(.+?)\s*$",
            regex::escape(prefix)
        ))
        .expect("regex");
        let end =
            Regex::new(&format!(r"^\s*{}\s*endregion\s*$", regex::escape(prefix))).expect("regex");
        Self { start, end }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_source_map(source: &str) -> impl Fn(&str) -> Result<String> + '_ {
        move |_path| Ok(source.to_string())
    }

    #[test]
    fn expands_markdown_region() {
        let source = r#"<!-- region: intro -->
Hello
<!-- endregion -->"#;
        let template = "Start\n{{region: docs/readme.md#intro}}\nEnd";
        let result = expand_regions(template, load_source_map(source)).expect("expand");
        assert!(result.contains("Hello"));
    }

    #[test]
    fn expands_hash_region() {
        let source = "# region: intro\nHello\n# endregion";
        let template = "{{region: config.yaml#intro}}";
        let result = expand_regions(template, load_source_map(source)).expect("expand");
        assert_eq!(result, "Hello");
    }
}
