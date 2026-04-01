<!-- llman-template-version: 1 -->
## Canonical TOON Spec Contract（experimental）

当项目配置 `spec_style: toon` 时，SDD 主 spec 与 delta spec 都必须以 **单个** fenced ` ```toon ` code block 承载一份 canonical TOON 文档。

### Main spec（`llmanspec/specs/<feature-id>/spec.md`）

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

### Delta spec（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,New requirement,System MUST do the new thing.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",the new trigger happens,the new outcome is observed
```

### 备注
- `toon` 使用显式数组头（例如 `requirements[<n>]{...}:`）和表格化行表达数组内容。
- `null` 表示该字段缺失（可选字段未设置）。
- 同一项目内不允许混用多种风格；跨风格迁移必须使用 `llman sdd convert`。
