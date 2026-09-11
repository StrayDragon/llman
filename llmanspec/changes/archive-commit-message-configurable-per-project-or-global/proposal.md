---
depends_on: [lifecycle-self-expressive]
---

# archive/finalize 自动提交说明可配置（项目级/全局级）

## Why

`lifecycle-self-expressive` 落地后，`change finalize` 收尾会自动生成固定提交说明 `archive(sdd): <change-id>`。固定式对多数项目够用，但不同团队有不同提交规范：有的前缀走 `chore(sdd)`，有的要带标题摘要，有的要求引用工单号（如 `[#123]`）。提交说明属于「各项目/各用户偏好」信息，适合跟随配置而非硬编码。

## What Changes

- **`config.yaml`（项目级）**：`sdd` 段增加可选 `archive_commit_message`（模板字符串，占位符如 `{id}`、`{title}`）；缺省时回退固定式 `archive(sdd): <id>`。
- **全局配置（`LLMAN_CONFIG_DIR`）**：与项目级同构的全局覆盖；项目级 > 全局级 > 内置固定式。
- `change finalize` 在自动提交时按解析顺序取模板生成 message。
- 不自动提交的 CLI 选项（`lifecycle-self-expressive` 引入）与本提案正交，不受影响。

## Capabilities / Impact

- `sdd-workflow`（finalize 自动提交相关条文）
- `config-schemas`（新增 schema 字段）

> 本提案仅记录方向；当前为 draft 草案，由 `lifecycle-self-expressive` 落地的固定式作为缺省行为，之后正式化时再细化模板语法与校验。
