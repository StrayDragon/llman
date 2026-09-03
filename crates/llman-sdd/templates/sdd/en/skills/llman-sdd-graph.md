---
name: "llman-sdd-graph"
description: "Visualize llman SDD change dependency relationships as a mermaid graph. Use to understand blocking and depends_on relationships for planning or inspection. Auxiliary tool — not part of the main implementation pipeline."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Dependency Graph

Use this skill to visualize dependencies between changes.

## Pipeline Position

```mermaid
flowchart LR
    pipeline["Main pipeline:<br/>propose → apply → verify → archive"]
    graph["📎 llman-sdd-graph<br/>Dependency visualization (utility)"]
    graph -.->|available at any stage| pipeline

    style graph fill:#e8f4e8,stroke:#28a745,stroke-width:2px
```

> 📎 Utility tool, available at any pipeline stage. To propose → `llman-sdd-propose`. To implement → `llman-sdd-apply` only when `readyToImplement=true`.

## Usage

**Focus view (seed mode):** Show a specific change and its relationship neighborhood.

```bash
llman sdd graph <change-id>              # the change + direct relationships (depth 1)
llman sdd graph <change-id> --depth 3    # recurse 3 levels
llman sdd graph <change-id> --depth 0    # just the change itself
```

Seed mode traverses three directions: upstream (depends_on), downstream (depended by), and blocks, automatically discovering active and archived changes.

**Global view (scope mode):** Show all changes by scope.

```bash
llman sdd graph                          # all active changes (default)
llman sdd graph --scope archived         # all archived (completed) changes
llman sdd graph --scope all              # everything
```

## Output

- Output is a mermaid flowchart to stdout, pipeable to a file or renderer:
  ```
  llman sdd graph c50 > deps.mmd
  llman sdd graph c50 --depth 2 | mmdc -i - -o deps.png
  ```
- Archived (completed) changes are shown with "✓ done" suffix and green highlight.
- When the graph contains disconnected groups, each group renders as an independent subgraph labeled "Active", "Done", or "Mixed".

## Proposal frontmatter format

```yaml
---
depends_on:
  - other-change-id
blocks:
  - blocked-change-id
---

## Why
...
```

> 💡 This is just a utility — main flow: `llman-sdd-propose` (Branch binding + Specs landing) → `llman-sdd-apply` (requires `readyToImplement`) → `llman-sdd-verify` → `llman-sdd-archive`.

> For command details run `llman sdd <cmd> --help`; the CLI is the command reference — skills embed no command tables (r139).

{{ unit("skills/ethics-governance") }}
