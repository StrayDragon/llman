---
depends_on: []
branch: sdd/add-external-subcommand-discovery
base_sha: 253123f2acee327a972eff870bba994cdec24da7
base_branch: main
---

# 外部子命令自动发现（llman-* PATH 插件合约）

> 本 change 仅覆盖发现与委托机制（探索阶段的 A 系列）。llman-sdd 子系统的拆分/重写不在范围内；但本合约是其未来换语言（如 TS）落地时的唯一接入通道，因此所有条款 MUST 语言无关。

## Why

- `Commands` 是封闭 clap 枚举（`src/cli.rs`），第三方无法在不改主程序、不重新编译的情况下扩展 llman 的 CLI 面。
- git 的 `git-*` 外部子命令模式证明了「PATH 上按前缀约定发现可执行文件」是成本最低、可演化性最好的插件机制：安装即接入，无需注册表/清单。
- 未来 llman-sdd 可能以 TS 重写为独立二进制；本 change 定义跨进程合约（命名/argv/env/stdio/cwd/退出码），使任何语言实现的 `llman-*` 可执行文件获得与内置命令一致的调用体验。

## What Changes

- CLI 新增**外部子命令委托**：`llman <name> <args...>` 未命中内置命令时，在 `PATH` 上解析 `llman-<name>` 可执行文件并以子进程执行。
- 六项**跨语言插件合约**：
  1. **命名**：`llman-<name>` ↔ `llman <name>`；内置命令自动优先（clap external 臂仅在不匹配任何内置时触达）。
  2. **argv**：`<name>` 之后的参数原样逐字转发，hub 不解析、不转写。
  3. **env**：全量继承 + 一项归一化——委托前将 hub 解析出的 config dir（优先级见 config-paths r17）注入子进程 `LLMAN_CONFIG_DIR`（`-C/--config-dir` 是 hub 的 argv 参数，子进程天然不可见，必须显式传递）。v1 不新增任何其他注入变量。
  4. **stdio**：stdin/stdout/stderr 全继承（TTY 直通，交互式插件可用）。
  5. **cwd**：继承。
  6. **退出**：退出码精确透传；Unix 下子进程被信号杀死则以 `128+signum` 退出（shell 惯例）。
- **未找到错误 UX**：`llman <name>` 在 PATH 上无 `llman-<name>` 时，输出 unrecognized 错误 + PATH 上已发现的 `llman-*` 插件清单提示（替代被 external 臂关闭的 clap did-you-mean）。
- **命名空间声明**：`LLMAN_*` 前缀为 llman 跨进程保留命名空间，插件程序 MUST NOT 注入或覆写 `LLMAN_*` 变量后回传假设（仅读）。
- 同名多命中取 PATH 靠前目录（与 `which` 一致）；Windows PATHEXT 解析（`.exe/.cmd/.bat/.ps1`，含 BatBadBut 转义陷阱）作为设计预留，实现为后续项，本 change 仅 target Unix 语义。

## Capabilities

- `cli`：新增一条 `@req`（外部子命令委托合约六项 + 未找到错误 UX）+ `@executable` 验收场景（委托执行、`-C` 归一化、退出码透传、未找到清单提示）。

## Impact

- 代码：`src/cli.rs`（external 臂 + 委托）、`crates/llman-core`（PATH 解析工具，供 hub 与测试复用）、`tests/bdd_steps.rs`（新 step + 假插件 fixture）。
- 内置命令行为零改动；`RequiresGlobalConfig` 守卫不适用于外部委托（子进程自行解析配置，hub 不代跑守卫）。
- README 命令表需重生成（`just readme`）。
- 已知取舍：加 external 臂后 clap 不再对未知子命令自动报错/建议，typo 兜底由委托臂的错误 UX 承担（与 git 相同的取舍）。
