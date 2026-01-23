use anyhow::Result;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct RequirementBlock {
    pub header_line: String,
    pub name: String,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct DeltaPlan {
    pub added: Vec<RequirementBlock>,
    pub modified: Vec<RequirementBlock>,
    pub removed: Vec<String>,
    pub renamed: Vec<RenamePair>,
    pub section_presence: SectionPresence,
}

#[derive(Debug, Clone)]
pub struct RenamePair {
    pub from: String,
    pub to: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SectionPresence {
    pub added: bool,
    pub modified: bool,
    pub removed: bool,
    pub renamed: bool,
}

pub fn normalize_requirement_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn parse_delta_spec(content: &str) -> Result<DeltaPlan> {
    let normalized = normalize_line_endings(content);
    let sections = split_top_level_sections(&normalized);

    let added_section = get_section_case_insensitive(&sections, "ADDED Requirements");
    let modified_section = get_section_case_insensitive(&sections, "MODIFIED Requirements");
    let removed_section = get_section_case_insensitive(&sections, "REMOVED Requirements");
    let renamed_section = get_section_case_insensitive(&sections, "RENAMED Requirements");

    let added = parse_requirement_blocks_from_section(&added_section.body);
    let modified = parse_requirement_blocks_from_section(&modified_section.body);
    let removed = parse_removed_names(&removed_section.body);
    let renamed = parse_renamed_pairs(&renamed_section.body);

    Ok(DeltaPlan {
        added,
        modified,
        removed,
        renamed,
        section_presence: SectionPresence {
            added: added_section.found,
            modified: modified_section.found,
            removed: removed_section.found,
            renamed: renamed_section.found,
        },
    })
}

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace("\r", "\n")
}

fn split_top_level_sections(content: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut headers: Vec<(String, usize)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some(title) = line.trim().strip_prefix("## ") {
            headers.push((title.trim().to_string(), idx));
        }
    }

    let mut sections = Vec::new();
    for i in 0..headers.len() {
        let (title, start_idx) = &headers[i];
        let end_idx = if i + 1 < headers.len() {
            headers[i + 1].1
        } else {
            lines.len()
        };
        let body = lines[(start_idx + 1)..end_idx].join("\n");
        sections.push((title.clone(), body));
    }

    sections
}

struct SectionLookup {
    body: String,
    found: bool,
}

fn get_section_case_insensitive(sections: &[(String, String)], desired: &str) -> SectionLookup {
    for (title, body) in sections {
        if title.eq_ignore_ascii_case(desired) {
            return SectionLookup {
                body: body.to_string(),
                found: true,
            };
        }
    }
    SectionLookup {
        body: String::new(),
        found: false,
    }
}

fn parse_requirement_blocks_from_section(section_body: &str) -> Vec<RequirementBlock> {
    if section_body.trim().is_empty() {
        return Vec::new();
    }
    let normalized = normalize_line_endings(section_body);
    let lines: Vec<&str> = normalized.lines().collect();
    let mut blocks = Vec::new();
    let header_re = Regex::new(r"^###\s*Requirement:\s*(.+)\s*$").expect("regex");
    let mut i = 0;
    while i < lines.len() {
        while i < lines.len() && !header_re.is_match(lines[i]) {
            i += 1;
        }
        if i >= lines.len() {
            break;
        }
        let header_line = lines[i];
        let name = header_re
            .captures(header_line)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim())
            .unwrap_or("");
        let mut buffer = vec![header_line.to_string()];
        i += 1;
        while i < lines.len()
            && !header_re.is_match(lines[i])
            && !lines[i].trim_start().starts_with("## ")
        {
            buffer.push(lines[i].to_string());
            i += 1;
        }
        let raw = buffer.join("\n").trim_end().to_string();
        blocks.push(RequirementBlock {
            header_line: header_line.to_string(),
            name: normalize_requirement_name(name),
            raw,
        });
    }
    blocks
}

fn parse_removed_names(section_body: &str) -> Vec<String> {
    if section_body.trim().is_empty() {
        return Vec::new();
    }
    let mut names = Vec::new();
    let normalized = normalize_line_endings(section_body);
    let lines: Vec<&str> = normalized.lines().collect();
    let header_re = Regex::new(r"^###\s*Requirement:\s*(.+)\s*$").expect("regex");
    let bullet_re = Regex::new(r"^\s*-\s*`?###\s*Requirement:\s*(.+?)`?\s*$").expect("regex");

    for line in lines {
        if let Some(caps) = header_re.captures(line) {
            if let Some(m) = caps.get(1) {
                names.push(normalize_requirement_name(m.as_str()));
            }
            continue;
        }
        if let Some(m) = bullet_re.captures(line).and_then(|caps| caps.get(1)) {
            names.push(normalize_requirement_name(m.as_str()));
        }
    }
    names
}

fn parse_renamed_pairs(section_body: &str) -> Vec<RenamePair> {
    if section_body.trim().is_empty() {
        return Vec::new();
    }
    let normalized = normalize_line_endings(section_body);
    let lines: Vec<&str> = normalized.lines().collect();
    let from_re =
        Regex::new(r"^\s*-?\s*FROM:\s*`?###\s*Requirement:\s*(.+?)`?\s*$").expect("regex");
    let to_re = Regex::new(r"^\s*-?\s*TO:\s*`?###\s*Requirement:\s*(.+?)`?\s*$").expect("regex");
    let mut current_from: Option<String> = None;
    let mut pairs = Vec::new();

    for line in lines {
        if let Some(caps) = from_re.captures(line) {
            current_from = caps.get(1).map(|m| normalize_requirement_name(m.as_str()));
            continue;
        }
        if let Some(caps) = to_re.captures(line) {
            let to = caps.get(1).map(|m| normalize_requirement_name(m.as_str()));
            if let (Some(from), Some(to)) = (current_from.take(), to) {
                pairs.push(RenamePair { from, to });
            }
        }
    }
    pairs
}
