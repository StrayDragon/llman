---
depends_on:
- rework-finalize-merge-semantics
branch: sdd/fix-r98-finalize-cleanup
base_sha: 7022218864ca6b900e3e5867bb3f165c67595193
base_branch: main
---

# r98 收尾对齐：squash 后清理用 -D + finalize next-step 提示行

## Why

`rework-finalize-merge-semantics` 落地 squash 缺省后，r98 暴露两处偏差（本 change 对其自举修复）：

1. **r98 文本缺陷（上一 change 引入）**：手动兜底命令后写「可选 `git branch -d <feature>`」，但 squash 收口后 feature 分支不再是目标分支的祖先，`git branch -d` 会被 git 以「not fully merged」拒绝——正确命令是 `git branch -D`（模板层已在 quick 路径修掉同类措辞，本 change 修 SSOT 本体）。
2. **r98 MUST 未实现（存量偏差）**：r98 要求「`finalize` 成功 stdout MUST 在归档提示后追加一行 next-step」，`finalize.rs` 自始未打印该行（E2E 输出可证）。补实现而非改 spec——MUST 语义本身合理。

## What Changes

- live spec `sdd-structured-skill-prompts.feature` r98：`可选 git branch -d <feature>` → `可选 git branch -D <feature>（squash 后分支不再是目标分支祖先，-d 会被拒绝）`。
- `finalize.rs`：auto-commit 成功路径在 `finalized change ...` 行后追加一行 next-step（指引在合并目标分支确认收口 commit；push / hosting PR 仅为可选）；幂等重试路径维持现状。测试断言同步。

## Capabilities / Impact

- `sdd-structured-skill-prompts`（r98 文本）
- `crates/llman-sdd/src/sdd/change/finalize.rs`（next-step 行 + 单测断言）

## 测试边界（seam）

- 单测：`finalize.rs` 既有 `finalize_auto_commits_archive_message` 补断言 stdout/输出含 next-step 行（函数内 println，经捕获验证或最小化拆分）。
- E2E：/tmp 最小仓 finalize 输出含 `next:` 行。
