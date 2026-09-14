# Design: 外部子命令自动发现

## 决策记录（探索阶段已定）

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 发现方式 | git 模式：动态扫描 PATH，无注册表/清单 | 「自动发现」诉求；安装即接入；无清单漂移 |
| 错误 UX 未找到插件 | unrecognized + PATH 上已发现 `llman-*` 清单 | 用户已确认（成本低价值高） |
| v1 新增注入 env | 零新增；仅 `-C` 归一化注入 `LLMAN_CONFIG_DIR` | 用户已确认；命名空间规则入合约 |
| Windows | 设计预留（PATHEXT），实现后续项 | 项目当前仅 target Unix |

## 机制设计

### clap 路由与优先级

```rust
#[derive(Subcommand)]
enum Commands {
    // ...内置命令不变...
    #[command(external_subcommand)]
    External(Vec<String>),   // args[0] = <name>, args[1..] = 原样参数
}
```

- 内置命令自动优先：clap 仅在无一内置匹配时落入 External 臂，无需手写遮蔽逻辑。
- `<name>` 之后的全部 argv 归插件（含 `-C` 等短横线参数，原样转发）；hub 全局选项只认 `<name>` 之前的位置（与 git 一致）。
- 已知代价：clap 不再对未知子命令报错，typo 兜底由委托臂复现（unrecognized + 插件清单提示）。

### 委托流程

```
External([name, rest..])
  → llman_core::ext_subcommand::resolve("llman-{name}")   // PATH 逐目录，首个命中
     ├─ None     → stderr: unrecognized + 已发现插件清单；exit 1（走 errors-exit r22 单行错误约定）
     └─ Some(p)  → Command::new(p)
                     .args(rest)                            // 原样
                     .env("LLMAN_CONFIG_DIR", resolved)     // 仅当 hub 侧可解析出值
                     .spawn() + wait()
                     → 退出码透传；unix 信号死 → 128+signum
```

- env 归一化只调 `resolve_config_dir_with`（纯解析：`-C` > env > 缺省，含 tilde 展开，config-paths r17/r4 语义），**不调用** `determine_config_dir` 的开发仓库守卫，也不跑 `ensure_global_sample_config`——External 臂完全绕过 `RequiresGlobalConfig`（子进程自行解析配置）。
- 两者都未提供时**不注入**，子进程按自身缺省解析（`~/.config/llman`），行为与直接运行插件一致。
- stdio/cwd 用 `Command` 默认继承行为，不显式改动。

### PATH 解析工具（llman-core）

- 新模块 `crates/llman-core/src/ext_subcommand.rs`（re-export 进 llman）：
  - `discover() -> Vec<ExternalCommand>`（去重后名字清单，供错误 UX 提示）
  - `resolve(name) -> Option<PathBuf>`（`llman-<name>`，首个 PATH 命中）
- Unix 判定：`is_file() && mode & 0o111 != 0`（覆盖 ELF / shebang 脚本 / npm sh shim）。Windows：编译通过的 stub（返回空），PATHEXT 实现留 TODO。

### 测试 seam（已确认）

- 唯一行为 seam = **CLI 子进程**（现有 harness：`tests/bdd_steps.rs` 的 `run_llman`/`run_llman_in`，`CARGO_BIN_EXE_llman`）。
- 假插件 fixture：TempDir 内生成可执行 `llman-fake-echo`（sh 脚本，echo 收到的 argv/env），将该目录前置进子进程 `PATH` 后运行 llman 断言转发行为。全程 TempDir，无固定路径。
- `llman-core` 单元测试直接用 TempDir 伪造 PATH 目录结构，不起子进程。
