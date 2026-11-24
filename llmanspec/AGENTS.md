<!-- LLMANSPEC:START -->

# llmanspec AGENTS.md

此文件由根目录的 `AGENTS.md` 托管块引用。可在此添加项目特定的规则、
上下文或约定，以便 AI 代理遵守。

<!-- 在此行下方添加你的规则 -->
<!-- LLMANSPEC:END -->

# llman Project Rules

写法见根 `AGENTS.md`「AGENTS.md 写法」。本文件只放 SDD 工件约定与本仓库约束。

## Change Proposal Frontmatter SSOT

`llmanspec/changes/` 下含 `proposal.md` 的叶子目录都是 change；叶子目录名是 change id（可用分组目录组织）。frontmatter 是变更元信息的唯一权威。正文不要复读 frontmatter 已有字段。

`llman-sdd validate` 只接受下表字段，其余报 ERROR。生命周期 stage 由磁盘工件推断，用 `llman-sdd show` / `list` 查看，不要写进 frontmatter 或正文。

| 字段 | 必填 | 谁写入 | 说明 |
|------|------|--------|------|
| `depends_on` | 是（骨架默认 `[]`） | agent | 依赖的其他 change id |
| `blocks` | 否 | agent | 阻塞哪些 change |
| `branch` | 否 | CLI（`change start` / `attach`） | 绑定的 feature 分支 |
| `base_sha` | 否 | CLI | 绑定审计用 SHA，不参与 diff 范围 |
| `base_branch` | 否 | CLI（`attach --base` 可覆盖） | finalize / archive 的合并目标；缺键回退本地默认分支 |
| `needs_specs_change` | 否（缺省 `true`） | agent | `false` 时跳过「绑定分支是否改了 `llmanspec/specs/`」检查 |

正文：

- 不要复读 frontmatter（不要横幅贴 `branch` / `depends_on`，不要 `## Status`）。
- 不要把 change id 当 H1；H1 用人类可读标题或省略。
- 横幅只写非元信息叙事（「本草案不实现」「与 Y 案的区别」）。
- 改 `@human` 规则只出 WARNING，不阻断。

## Project constraints

- 主平台 Linux / macOS；Windows 不是目标。
- 对外输出与退出码可以变，但必须在对应 change 里写明。
- 测试与开发命令不要碰真实用户配置，用 `LLMAN_CONFIG_DIR`。
- 一次一事，可审查；不要引入大型新框架或重写。
- 不要为 Windows 做完整支持扩张。
- 破坏性变更要有文档化回退路径。
- 共享逻辑优先复用 `crates/llman-core/` 与 `src/` 已有辅助。
- 解析或改用户文件时避免冒险默认行为。
