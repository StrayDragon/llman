# sdd-ab-evaluation — Delta Specification (sdd-style-eval-v2)

## ADDED Requirements

### Requirement: Format-Sensitive Agentic Tasks
The evaluation suite MUST include at least one agentic task that requires reading and editing the style-specific spec files (main spec and/or delta spec), so that format differences (ison/toon/yaml) materially affect the agent’s context and actions.

#### Scenario: Agent reads and edits main spec file
- **WHEN** the multi-style agentic eval runs for `spec_style: ison|toon|yaml`
- **THEN** the agent reads `llmanspec/specs/**/spec.md` in the current workspace
- **AND** makes a file-level edit that changes the spec content
- **AND** `llman sdd validate --all --strict --no-interactive` passes

#### Scenario: Agent reads and edits delta spec file
- **WHEN** the eval includes a change under `llmanspec/changes/**`
- **THEN** the agent reads `llmanspec/changes/**/specs/**/spec.md`
- **AND** makes a file-level edit that changes the delta content
- **AND** `llman sdd validate --all --strict --no-interactive` passes

### Requirement: Seeded Baseline Content Is Semantically Equivalent Across Styles
The eval runner MUST pre-seed each style workspace with semantically equivalent baseline specs/changes before starting Promptfoo evaluation, so that outcomes can be compared with reduced variance.

#### Scenario: Runner seeds baseline before evaluation
- **WHEN** a new eval run starts
- **THEN** the runner creates three isolated workspaces (`ison/toon/yaml`)
- **AND** seeds the same logical capability + change content in each workspace
- **AND** the seeded content is style-correct for that workspace

### Requirement: Multi-Run Aggregate Metrics Report
When the runner is executed with `--runs N` (N ≥ 2), it MUST generate an aggregate report that summarizes pass rate and token/turn/cost distributions per style across runs.

#### Scenario: Aggregation outputs a batch report
- **WHEN** a maintainer runs `just sdd-claude-style-eval --runs 10` (or equivalent)
- **THEN** the runner writes an aggregate summary to a batch-level `meta/aggregate.md` (and/or `meta/aggregate.json`)
- **AND** the report includes at least: pass rate, mean/median/p90 of total tokens and turns per style
