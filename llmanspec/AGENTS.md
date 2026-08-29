# llman Project Rules

This file is referenced by the root `AGENTS.md`. Use it to add project-specific
rules, context, or conventions that AI agents should follow.

## Change Proposal Frontmatter SSOT

`llmanspec/changes/` 下任意深度（默认扫描深度 8，可用 `llman sdd --max-scan-depth` 调整）含 `proposal.md` 的目录都是 change；叶子目录名为 change id（可用分组目录组织，如 `changes/<group>/<id>/proposal.md`）。其 `proposal.md` 的 frontmatter（YAML）是**变更元信息的唯一权威**。
正文 MUST NOT 重复声明已在 frontmatter 中声明的字段，否则 SSOT 失效。

### 合法字段集（r124 强制）

`llman sdd validate` 对 frontmatter 做未知字段检测：只接受下表字段，其余（如 `status`、`title`、`priority`、`author`）报 **ERROR**。

| 字段 | 必填 | 谁写入 | 说明 |
|------|------|--------|------|
| `depends_on` | 是（CLI 骨架默认 `[]`） | agent | 依赖的其他 change id 列表 |
| `blocks` | 否 | agent | 反向依赖（阻塞哪些 change） |
| `branch` | 否 | **CLI**（`change start`/`attach`） | attach binding 的 feature 分支 |
| `base_sha`（或 `baseSha`） | 否 | **CLI** | attach binding 的 base SHA |
| `checkpointed` | 否 | **CLI**（`checkpoint`） | 是否已 checkpoint |
| `checkpoint_sha`（或 `checkpointSha`） | 否 | **CLI** | checkpoint 的 SHA |
| `skip_specs_landing` | 否 | agent | `true` 时无 live `llmanspec/specs/**` 变更也可 `readyToImplement` |
| `rules_edit_acked` | 否 | 人工确认后由 agent 写入 | `true` 时允许本 change 修改/删除锁定的 `@human` 规则场景（spec-format r135） |

> **生命周期阶段不是 frontmatter 字段**：它由 `determine_stage`（r93）实时从磁盘 artifacts 推断（Draft/Designed/Full），用 `llman sdd show` / `llman sdd list` 查看。`status` 字段已废弃——不要再写进 frontmatter，CLI 会拒绝。

### 正文写作约束

- **MUST NOT** 在正文复读 frontmatter 字段：frontmatter 已声明 `branch`/`depends_on` 等，正文就不要再贴同样信息的横幅或 `## Status` 段。
- **MUST NOT** 把 `change_id` 当作 H1 重复（目录名已是 id）。正文 H1 用人类可读标题或省略。
- 正文横幅留给**非元信息**：如「本草案不实现」「前置 change 是 X」「与 Y 案的区别」等叙事说明。
- 生命周期阶段用 `llman sdd show` / `llman sdd list` 查看推断的 stage（r93），**不要**在正文写 status 段，也**不要**在 frontmatter 写 `status` 字段（已被 CLI 拒绝，见 r124）。

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
- Prefer shared helpers in `src/config.rs` and `src/path_utils.rs`.
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
- `cargo +nightly test --all` passes.
- Manual smoke checks for `llman x cc`, `llman x codex`, `llman x cursor`, `llman prompt`, `llman tool`.
