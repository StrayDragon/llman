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

### Main spec with BDD feature_refs（可选）

当 `config.yaml` 定义了 `bdd` 块时，spec 可以引用 `.feature` 文件用于可执行 BDD 验证：

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
feature_refs[1]{path,scope,required}:
  tests/features/sample.feature,acceptance,true
```

- `path`: 相对于项目根的 `.feature` 文件路径
- `scope`: `acceptance` | `unit` | `reference`
- `required`: `true` → 缺失时报 ERROR；`false` → 缺失时报 WARNING

### Delta spec（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,New requirement,System MUST do the new thing.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",the new trigger happens,the new outcome is observed
```

### 表格化行的引号规则

在表格化数组行中（值以逗号分隔），如果值包含**逗号**、**冒号**、**方括号**（`[`, `]`, `{`, `}`）或首尾有空白字符，**必须使用双引号包裹**：

```
# 错误：statement 中的逗号被解析为分隔符 → 字段数量不匹配
r1,title,System MUST do X, Y, and Z.

# 正确：用引号包裹包含逗号的值
r1,title,"System MUST do X, Y, and Z."
```

- 空字符串：`""`
- 未设置的可选字段：`null`
- 不确定时，优先使用引号。

### 备注
- `toon` 使用显式数组头（例如 `requirements[<n>]{...}:`）和表格化行表达数组内容。
- `null` 表示该字段缺失（可选字段未设置）。
- 同一项目内不允许混用多种风格；跨风格迁移必须使用 `llman sdd convert`。
