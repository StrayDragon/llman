# language: zh-CN
# capability: sdd-specs-compaction-guidance
# purpose: 规范 SDD specs 压缩治理技能的生成与流程要求。
# scope: llmanspec/specs/sdd-specs-compaction-guidance

功能: sdd-specs-compaction-guidance

  @req:r31 @human
  场景: specs 压缩 CLI 预留未实现且治理基于事实源并含安全门
    - 对应 spec: sdd-specs-compaction-guidance — 当前版本 MUST NOT 实现 specs compact CLI 子命令；压缩治理 MUST 以代码与 specs 为事实源（而非已废弃的 ISON 制品）； 且 MUST 包含压缩前后安全回归比对步骤。

  @req:r64 @human
  场景: specs 压缩治理技能可生成且含 freeze 建议
    - 对应 spec: sdd-specs-compaction-guidance — llman sdd init --update MUST 生成 llman-sdd-specs-compact 技能，提供 specs 压缩治理流程；且在 archive 历史噪声较大时 建议先执行 freeze。
