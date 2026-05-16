use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::list_changes;
use crate::sdd::spec::frontmatter::split_frontmatter;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GraphArgs {
    pub format: String,
}

struct DependencyEdge {
    from: String,
    to: String,
    kind: EdgeKind,
}

#[derive(Clone, Copy)]
enum EdgeKind {
    DependsOn,
    Blocks,
}

pub fn run(args: GraphArgs) -> Result<()> {
    let root = Path::new(".");
    match args.format.as_str() {
        "mermaid" => render_mermaid(root),
        other => Err(anyhow!("Unsupported format: {}. Supported: mermaid", other)),
    }
}

fn collect_edges(root: &Path) -> Result<(Vec<String>, Vec<DependencyEdge>)> {
    let change_ids = list_changes(root)?;
    if change_ids.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Also scan for directories without proposal.md (partial changes)
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let mut all_ids = change_ids.clone();
    if let Ok(entries) = fs::read_dir(&changes_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "archive" {
                continue;
            }
            if !all_ids.contains(&name) {
                all_ids.push(name);
            }
        }
    }

    let mut edges = Vec::new();
    for id in &all_ids {
        let change_dir = changes_dir.join(id);
        let fm = parse_proposal_frontmatter(&change_dir);
        for dep in &fm.depends_on {
            edges.push(DependencyEdge {
                from: id.clone(),
                to: dep.clone(),
                kind: EdgeKind::DependsOn,
            });
        }
        for blocked in &fm.blocks {
            edges.push(DependencyEdge {
                from: id.clone(),
                to: blocked.clone(),
                kind: EdgeKind::Blocks,
            });
        }
    }

    Ok((all_ids, edges))
}

struct ProposalDeps {
    depends_on: Vec<String>,
    blocks: Vec<String>,
}

fn parse_proposal_frontmatter(change_dir: &Path) -> ProposalDeps {
    let content = match fs::read_to_string(change_dir.join("proposal.md")) {
        Ok(c) => c,
        Err(_) => return ProposalDeps { depends_on: Vec::new(), blocks: Vec::new() },
    };
    let (yaml_str, _) = split_frontmatter(&content);
    let Some(yaml_str) = yaml_str else {
        return ProposalDeps { depends_on: Vec::new(), blocks: Vec::new() };
    };
    let parsed: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => return ProposalDeps { depends_on: Vec::new(), blocks: Vec::new() },
    };
    ProposalDeps {
        depends_on: extract_string_list(&parsed, "depends_on"),
        blocks: extract_string_list(&parsed, "blocks"),
    }
}

fn extract_string_list(doc: &serde_yaml::Value, key: &str) -> Vec<String> {
    let Some(value) = doc.get(key) else {
        return Vec::new();
    };
    match value {
        serde_yaml::Value::Sequence(values) => values
            .iter()
            .filter_map(|v| match v {
                serde_yaml::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn render_mermaid(root: &Path) -> Result<()> {
    let (nodes, edges) = collect_edges(root)?;

    if nodes.is_empty() {
        println!("graph LR");
        println!("    empty[\"No active changes\"]");
        return Ok(());
    }

    println!("graph LR");
    for node in &nodes {
        println!("    {}[\"{}\"]", sanitize_id(node), node);
    }
    for edge in &edges {
        let label = match edge.kind {
            EdgeKind::DependsOn => "depends on",
            EdgeKind::Blocks => "blocks",
        };
        println!(
            "    {} -->|{}| {}",
            sanitize_id(&edge.from),
            label,
            sanitize_id(&edge.to),
        );
    }

    Ok(())
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}
