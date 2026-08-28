---
depends_on: []
skip_specs_landing: true
branch: sdd/src-cleanup-pre-split
base_sha: 0728fc36a51f97d66f1a2ffc0c0c4f9bde6d9ea5
checkpointed: true
checkpoint_sha: 0728fc36a51f97d66f1a2ffc0c0c4f9bde6d9ea5
---

目的已决议：**代码组织 + 去重（避免 slop）为主，编译速度为辅**；最终形态是
`llman-sdd` 独立 crate（含 sdd core 层），以拆 crate 收尾。
主体属内部实现重构，不改外部行为 → 以快速路径为主实施；
justfile 配方增补按 r48「治理/工具变更」要求在此记录 why。

方向演进（2026-08-28）：不做 features 门控——`cargo install llman` 必须直接获得
全部功能，不向用户暴露 feature 选择（原 id 中 build-tune 口径随之失效，change
更名为 src-cleanup-pre-split）。路径为「宏观（src 全局组织与去重）→
微观（sdd 内部组织与去重）→ 拆 crate」，一步一步来，三阶段同属本 change。

## Why

- 重复与漂移经扫描实测（2026-08-28），主要证据：
  - sdd/shared 自身：`print_json` 同签名 ×3（list.rs/show.rs/validate.rs）、
    `non_interactive_hint_message` ×2（show.rs/validate.rs）；
  - 跨模块小工具漂移：`is_symlink_dir` ×2（skills/cli 与 skills/catalog）、
    `slugify` ×2（skills/catalog 与 sdd/project/interop，签名还不同）、
    `normalize_newlines` ×2（tool/sync_ignore 与 sdd/spec/frontmatter）、
    `push_unique` ×2（tool/sync_ignore 与 sdd/project/config）；
  - git 操作分裂：`sdd::change::git_native` 与 `skills::shared::git::find_git_root`
    并存，`init_repo` ×4、`git_ref_exists` ×2；
  - x/claude_code 与 x/codex 有 19 个同名命令流函数；抽查证实两类：
    纯叶子逐字相同（`mask_secret`），命令流骨架逻辑同构、差异几乎全是
    i18n key 前缀与模板路径（`handle_account_edit_with` 56/50 行）；
  - Config 层 sprawl：15+ 个 Config 结构分散在 config.rs / config_schema.rs /
    tool/config.rs / skills/config / sdd/project/config.rs 五处，
    `x::codex::Config` 与全局 `Config` 重名，`ToolConfig` 与 `ToolsConfig` 近名。
  - 五处 config 判定为「各有其职」，归属本身正确；问题在环、近名与横切点过载。
- 架构边界问题：`config_schema`(顶层) ↔ `sdd::project` 成环（schema 生成与
  校验工具互相引用），且 config_schema 同时承载三份 schema 定义 + 通用校验工具，
  横切点过载；tool 向上引用 `sdd::change::git_native`；
  sdd 是天然叶子（对外仅 fs_utils / config_schema / managed_block / env_safety
  共 12 处引用），方向规则值得用测试锁定而非靠自觉。
- 单 crate 的痛点是冷/全量构建无法并行 type-check；拆 `llman-sdd` crate 是
  最终解，但先清理再搬运，避免把 slop 一起搬过去。
- target/ 实测 74GB：5 个 `target/bdd-{sha}` 校验沙箱残积约 13.5GB 已手工清除
  （74G→61G，2026-08-27）；该目录由 validate full mode 为 run_command 隔离而创建、
  用后不回收，长期必然再积。

## What Changes

三阶段推进，每阶段以 expand-contract 小步 PR 落地，行为面零变化。

- **阶段 1（宏观：src 全局组织与去重）**：
  - 解环 + 拆 config_schema（已决议）：`config_schema.rs` 拆成两块——
    schema 定义（GlobalConfig/ProjectConfig 等结构与 generate）留在
    config_schema；`validate_yaml_value`/`prepend_schema_header` 等通用校验
    工具下沉顶层工具层。llmanspec schema 生成（`generate_schema::<SddConfig>`）
    移入 `sdd::project::config`，之后 config_schema 单向调用 sdd，环消失。
  - git 工具归并：`skills::shared::git` + `sdd::change::git_native`
    （+ `init_repo`/`git_ref_exists` 重复实例）→ 顶层统一 git 工具模块，
    消掉 tool 与 skills 的向上/横向引用。
  - 小工具去重：`is_symlink_dir`/`slugify`/`normalize_newlines`/`push_unique`
    等语义相同者合并进顶层工具层，语义不同者改名以示区分（防错误抽象）。
  - Config 消歧：`x::codex::Config` → `CodexConfig`；
    `ToolConfig`/`ToolsConfig` 查实际用途后合并或改名消近名；
    五处 config 模块所有权不动，配置文件格式不变。
  - 方向规则用 arch test 锁定（扫描 `use crate::` 断言 sdd/skills/tool/x
    分层表），挂进 `just check`；sdd 对外可见性收敛（`pub(crate)` 纪律）。
- **阶段 2（微观：sdd 内部组织与去重）**：
  - sdd/shared 自身去重（`print_json`、`non_interactive_hint_message` 等）。
  - 子模块间重复收敛（`proposal_path`/`change_dir`/`list_specs`/`normalize_type`/
    `init_repo`/git 操作等）。
  - change/project/shared/spec/context/authoring 六个子块边界检视与
    可见性纪律，产出拆 crate 的边界图输入。
  - x 双 provider 去重（已决议「先叶子后定骨架」）：本 change 仅抽逐字相同
    叶子入 `src/x/shared/`；命令流骨架参数化与 i18n key 命名空间统一
    **不在本 change**，叶子落地后依据实际分歧清单单独评估（后续 change 或放弃）。
- **阶段 3（拆 crate，本 change 收尾）**：
  - 抽 `llman-sdd` crate（门面 `pub use` 保持导入路径不变）；内部是否再分
    sdd core 层由 design.md 依据阶段 2 检视结果定稿。
  - 阶段 1/2 沉淀的顶层工具层即未来 core crate 的雏形。
  - 弹性条款：若阶段 1/2 实施中发现体量显著超预期，阶段 3 可降级为
    另立 change（届时 design.md 冻结的拆分蓝图直接复用），不硬塞。
- **磁盘卫生**：justfile 新增 `clean-bdd-targets` 类配方清理 `target/bdd-*`
  （本条即工具配置变更的 why 记录）；沙箱 TTL/复用若要进行为合约，另行 SDD。
- **编译基线记录**：design.md 记录冷/热 `cargo build` 时间与 target/ 占位，
  拆 crate 前后各测一次（无 features 门控，收益唯一来源是 crate 边界本身）。

## Non-goals

- **不做 features 门控**：`cargo install llman` 即全功能，不引入
  `--no-default-features`/minimal 组合，不向用户暴露 feature 列表。
- 不做 x 双 provider 的命令流骨架参数化与 i18n key 命名空间统一（见阶段 2 决议）。
- 不做外置子进程插件协议 / PATH 发现机制。
- 不追求发布二进制瘦身。
- 不改任何 YAML/JSON/TOML 配置文件格式与 CLI 行为面（去重与归位是纯重构）。

## Open Questions

- 拆 crate 的 crate 边界图终稿：依赖阶段 2 检视结果，propose 时给出草案
  （sdd 六子块 → crate 内模块 vs 独立 crate；顶层工具层 → core crate 雏形）。
- `[profile.dev] debug = "line-tables-only"` 压盘实验的取舍
  （会丢变量级调试信息），随编译基线一并进 design.md。

## Verification Sketch

- 行为面零变化：`just check-all` 全绿；`llman --help` 命令面快照不变；
  配置文件读写格式不变（现有测试覆盖）。
- arch test 生效：方向规则表全部通过，且断言本身在 `just check` 中被执行。
- 去重以扫描复核：同名同义函数在目标层（顶层工具层 / sdd/shared / x/shared）
  各至多一份。
- 编译对比记录进 design.md：冷/热 `cargo build` 时间与 target/ 占位
  拆 crate 前后对照。