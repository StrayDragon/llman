---
name: "llman-sdd-graph"
description: "根据变更提案的 frontmatter（depends_on/blocks）生成依赖关系图。"
---

# LLMAN SDD 依赖图

使用此 skill 可视化变更之间的依赖关系。

## 步骤
1. 运行 `llman sdd graph` 从所有活跃变更生成 mermaid 依赖图。
2. 图表读取每个变更 `proposal.md` YAML frontmatter 中的 `depends_on` 和 `blocks`。
3. 输出到标准输出，可按需管道到文件或渲染器：
   ```
   llman sdd graph > deps.mmd
   llman sdd graph | mmdc -i - -o deps.png
   ```
4. 使用 `--format mermaid` 显式选择格式（mermaid 为默认）。

## 提案 frontmatter 格式

```yaml
---
depends_on:
  - other-change-id
blocks:
  - blocked-change-id
---

## Why
...
```

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
