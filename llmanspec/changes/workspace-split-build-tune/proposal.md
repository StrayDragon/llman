---
depends_on: []
---

目的已决议：**编译速度 + 架构边界**（外置插件协议明确 out-of-scope，远期再议——
当前仅 sdd 一个大模块，真协议有 Speculative Generality 嫌疑，违反仓库自身 smell baseline）。
主体属内部实现重构，不改外部行为 → 以快速路径为主实施；
justfile 配方增补按 r48「治理/工具变更」要求在此记录 why。

## Why

- 单 crate workspace 下全量增量编译串行瓶颈明显；src/sdd ≈ 20k 行占绝对大头，
  与 prompts/tool/x/skills 的演进节奏互相拖累。
- 重依赖使用点已天然隔离（tree-sitter×6 → tool/clean_comments；
  ratatui+crossterm → skills tui_picker；async-openai+tokio → context chat；
  sevenz-rust2 → change/freeze），是现成的 crate 接缝。
- target/ 实测 74GB：5 个 `target/bdd-{sha}` 校验沙箱残积约 13.5GB 已手工清除
  （74G→61G，2026-08-27）；该目录由 validate full mode 为 run_command 隔离而创建、
  用后不回收，长期必然再积。

## What Changes

- **方案 A（workspace 拆 crate）+ 方案 B（features 门控重依赖）组合**：
  - 候选拆分：门面 bin/lib（cli.rs、config、i18n）保持 `llman` 名义不变 +
    内部 crates 按 seam 划分（候选边界 = 上表重依赖模块 + sdd 核）。
    具体切法在 design.md 定稿，依赖方向单向、禁止逆向引用。
  - features 默认集保持当前完整行为不变；贡献者可用 slim 组合（如
    `--no-default-features -F minimal-dev`）显著缩短迭代编译。
- **磁盘卫生**：justfile 新增 `clean-bdd-targets` 类配方清理 `target/bdd-*`
  （本条即工具配置变更的 why 记录）；后续若将沙箱 TTL/复用写进 validate 行为合约，
  另行走 SDD。
- 分期以 expand-contract 小步 PR 推进；预计 live specs 无变更 → `skip_specs_landing: true`。

## Non-goals

- 不做外置子进程插件协议 / PATH 发现机制。
- 不追求发布二进制瘦身（拆 crate 本身不减最终体积；体积问题留给未来的 features 发行组合）。

## Open Questions

- crate 数量与命名：两 crate（facade+sdd）够用还是三 crate（再加 core utils）？
  宁少勿多，避免伪模块化。
- `[profile.dev] debug = 1` 是否实验 `line-tables-only` 进一步压盘（rust-build-tune 建议，
  会丢变量级调试信息）；sccache 是否引入由使用者自配即可，不进 repo 配置。

## Verification Sketch

- 行为面零变化：`just check-all` 全绿；`llman --help` 命令面快照不变。
- 编译对比记录进 design.md：冷/热 `cargo build` 时间与 target/ 占位前后对照。
