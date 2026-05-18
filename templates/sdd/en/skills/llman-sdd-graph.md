---
name: "llman-sdd-graph"
description: "Generate a dependency graph from change proposal frontmatter (depends_on/blocks)."
---

# LLMAN SDD Graph

Use this skill to visualize change dependencies as a graph.

## Usage

**Focused view (seed mode):** Show a specific change and its relationship neighborhood.

```bash
llman sdd graph <change-id>              # change + direct relationships (depth 1)
llman sdd graph <change-id> --depth 3    # recurse 3 levels
llman sdd graph <change-id> --depth 0    # just the change itself
```

The seed mode traverses upstream (depends_on), downstream (who depends on it), and blocks edges in all directions. It discovers both active and archived changes automatically.

**Full view (scope mode):** Show all changes by scope.

```bash
llman sdd graph                          # all active changes (default)
llman sdd graph --scope archived         # all archived (done) changes
llman sdd graph --scope all              # everything
```

## Output

- Output goes to stdout as mermaid flowchart. Pipe to a file or renderer:
  ```
  llman sdd graph c50 > deps.mmd
  llman sdd graph c50 --depth 2 | mmdc -i - -o deps.png
  ```
- Archived (done) changes are shown with a "✓ done" suffix and green highlight.
- When the graph has disconnected groups, each group renders as a separate subgraph labeled "Active", "Done", or "Mixed".

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

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
