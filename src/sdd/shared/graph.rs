use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{extract_archived_change_id, list_changes};
use crate::sdd::spec::frontmatter::split_frontmatter;
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GraphArgs {
    pub format: String,
    pub scope: String,
    pub depth: usize,
    pub change: Option<String>,
}

#[derive(Clone, Copy)]
enum EdgeKind {
    DependsOn,
    Blocks,
}

struct DependencyEdge {
    from: String,
    to: String,
    kind: EdgeKind,
}

#[derive(Clone)]
struct GraphNode {
    id: String,
    archived: bool,
}

pub fn run(args: GraphArgs) -> Result<()> {
    let root = Path::new(".");
    match args.format.as_str() {
        "mermaid" => render_mermaid(root, &args),
        other => Err(anyhow!("Unsupported format: {}. Supported: mermaid", other)),
    }
}

// ---------------------------------------------------------------------------
// Node collection helpers
// ---------------------------------------------------------------------------

fn collect_active_nodes(root: &Path) -> Vec<GraphNode> {
    let active_ids = list_changes(root).unwrap_or_default();
    let changes_dir = root.join(LLMANSPEC_DIR_NAME).join("changes");
    let mut nodes: Vec<GraphNode> = active_ids
        .iter()
        .map(|id| GraphNode {
            id: id.clone(),
            archived: false,
        })
        .collect();
    // Also scan for directories without proposal.md (partial changes)
    if let Ok(entries) = fs::read_dir(&changes_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "archive" {
                continue;
            }
            if !nodes.iter().any(|n| n.id == name) {
                nodes.push(GraphNode {
                    id: name,
                    archived: false,
                });
            }
        }
    }
    nodes
}

fn collect_archived_nodes(root: &Path) -> Vec<GraphNode> {
    let archive_dir = root
        .join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join("archive");
    let entries = match fs::read_dir(&archive_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(change_id) = extract_archived_change_id(&name)
            && seen.insert(change_id.clone())
        {
            nodes.push(GraphNode {
                id: change_id,
                archived: true,
            });
        }
    }
    nodes
}

fn collect_all_nodes(root: &Path) -> Vec<GraphNode> {
    let mut combined = collect_active_nodes(root);
    let active_ids: HashSet<String> = combined.iter().map(|n| n.id.clone()).collect();
    for n in collect_archived_nodes(root) {
        if !active_ids.contains(&n.id) {
            combined.push(n);
        }
    }
    combined
}

fn find_node_dir(root: &Path, node: &GraphNode) -> std::path::PathBuf {
    if node.archived {
        let archive_dir = root
            .join(LLMANSPEC_DIR_NAME)
            .join("changes")
            .join("archive");
        find_latest_archived_dir(&archive_dir, &node.id)
            .unwrap_or_else(|| archive_dir.join(&node.id))
    } else {
        root.join(LLMANSPEC_DIR_NAME).join("changes").join(&node.id)
    }
}

fn find_latest_archived_dir(archive_dir: &Path, change_id: &str) -> Option<std::path::PathBuf> {
    let entries = fs::read_dir(archive_dir).ok()?;
    let mut latest: Option<(String, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(id) = extract_archived_change_id(&name)
            && id == change_id
        {
            let date_part = &name[..10];
            match &latest {
                Some((prev, _)) if date_part <= prev.as_str() => {}
                _ => latest = Some((date_part.to_string(), entry.path())),
            }
        }
    }
    latest.map(|(_, p)| p)
}

// ---------------------------------------------------------------------------
// Seed-based neighborhood BFS
// ---------------------------------------------------------------------------

/// Build global relationship maps from all known changes (active + archived).
struct RelationMaps {
    depends_on: HashMap<String, Vec<String>>,
    reverse_depends: HashMap<String, Vec<String>>,
    blocks: HashMap<String, Vec<String>>,
    reverse_blocks: HashMap<String, Vec<String>>,
    node_set: HashSet<String>,
}

fn build_relation_maps(root: &Path, all_nodes: &[GraphNode]) -> RelationMaps {
    let mut depends_on: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse_depends: HashMap<String, Vec<String>> = HashMap::new();
    let mut blocks: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse_blocks: HashMap<String, Vec<String>> = HashMap::new();
    let mut node_set: HashSet<String> = HashSet::new();

    for node in all_nodes {
        node_set.insert(node.id.clone());
        let dir = find_node_dir(root, node);
        let deps = parse_proposal_frontmatter(&dir);

        for dep in &deps.depends_on {
            depends_on
                .entry(node.id.clone())
                .or_default()
                .push(dep.clone());
            reverse_depends
                .entry(dep.clone())
                .or_default()
                .push(node.id.clone());
        }
        for blocked in &deps.blocks {
            blocks
                .entry(node.id.clone())
                .or_default()
                .push(blocked.clone());
            reverse_blocks
                .entry(blocked.clone())
                .or_default()
                .push(node.id.clone());
        }
    }

    RelationMaps {
        depends_on,
        reverse_depends,
        blocks,
        reverse_blocks,
        node_set,
    }
}

fn build_seed_neighborhood(root: &Path, seed_id: &str, max_depth: usize) -> Result<Vec<GraphNode>> {
    let all_nodes = collect_all_nodes(root);
    let node_map: HashMap<&str, &GraphNode> =
        all_nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    if !node_map.contains_key(seed_id) {
        let suggestions: Vec<&str> = node_map
            .keys()
            .filter(|k| k.starts_with(seed_id.split('-').next().unwrap_or("")))
            .copied()
            .collect();
        return Err(anyhow!(
            "Change '{}' not found.{}",
            seed_id,
            if suggestions.is_empty() {
                String::new()
            } else {
                format!(" Did you mean one of: {}?", suggestions.join(", "))
            }
        ));
    }

    let maps = build_relation_maps(root, &all_nodes);

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    visited.insert(seed_id.to_string());
    queue.push_back((seed_id.to_string(), 0));

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        // Collect neighbors from all 4 directions
        let neighbors: Vec<&String> = [
            &maps.depends_on,
            &maps.reverse_depends,
            &maps.blocks,
            &maps.reverse_blocks,
        ]
        .iter()
        .flat_map(|map| map.get(&node_id).map(|v| v.iter()).into_iter().flatten())
        .filter(|n| maps.node_set.contains(n.as_str()) && !visited.contains(n.as_str()))
        .collect();

        for neighbor in neighbors {
            visited.insert(neighbor.clone());
            queue.push_back((neighbor.clone(), depth + 1));
        }
    }

    // Return nodes in visited set, preserving type info
    let mut result: Vec<GraphNode> = visited
        .iter()
        .filter_map(|id| node_map.get(id.as_str()).map(|n| (*n).clone()))
        .collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

// ---------------------------------------------------------------------------
// Edge collection (from a specific node set)
// ---------------------------------------------------------------------------

fn collect_edges_for_nodes(root: &Path, nodes: &[GraphNode]) -> Vec<DependencyEdge> {
    let mut edges = Vec::new();
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    for node in nodes {
        let dir = find_node_dir(root, node);
        let deps = parse_proposal_frontmatter(&dir);
        for dep in &deps.depends_on {
            if node_ids.contains(dep.as_str()) {
                edges.push(DependencyEdge {
                    from: node.id.clone(),
                    to: dep.clone(),
                    kind: EdgeKind::DependsOn,
                });
            }
        }
        for blocked in &deps.blocks {
            if node_ids.contains(blocked.as_str()) {
                edges.push(DependencyEdge {
                    from: node.id.clone(),
                    to: blocked.clone(),
                    kind: EdgeKind::Blocks,
                });
            }
        }
    }
    edges
}

// ---------------------------------------------------------------------------
// Connected components (union-find)
// ---------------------------------------------------------------------------

fn find_connected_components(node_ids: &[String], edges: &[DependencyEdge]) -> Vec<Vec<String>> {
    if node_ids.is_empty() {
        return Vec::new();
    }

    let mut parent: HashMap<String, String> = HashMap::new();
    for id in node_ids {
        parent.insert(id.clone(), id.clone());
    }

    fn find(parent: &mut HashMap<String, String>, x: &str) -> String {
        let root = parent.get(x).unwrap().clone();
        if root == x {
            return root;
        }
        let found = find(parent, &root);
        parent.insert(x.to_string(), found.clone());
        found
    }

    fn union(parent: &mut HashMap<String, String>, a: &str, b: &str) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent.insert(ra, rb);
        }
    }

    for edge in edges {
        union(&mut parent, &edge.from, &edge.to);
    }

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for id in node_ids {
        let root = find(&mut parent, id);
        groups.entry(root).or_default().push(id.clone());
    }

    let mut result: Vec<Vec<String>> = groups.into_values().collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.len()));
    result
}

fn compute_subgraph_label(nodes: &[GraphNode], component_ids: &HashSet<&str>) -> &'static str {
    let comp_nodes: Vec<&GraphNode> = nodes
        .iter()
        .filter(|n| component_ids.contains(n.id.as_str()))
        .collect();
    let all_active = comp_nodes.iter().all(|n| !n.archived);
    let all_archived = comp_nodes.iter().all(|n| n.archived);
    match (all_active, all_archived) {
        (true, _) => "Active",
        (_, true) => "Done",
        _ => "Mixed",
    }
}

// ---------------------------------------------------------------------------
// Proposal frontmatter parsing
// ---------------------------------------------------------------------------

struct ProposalDeps {
    depends_on: Vec<String>,
    blocks: Vec<String>,
}

fn parse_proposal_frontmatter(change_dir: &Path) -> ProposalDeps {
    let content = match fs::read_to_string(change_dir.join("proposal.md")) {
        Ok(c) => c,
        Err(_) => {
            return ProposalDeps {
                depends_on: Vec::new(),
                blocks: Vec::new(),
            };
        }
    };
    let (yaml_str, _) = split_frontmatter(&content);
    let Some(yaml_str) = yaml_str else {
        return ProposalDeps {
            depends_on: Vec::new(),
            blocks: Vec::new(),
        };
    };
    let parsed: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => {
            return ProposalDeps {
                depends_on: Vec::new(),
                blocks: Vec::new(),
            };
        }
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

// ---------------------------------------------------------------------------
// Mermaid rendering
// ---------------------------------------------------------------------------

fn render_mermaid(root: &Path, args: &GraphArgs) -> Result<()> {
    let nodes = if let Some(ref seed) = args.change {
        build_seed_neighborhood(root, seed, args.depth)?
    } else {
        match args.scope.as_str() {
            "active" => collect_active_nodes(root),
            "archived" => collect_archived_nodes(root),
            "all" => collect_all_nodes(root),
            other => {
                return Err(anyhow!(
                    "Unknown scope: {}. Supported: active, archived, all",
                    other
                ));
            }
        }
    };

    if nodes.is_empty() {
        println!("flowchart TD");
        if let Some(ref seed) = args.change {
            println!(
                "    empty[\"Change '{}' not found or has no relationships\"]",
                seed
            );
        } else {
            println!("    empty[\"No changes in scope '{}'\"]", args.scope);
        }
        return Ok(());
    }

    let edges = collect_edges_for_nodes(root, &nodes);

    let node_id_list: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let components = find_connected_components(&node_id_list, &edges);
    let has_archived = nodes.iter().any(|n| n.archived);

    println!("flowchart TD");

    if components.len() > 1 {
        // Subgraph rendering for disconnected components
        for (idx, component) in components.iter().enumerate() {
            let comp_ids: HashSet<&str> = component.iter().map(|s| s.as_str()).collect();
            let label = compute_subgraph_label(&nodes, &comp_ids);
            println!("    subgraph sg{}[\"{}\"]", idx + 1, label);
            render_nodes_and_edges(&nodes, &edges, &comp_ids, true);
            println!("    end");
        }
    } else {
        let all_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        render_nodes_and_edges(&nodes, &edges, &all_ids, false);
    }

    if has_archived {
        println!("    classDef archived fill:#d4edda,stroke:#28a745,color:#333");
    }

    Ok(())
}

fn render_nodes_and_edges(
    nodes: &[GraphNode],
    edges: &[DependencyEdge],
    component_ids: &HashSet<&str>,
    indented: bool,
) {
    let pad = if indented { "        " } else { "    " };

    for node in nodes {
        if !component_ids.contains(node.id.as_str()) {
            continue;
        }
        let sid = sanitize_id(&node.id);
        if node.archived {
            println!("{}{}[\"{} ✓ done\"]:::archived", pad, sid, node.id);
        } else {
            println!("{}{}[\"{}\"]", pad, sid, node.id);
        }
    }

    for edge in edges {
        if !component_ids.contains(edge.from.as_str()) {
            continue;
        }
        match edge.kind {
            EdgeKind::DependsOn => println!(
                "{}{} -->|depends on| {}",
                pad,
                sanitize_id(&edge.from),
                sanitize_id(&edge.to),
            ),
            EdgeKind::Blocks => println!(
                "{}{} -.->|blocks| {}",
                pad,
                sanitize_id(&edge.from),
                sanitize_id(&edge.to),
            ),
        }
    }
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_connected_components_single() {
        let nodes = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let edges = vec![
            DependencyEdge {
                from: "a".into(),
                to: "b".into(),
                kind: EdgeKind::DependsOn,
            },
            DependencyEdge {
                from: "b".into(),
                to: "c".into(),
                kind: EdgeKind::DependsOn,
            },
        ];
        let components = find_connected_components(&nodes, &edges);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }

    #[test]
    fn test_find_connected_components_disconnected() {
        let nodes = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];
        let edges = vec![DependencyEdge {
            from: "a".into(),
            to: "b".into(),
            kind: EdgeKind::DependsOn,
        }];
        let components = find_connected_components(&nodes, &edges);
        assert_eq!(components.len(), 3); // {a,b}, {c}, {d}
    }

    #[test]
    fn test_compute_subgraph_label() {
        let active = GraphNode {
            id: "a".into(),
            archived: false,
        };
        let done = GraphNode {
            id: "b".into(),
            archived: true,
        };
        let mixed_nodes = vec![active.clone(), done.clone()];

        let all_active_ids: HashSet<&str> = ["a"].into_iter().collect();
        assert_eq!(compute_subgraph_label(&[active], &all_active_ids), "Active");

        let all_done_ids: HashSet<&str> = ["b"].into_iter().collect();
        assert_eq!(compute_subgraph_label(&[done], &all_done_ids), "Done");

        let mixed_ids: HashSet<&str> = ["a", "b"].into_iter().collect();
        assert_eq!(compute_subgraph_label(&mixed_nodes, &mixed_ids), "Mixed");
    }
}
