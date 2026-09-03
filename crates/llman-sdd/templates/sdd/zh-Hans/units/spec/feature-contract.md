## 单轨 Feature 合约规范

每个 capability 只有一个 Gherkin 文件：`llmanspec/specs/<capability>/<capability>.feature`。
它是唯一的 spec 工件——不存在 `spec.toon`。

```gherkin
# language: zh-CN
# capability: sample
# purpose: One-line overview.
# scope: src/

功能: sample

  @req:r1 @human
  场景: Rule title
    System MUST do something.

  @req:r1 @executable
  场景: happy
    假如 a precondition
    当 a trigger happens
    那么 the outcome is observed
```

- 头注释（`# capability:` / `# purpose:` / `# scope:`）必填；`scope` 驱动 staleness 检查。
- `@human` 场景是人拥有的约束场景；规则 statement 全文放在场景描述里。修改/删除须在 change 提案 frontmatter 中带 `rules_edit_acked: true`。
- `@executable` 场景是 runner 绑定的验收场景；用 `@req:<req_id>` 挂回规则。
- 覆盖三态分级：enforced（有验收）/ manual（`@manual`）/ pending——`list --specs` 逐项输出。
- 场景 MUST 保持顶层：`Rule:` 块会被拒绝（runner 会静默跳过其中场景）。
