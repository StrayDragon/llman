# sdd-eval-acp-pipeline Specification (Delta)

## MODIFIED Requirements

### Requirement: Variants combine workflow style and agent preset
A playbook MUST define one or more `variants` that the workflow can execute.

Each variant MUST specify:
- an ACP `agent` definition (e.g. `claude-code-acp` or `codex-acp`)
- an account preset reference (Claude Code: `llman x cc` group; Codex: `llman x codex` group)

The workflow style for this pipeline MUST be new-style SDD only; legacy styles (for example `sdd-legacy`) MUST NOT be supported.

Variants MUST be addressable by a stable id (for example `variants.a`, `variants.b`) so that jobs can reference them via `strategy.matrix.variant`.

#### Scenario: Missing variants fails loudly
- **WHEN** the playbook has no variants and the user runs `llman x sdd-eval run ...`
- **THEN** the command exits non-zero and explains that at least one variant is required

#### Scenario: Matrix references unknown variant fails loudly
- **WHEN** a job defines `strategy.matrix.variant: ["a"]`
- **AND** the playbook does not define `variants.a`
- **THEN** the command exits non-zero
- **AND** the error explains that the matrix references a missing variant id

### Requirement: Workflow initialization is performed per variant
For each variant workspace, the runner MUST initialize/update SDD templates using new-style SDD.

In workflow-based playbooks, initialization MUST be exposed as a built-in action (for example `builtin:sdd-eval/sdd.prepare`) so it can be composed as an explicit step.

#### Scenario: Variant workspace is initialized using new-style templates
- **WHEN** a variant workspace is prepared for a run
- **THEN** the variant workspace is initialized using new-style SDD templates (equivalent to `llman sdd init` + `llman sdd update`)
