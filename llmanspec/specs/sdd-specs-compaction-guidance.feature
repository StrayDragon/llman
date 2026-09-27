# language: zh-CN
# capability: sdd-specs-compaction-guidance
# purpose: 规范 SDD specs 压缩治理技能的生成与流程要求。
# scope: llmanspec/specs/sdd-specs-compaction-guidance.feature

功能: sdd-specs-compaction-guidance

  @req:r31
  规则: specs 压缩 CLI 预留未实现且治理基于事实源并含安全门
    对应 spec: sdd-specs-compaction-guidance — 当前版本 MUST NOT 实现 specs compact CLI 子命令；压缩治理 MUST 以代码与 specs 为事实源（而非已废弃的 ISON 制品）； 且 MUST 包含压缩前后安全回归比对步骤。

    场景: specs-compact-cli-absent
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd specs compact
      那么 退出码非零
      那么 stderr 包含 unknown command
  @req:r64
  规则: specs 压缩治理技能可生成且含 freeze 建议
    对应 spec: sdd-specs-compaction-guidance — llman sdd init --update MUST 生成 llman-sdd-specs-compact 技能，提供 specs 压缩治理流程；且在 archive 历史噪声较大时 建议先执行 freeze。

    场景: specs-compact-skill-generated
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      那么 渲染技能 llman-sdd-specs-compact 存在
      那么 渲染技能 llman-sdd-specs-compact 内容包含 freeze
