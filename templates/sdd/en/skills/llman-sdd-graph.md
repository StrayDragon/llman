---
name: "llman-sdd-graph"
description: "Generate a dependency graph from change proposal frontmatter (depends_on/blocks)."
---

# LLMAN SDD Graph

Use this skill to visualize change dependencies as a graph.

## Steps
1. Run `llman sdd graph` to generate a mermaid dependency graph from all active changes.
2. The graph reads `depends_on` and `blocks` from each change's `proposal.md` YAML frontmatter.
3. Output goes to stdout. Pipe it to a file or renderer as needed:
   ```
   llman sdd graph > deps.mmd
   llman sdd graph | mmdc -i - -o deps.png
   ```
4. Use `--format mermaid` to explicitly select the format (mermaid is the default).

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
