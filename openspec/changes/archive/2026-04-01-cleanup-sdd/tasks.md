## 1. Remove `sdd-legacy` CLI Surface

- [x] 1.1 Remove the `llman sdd-legacy ...` command group wiring (clap/subcommand entrypoints) and update help text to reflect single-track SDD.
- [x] 1.2 Delete legacy-only subcommands (including any legacy-only archive subcommands like freeze/thaw, if present) and remove any routing code that still dispatches to legacy.
- [x] 1.3 Update integration tests to remove `sdd-legacy` coverage and assert the new canonical rewrite guidance instead.

## 2. Remove Legacy Templates and Style Selection

- [x] 2.1 Delete `templates/sdd-legacy/**` and remove any template loader code paths that reference it (for example `TemplateStyle::Legacy` and style selection flags).
- [x] 2.2 Update `templates/sdd/{locale}/agents*.md` to remove references to the legacy track and to reflect the single-track workflow.

## 3. Remove Legacy JSON-in-ISON Parsing/Validation Paths

- [x] 3.1 Remove legacy JSON-in-` ```ison ` parsing support and any legacy parser modules (for example `ison_v1`), ensuring `llman sdd` supports canonical table/object ISON only.
- [x] 3.2 Update error messages and validation hints to remove any “try `llman sdd-legacy ...`” guidance and replace it with canonical rewrite guidance.

## 4. Update `llman x sdd-eval` to Single-Style

- [x] 4.1 Remove `sdd-legacy` from the playbook DSL/schema and runner implementation (variants no longer carry `style`).
- [x] 4.2 Remove legacy variants (for example `sdd-legacy-codex`) and update any bundled playbook templates/examples accordingly.

## 5. Add `llman-sdd-propose` (Ported From `openspec-propose`)

- [x] 5.1 Add new templates: `templates/sdd/en/skills/llman-sdd-propose.md` and `templates/sdd/zh-Hans/skills/llman-sdd-propose.md`, adapting `openspec-propose` semantics to `llmanspec/` + `llman sdd` commands (and using the shared structured protocol units).
- [x] 5.2 Update `llman sdd update-skills --all` generation to include `llman-sdd-propose`, and ensure Claude command generation stays consistent (if applicable).
- [x] 5.3 Update any docs/help that enumerate workflow skills/commands so `llman-sdd-propose` is discoverable.

## 6. Verification

- [x] 6.1 Run `openspec validate cleanup-sdd --strict --no-interactive` and fix any validation errors in the change artifacts.
- [x] 6.2 Run `just check` (or `cargo +nightly fmt -- --check`, `cargo +nightly clippy --all-targets --all-features -- -D warnings`, `cargo +nightly test --all`) and ensure CI parity.
- [x] 6.3 Manual smoke: verify `llman sdd` core flows (`init/update/update-skills/list/show/validate/archive`) and `llman x sdd-eval` still run successfully with the new single-style assumptions.
