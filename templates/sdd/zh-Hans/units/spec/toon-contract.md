## Canonical TOON Spec Contract

SDD 主 spec 与 delta spec 都以**独立的 `.toon` 文件**承载——每个文件一份 TOON 文档，没有 Markdown 外壳，也没有 fenced code block。所有结构化信息（包括原先位于 YAML frontmatter 的校验元数据）都在 TOON 文档内部。

### Main spec（`llmanspec/specs/<feature-id>/spec.toon`）

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[2]: src/,tests/
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

- `kind` 必须为 `llman.sdd.spec`。
- `name` 应与 spec 目录名一致。
- `valid_scope` 是校验作用域（驱动 staleness 检查）。必须存在且非空，为扁平单列表格化数组（例如 `valid_scope[2]: src/,tests/`）。（`valid_commands` 与 `evidence` 已移除——仅有 `valid_scope` 被实际消费。）

### Main spec with BDD feature_refs（point-only 模式）

当 `config.yaml` 定义了 `bdd` 块时，spec 可引用 `.feature` 文件用于可执行 BDD。在 **point-only** 模式下（bdd 已启用 + `feature_refs` 非空），行为定义在 `.feature` 文件中，因此 `requirements`/`scenarios` 可以完全省略：

```toon
kind: llman.sdd.spec
name: sample
purpose: "Behavior lives in the referenced .feature file."
valid_scope[2]: src/,tests/features/sample.feature
feature_refs[1]{path,scope,required}:
  tests/features/sample.feature,acceptance,true
```

- `path`: 相对于项目根的 `.feature` 文件路径
- `scope`: `acceptance` | `unit` | `reference`
- `required`: `true` → 缺失时报 ERROR；`false` → 缺失时报 WARNING
- 若仍提供 `requirements`，其校验规则依旧（statement 必须含 SHALL/MUST），但“每个 requirement 必须有 scenario”的规则会放宽。
- 当 bdd 已启用、无 `feature_refs` 且无 `requirements` 时为 ERROR（必须显式选择一种模式）。

### Delta spec（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.toon`）

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,New requirement,System MUST do the new thing.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",the new trigger happens,the new outcome is observed
```

- `kind` 必须为 `llman.sdd.delta`。
- Delta spec 不携带校验元数据（仅主 spec 需要）。

### 表格化行的引号规则

在表格化数组行中（值以逗号分隔），如果值包含**空格**、**逗号**、**冒号**、**方括号**（`[`, `]`, `{`, `}`）或首尾有空白字符，**必须使用双引号包裹**：

```
# 错误：未加引号的空格值会被拆成多个值
r1,happy,"",a trigger happens,the outcome is observed

# 正确：多词值加引号
r1,happy,"","a trigger happens","the outcome is observed"
```

- 空字符串：`""`
- 未设置的可选字段：`null`
- 不确定时，优先使用引号。

### 备注
- 每个 spec 一个 `.toon` 文件；没有 Markdown，没有 ```` ```toon ```` fence。
- `null` 表示该字段缺失（可选字段未设置）。
- 从旧版 `.md`+fence 迁移请使用 `llman sdd convert`。
