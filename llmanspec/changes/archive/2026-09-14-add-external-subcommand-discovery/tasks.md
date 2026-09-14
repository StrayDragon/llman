# Tasks

垂直切片：每个 task 打穿「实现 → 测试 → 可独立验证」一条窄路径。
测试 seam：CLI 子进程（`tests/bdd_steps.rs` 既有 harness）+ `llman-core` 公共函数；fixture 用 TempDir 假插件（`llman-fake-echo` sh 脚本 + PATH 前置），MUST 全部落在 TempDir 内。

## T1: PATH 解析工具（llman-core）

- [x] 新建 `crates/llman-core/src/ext_subcommand.rs`：`discover()`（去重名字清单）+ `resolve(name) -> Option<PathBuf>`（`llman-<name>` 首个 PATH 命中；`is_file() && mode & 0o111`），Windows stub 编译通过
- [x] llman-core re-export + 单元测试（TempDir 伪造 PATH：命中/未命中/非可执行文件忽略/首目录优先）

## T2: CLI external 臂 + 委托执行 [blocked-by: T1]

- [x] `src/cli.rs`：`Commands::External(Vec<String>)`（`#[command(external_subcommand)]`）；External 臂绕过 `RequiresGlobalConfig` 守卫
- [x] 委托模块：`resolve` → spawn（args 原样、env 注入 `LLMAN_CONFIG_DIR` 归一化——仅 `resolve_config_dir_with` 纯解析，不跑开发守卫；未解析出值则不注入）→ 退出码透传 / unix 信号 `128+signum`
- [x] 集成测试（tests/it 新模块，注册进 `tests/it/main.rs`）：假插件命中委托、argv 原样到达、`llman -C <dir> fake` 时子进程读到 `LLMAN_CONFIG_DIR`、退出码透传（插件 exit 3 → llman exit 3）

## T3: 未找到错误 UX [blocked-by: T1]

- [x] 未命中：stderr 单行 unrecognized 错误 + 已发现插件清单提示（`discover()`），exit 1（对齐 errors-exit r22）
- [x] 集成测试：无插件时报错且含清单提示段（空清单时优雅降级）、exit 1

## T4: BDD 合约绑定 [blocked-by: T2, T3]

- [x] `tests/bdd_steps.rs` 新增 step：假插件 fixture 写入（TempDir + PATH 前置）、argv/env/退出码/stdout 断言、未找到清单提示断言
- [x] `scenarios!` 注册 `cli.feature` 本 change 新增的全部 `@executable` 场景（占位符引号注意 `{mode}` trim 陷阱）

## T5: 文档收尾 [blocked-by: T2]

- [x] `just readme` 重生成（若 CLI surface 输出受影响）——无 diff（命令表不含 external 臂），另补 README 插件机制一节
- [x] `AGENTS.md` / README 如需提及插件机制则补一段（可选，最小化）——README「外部子命令（llman-* 插件）」一节
