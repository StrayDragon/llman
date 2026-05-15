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
