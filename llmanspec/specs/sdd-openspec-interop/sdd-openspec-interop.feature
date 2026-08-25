# language: zh-CN
# capability: sdd-openspec-interop
# purpose: 规范 `llman sdd import/export` 与 OpenSpec 目录的双向互转行为合约。
# scope: llmanspec/specs/sdd-openspec-interop

功能: sdd-openspec-interop

  @req:r30 @human
  场景: OpenSpec 双向互转命令与安全门禁
    - 对应 spec: sdd-openspec-interop — 系统 MUST 提供 import/export --style openspec 双向互转； --style 必填且仅允许 openspec；import/export MUST 默认先 dry-run，交互终端双确认后写入， 非交互拒绝写入返回非零。

  @req:r63 @human
  场景: 迁移范围、冲突策略、旧目录删除与元数据补齐
    - The system MUST satisfy the harness scenarios for `迁移范围、冲突策略、旧目录删除与元数据补齐`: 对应 spec: sdd-openspec-interop — 迁移范围含 specs/active changes/archive；同名冲突即失败 且不覆盖；非标准目录输出 warning 并按相对路径复制；迁移成功后交互式默认不删除旧目录； export 补齐 OpenSpec 元数据，import 补齐 valid_scope。
