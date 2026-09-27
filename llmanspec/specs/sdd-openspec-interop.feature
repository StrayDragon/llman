# language: zh-CN
# capability: sdd-openspec-interop
# purpose: 规范 `llman sdd import/export` 与 OpenSpec 目录的双向互转行为合约。
# scope: llmanspec/specs/sdd-openspec-interop.feature

功能: sdd-openspec-interop

  @req:r30
  规则: OpenSpec 双向互转命令与安全门禁
    对应 spec: sdd-openspec-interop — 上游 llman-sdd 0.5 CLI 未提供 openspec import/export 命令面（project 子命令仅 dedupe-req-ids 与 migrate）：`llman sdd project import` / `export` MUST 以 unknown command 非零退出。若上游未来恢复该能力，应另开 change 重写本规则。
    场景: openspec-import-absent
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd project import --style openspec
      那么 退出码非零
      那么 stderr 包含 unknown command
  @req:r63
  规则: 迁移范围、冲突策略、旧目录删除与元数据补齐
    迁移范围/冲突策略/元数据补齐等行为属该未提供命令面的内部契约——上游 0.5 CLI 无 import/export，MUST 以 unknown command 非零退出（缺席即契约）。

    场景: openspec-export-absent
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd project export --style openspec
      那么 退出码非零
      那么 stderr 包含 unknown command
