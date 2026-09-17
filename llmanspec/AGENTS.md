<!-- LLMANSPEC:START -->
# llmanspec AGENTS.md

此文件由根目录的 `AGENTS.md` 托管块引用。可在此添加项目特定的规则、
上下文或约定，以便 AI 代理遵守。

<!-- 在此行下方添加你的规则 -->
<!-- LLMANSPEC:END -->
# llman Project Rules

This file is referenced by the root `AGENTS.md`. Use it to add project-specific
rules, context, or conventions that AI agents should follow.

## Change Proposal Frontmatter SSOT

`llmanspec/changes/` 下任意深度（默认扫描深度 8，可用 `llman-sdd --max-scan-depth` 调整）含 `proposal.md` 的目录都是 change；叶子目录名为 change id（可用分组目录组织，如 `changes/<group>/<id>/proposal.md`）。其 `proposal.md` 的 frontmatter（YAML）是**变更元信息的唯一权威**。
正文 MUST NOT 重复声明已在 frontmatter 中声明的字段，否则 SSOT 失效。

> 下文规约锚点 `<capability> r<n>` = `llmanspec/specs/<capability>.feature` 中 `@req:r<n>` 的场景；看全文用 `llman-sdd show <capability>`。

### 合法字段集（规约 sdd-workflow r124 强制）

`llman-sdd validate` 对 frontmatter 做未知字段检测：只接受下表字段，其余（如 `status`、`title`、`priority`、`author`）报 **ERROR**。

| 字段 | 必填 | 谁写入 | 说明 |
|------|------|--------|------|
| `depends_on` | 是（CLI 骨架默认 `[]`） | agent | 依赖的其他 change id 列表 |
| `blocks` | 否 | agent | 反向依赖（阻塞哪些 change） |
| `branch` | 否 | **CLI**（`change start`/`attach`） | attach binding 的 feature 分支 |
| `base_sha` | 否 | **CLI** | attach binding 的 base SHA（`baseSha` 别名已移除，出现即 ERROR） |
| `base_branch` | 否 | **CLI**（`change start`/`attach`，`attach --base <branch>` 可覆盖） | attach binding 的 fork 基准分支，仅用于 finalize/archive 合并目标解析（规约 sdd-workflow r111、r113）；缺键回退本地默认分支，MUST NOT 参与 diff/lock-gate 范围计算 |
| `needs_specs_change` | 否（缺省 `true`） | agent | `false` 时跳过「绑定分支是否改动 `llmanspec/specs/`」检查（规约 sdd-workflow r1） |

> **生命周期阶段不是 frontmatter 字段**：它由 `determine_stage`（规约 sdd-workflow r93）实时从磁盘 artifacts 推断四档（Draft/Designed/Planned/Full），用 `llman-sdd show` / `llman-sdd list` 查看。`status` 字段已废弃——不要再写进 frontmatter，CLI 会拒绝。`checkpointed`/`checkpoint_sha`/`skip_specs_landing`/`rules_edit_acked`/`rules_touched`/`agent_acked` 已移除（出现即 ERROR）。锁定 `@human` 规则的改动为报告制（只出 WARNING，不阻断；规约 spec-format r135）。

### 正文写作约束

- **MUST NOT** 在正文复读 frontmatter 字段：frontmatter 已声明 `branch`/`depends_on` 等，正文就不要再贴同样信息的横幅或 `## Status` 段。
- **MUST NOT** 把 `change_id` 当作 H1 重复（目录名已是 id）。正文 H1 用人类可读标题或省略。
- 正文横幅留给**非元信息**：如「本草案不实现」「前置 change 是 X」「与 Y 案的区别」等叙事说明。
- 生命周期阶段用 `llman-sdd show` / `llman-sdd list` 查看推断的 stage（规约 sdd-workflow r93），**不要**在正文写 status 段，也**不要**在 frontmatter 写 `status` 字段（已被 CLI 拒绝，见规约 sdd-workflow r124）。

## Project Context

Project: llman CLI quality uplift
Primary usage: interactive CLI distributed via cargo install.
Platforms: Linux and macOS are primary; Windows support is partial and not a target.
Compatibility: output and exit codes may change, but changes must be documented per task.

Constraints:
- Do not touch real user config in tests/dev commands; use `LLMAN_CONFIG_DIR`.
- Keep changes incremental and reviewable (one task per PR/merge).

Goals:
- Improve maintainability, readability, and separation of concerns.
- Improve reliability, error signaling, testability, and CI signal quality.
- Improve CLI experience: error messages, help, and consistency.

Non-goals:
- No large new frameworks or rewrites.
- No full Windows support expansion.
- No breaking changes without a documented rollback path.

Guiding principles:
- Small, mergeable steps (one task at a time).
- Prefer shared helpers in `src/config.rs` and `crates/llman-core/src/path_utils.rs` (re-exported as `llman::path_utils`).
- Fail loudly and consistently for errors.
- Avoid risky behavior when parsing or modifying user files.

Risks and mitigations:
- Output and exit code changes can surprise users; document changes and provide examples.
- Config path changes can move data; keep the same default path and add migration notes.
- Safer comment cleaning may remove fewer comments; warn clearly and keep risky fallback opt-in.
- Stricter CI can slow feedback; keep steps minimal and prefer `just check`.

Milestones:
- M1: consistent config path resolution and error/exit handling.
- M2: cursor prompts / sync-ignore correctness and safer tool behavior (stats/export removed).
- M3: quality gates (fmt/clippy) and message consistency.

Acceptance overview:
- `cargo +nightly fmt -- --check` passes.
- `cargo +nightly clippy --all-targets --all-features -- -D warnings` passes.
- `cargo +nightly test --workspace` passes.
- Manual smoke checks for `llman x cc`, `llman x codex`, `llman x cursor`, `llman prompt`, `llman tool`.
