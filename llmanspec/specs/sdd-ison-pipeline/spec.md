---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - llman sdd validate sdd-ison-pipeline --type spec --strict --no-interactive
llman_spec_evidence:
  - migrated from openspec
---

```toon
kind: llman.sdd.spec
name: "sdd-ison-pipeline"
purpose: "TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive."
requirements[5]{req_id,title,statement}:
  r1,"ISON-First SDD Template Sources",The SDD template system MUST support ISON source templates as the primary authoring format for the new style track.
  r2,ISON Validation Before Render,The system MUST validate ISON source templates before rendering outputs.
  r3,Runtime Spec Parsing Uses ISON Container,"The SDD runtime MUST parse `llmanspec` main specs according to the project’s configured `spec_style`, rather than assuming all `spec.md` payloads are ISON. - for `spec_style: ison`, the runtime MUST parse canonical table/object ISON from fenced ` ```ison ` blocks - for `spec_style: toon`, the runtime MUST parse one canonical TOON document from a fenced ` ```toon ` block - for `spec_style: yaml`, the runtime MUST parse one canonical YAML document from a fenced ` ```yaml ` block The parser MUST se"
  r4,Runtime Delta Parsing Uses ISON Ops,"The SDD runtime MUST parse change delta specs according to the project’s configured `spec_style`, rather than assuming all delta specs use ISON ops blocks. - for `spec_style: ison`, the runtime MUST read delta ops from `table.ops` and scenarios from `table.op_scenarios` - for `spec_style: toon` and `spec_style: yaml`, the runtime MUST read delta ops from canonical `ops` collections and scenarios from canonical `op_scenarios` collections The runtime MUST key add/modify/remove/rename semantics by "
  r5,多风格解析必须先归一化到共享语义模型,"SDD runtime MUST 在风格相关解析完成后，先将主 spec 与 delta spec 归一化到共享语义模型，再驱动： - `llman sdd list` - `llman sdd show` - `llman sdd validate` - `llman sdd archive` - `llman sdd spec` - `llman sdd delta` 命令实现 MUST NOT 为不同风格复制三套独立的需求/场景/op 业务逻辑；风格差异 MUST 仅停留在 envelope parsing 与 serialization 层。"
scenarios[8]{req_id,id,given,when,then}:
  r1,"new-style-template-generation-reads-ison-source","",a maintainer runs SDD template refresh for new style,the system reads ISON source templates
  r2,"invalid-ison-source-blocks-rendering","",a new style ISON template has structural or type errors,"SDD template generation fails with non-zero exit"
  r3,"show-list-validate-parse-yaml-main-spec-by-configured-style","","a user runs SDD commands that read `llmanspec/specs/<capability>/spec.md` in a project with `spec_style: yaml`",the parser extracts and parses the ` ```yaml ` payload as canonical semantic source
  r3,"style-mismatch-is-rejected-without-fallback","","a project declares `spec_style: yaml`","validation fails with non-zero exit"
  r3,"validation-rejects-legacy-json-payloads-in-ison-projects","","a user runs validation in a project with `spec_style: ison` on a main spec whose ` ```ison ` payload is JSON","validation fails with non-zero exit"
  r4,"change-validation-parses-yaml-ops-collection","","a user validates a change in a project with `spec_style: yaml`",delta operations are read from the YAML `ops` collection
  r4,"validation-rejects-legacy-delta-json-payloads-in-ison-projec","","a user runs validation in a project with `spec_style: ison` on a delta spec whose ` ```ison ` payload is JSON","validation fails with non-zero exit"
  r5,不同风格共享同一验证语义,"",同一份 requirement/scenario 语义分别以 `ison` 与 `yaml` 表达,"strict validation 对“缺失 scenario”或“重复 `(req_id, id)`”给出相同语义结论"
```
