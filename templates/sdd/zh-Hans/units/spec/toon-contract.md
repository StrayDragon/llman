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

### Main spec BDD-on（solidify 工作流）

当 `config.yaml` 定义了 `bdd` 块时，行为规格在 `spec.toon` 中——结构与 BDD-off 相同。`.feature` 文件是 `llman sdd solidify` 从 `scenarios` 表生成的**衍生工件**：

```toon
kind: llman.sdd.spec
name: sample
purpose: "所有行为定义在下面的 requirements + scenarios 中。"
valid_scope[1]: llmanspec/specs/sample
requirements[1]{req_id,title,statement}:
  r1,新增需求,系统 MUST 完成新功能。
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,happy,"llman 二进制已构建","运行 llman sample --flag","退出码为 0 且 stdout 包含预期内容",true
```

- `feature: true`（默认）：`solidify` 将该 scenario 写入 `.feature` 文件。
- `feature: false`：留在 TOON 内仅作文档（如内部行为描述、自指 validate 场景）。
- propose 时不要创建 `.feature` delta 文件——仅 TOON `spec.toon`。
- apply 完成后运行 `llman sdd solidify <change-id>` 重新生成 `.feature` 文件。
- BDD 已启用、`requirements` 和 `scenarios` 均为空的 spec 是 ERROR。

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
- 从旧版 `.md`+fence 迁移请使用 `llman sdd migrate`。
