pub(crate) fn split_frontmatter(content: &str) -> (Option<String>, String) {
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

pub(crate) fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}
