<!-- llman-template-version: 1 -->
常见校验修复（TOON 风格）：

1) Main spec 缺少 YAML frontmatter（仅 main spec 需要）：
```markdown
---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - llman sdd validate <feature-id> --type spec --strict --no-interactive
llman_spec_evidence:
  - <evidence>
---
```

2) Main spec 缺少 canonical ` ```toon ` payload（`llmanspec/specs/<feature-id>/spec.md`）：
```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
requirements[1]{req_id,title,statement}:
  r1,Title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

3) Change 缺少 delta ops：至少补一个 op + scenario（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）：
```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,Title,System MUST do something.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

备注：
- `toon` 文件必须只有一个 ` ```toon ` fence。
- `null` 表示可选字段缺失。
- `toon` 为 experimental：跨风格迁移请使用显式的 `llman sdd convert`。
