# Design — toon-longtail-consistency-purge

## 裁决记录

| # | 议题 | 裁决 | 依据 |
|---|------|------|------|
| D1 | 主库命中 spec.toon 的行为 | **硬 ERROR + 指引 toon2features**，错误消息须友好清晰 | 用户拍板「彻底杜绝旧代码，走全新逻辑无债务」 |
| D2 | `--kind spec-md2toon` 归宿 | **退役**（与 D1 同向）；迁移唯一入口为 toon2features | spec-format r136 已如此合约且有 executable 场景；r115 的「仅保留 md2toon」是把 md 转成被禁止载体的残留笔误级矛盾 |
| D3 | sdd-workflow r60（show 双源分段） | **整条删除**，show/list 的双源渲染码一并拆除 | 用户拍板；单轨世界无「来自 spec.toon 的 requirements」 |
| D4 | discovery 的 legacy 宽容逻辑 | **保留**——重新定性：它是实现 D1 友好报错的探测手段（报「请 migrate」而非「no such file」），非遗留债务 | discovery.rs 注释自述意图与 validation.rs:244 文案互证 |

## 现状勘误（相对探索阶段判断的修正）

- spec-format.feature **无需改动**：r131–r136 即目标态。本 change 主体是让其余能力域文本、
  src 残余读路径与模板向它看齐。
- 命令名对齐：现行子命令为 `llman sdd spec skeleton`（r133）；r114 所写 `scaffold` 作废，
  D3/D2 之外额外多一处命名统一收益。

## 矛盾/过期清单（landing 范围）

| 文件 | 规则 | 处置 |
|------|------|------|
| sdd-workflow r60 | show 分段双源 | 整条删除（D3） |
| sdd-workflow r61 / r100 / r103 / r107 | carrier 措辞 | 收窄为 `<capability>.feature` 单载体表述 |
| sdd-workflow r114 | scaffold 双载体骨架 | 重写：仅生成 `.feature` 骨架，命令名对齐 skeleton |
| sdd-workflow r115 | 「仅保留 spec-md2toon」 | 修正为 D2 口径（reject md2toon, reject partitioned 不变） |
| sdd-context r58 / r79 | Partitioned SSOT 双载体哈希/索引语义 | 重写为单载体：compute_spec_hash 仅哈希 `.feature`；索引仅编入 `.feature` |
| structured-skill-prompts r65 / r96 | 「live `.feature`/`spec.toon`」 | 措辞收窄 |
| bdd-mode-compat r26 | 「validate 校验 spec.toon …」 | 收窄 + 引用 spec-format r131 ERROR 门禁 |
| root AGENTS.md 常用命令行 | 「仅保留 --kind spec-md2toon」 | 同步为 D2 口径（自由区文档，随 landing 一并提交） |

合法保留（不动）：spec-format 全部；r115/r26 中历史语境句（`change/specs/`、delta 废除叙述）；
templates 中「发现遗留请跑 toon2features」的教学句。

## 风险与缓解

- **sdd-context 重写牵连面最大**：pageindex/retrieve/tree 单测多处断言 toon 读路径与
  旧 tree.json `scenarios` 字段兼容 —— 后者为 bdd-mode-compat 明文要求，MUST 保留加载兼容，
  只删「读 toon 内容」，不删「容忍旧缓存结构」。
- 锁定门禁：上述 @human 改动全部依赖 frontmatter `rules_edit_acked: true`（已写入）；
  新增验收尽量做成挂现有 `@req` 的纯 `@executable` 场景（不触碰 @human 哈希集）。
- locales/app.yml 含 md2toon 成功文案与双载体提示串：属 repo 文件非 landing 口径，
  在 apply 的对应切片随实现清理，避免 landing diff 越界。

## 实施形态

expand-contract：本 landing 先把全部目标态口径一次定稿（contract 侧收敛），
apply 按 tasks 六切片拔旧码，最后全量门禁清场。
