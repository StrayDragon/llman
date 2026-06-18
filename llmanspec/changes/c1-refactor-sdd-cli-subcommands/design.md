# Design: Refactor `llman sdd` CLI Subcommands

## 现状分析

### 当前子命令树

```
llman sdd
├── init                    # 初始化 llmanspec
├── list                    # 列出 changes 或 specs
├── show                    # 查看 change 或 spec
├── validate                # 校验 changes/specs
├── archive                 # 归档 workflow
│   ├── run                 # 执行归档
│   ├── freeze              # 冻结归档目录
│   └── thaw                # 解冻归档
├── spec                    # Spec 编辑助手
│   ├── skeleton            # 生成 main spec 骨架
│   ├── add-requirement     # 添加 requirement
│   └── add-scenario        # 添加 scenario
├── delta                   # Delta 编辑助手
│   ├── skeleton            # 生成 delta spec 骨架
│   ├── add-op              # 添加 delta op
│   └── add-scenario        # 添加 delta scenario
├── graph                   # 生成依赖图
├── orphans                 # 检测孤立 defer items
├── import                  # 从 OpenSpec 导入
├── migrate                 # 迁移 spec 格式
└── upgrade-guide           # 输出升级指南
```

### 问题详细分析

#### 问题 1：`archive` 的 Legacy 别名导致解析歧义

```rust
// 当前实现：Archive 结构体同时接受位置参数和子命令
pub struct Archive {
    /// Legacy: change id
    change: Option<String>,
    /// Legacy: skip updating specs
    skip_specs: bool,
    /// Legacy: dry run mode
    dry_run: bool,

    #[command(subcommand)]
    command: Option<ArchiveSubcommand>,
}
```

**问题**：
- `llman sdd archive c1` 是 legacy 语法（等同于 `archive run c1`）
- `llman sdd archive run c1` 是现代语法
- 两种方式并存，增加维护成本和用户困惑
- `--skip-specs`、`--dry-run` 等选项在两个层级重复定义

**影响**：代码冗余 ~50 行，用户文档需解释两种调用方式

#### 问题 2：`spec` 和 `delta` 子命令命名不一致

| 操作 | spec 子命令 | delta 子命令 | 差异 |
|------|-------------|--------------|------|
| 添加需求 | `add-requirement` | `add-op` | 命名完全不同 |
| 添加场景 | `add-scenario` | `add-scenario` | 一致 |
| 重命名 | N/A | `add-op --op rename` | 隐藏在 op 参数中 |
| 删除 | N/A | `add-op --op remove` | 隐藏在 op 参数中 |

**问题**：
- `add-op` 是一个通用名称，实际有 4 种不同操作（add/modify/remove/rename）
- 用户需要记住 `--op` 参数的可选值
- `spec` 和 `delta` 的对应命令名称不同

#### 问题 3：`show` 命令选项过多

```rust
pub struct ShowArgs {
    item: Option<String>,
    json: bool,
    compact_json: bool,
    meta_only: bool,
    item_type: Option<String>,
    no_interactive: bool,
    deltas_only: bool,
    requirements_only: bool,  // deprecated
    requirements: bool,
    no_scenarios: bool,
    requirement: Option<usize>,
}
```

**问题**：11 个选项，组合复杂，用户难以记忆

#### 问题 4：非核心命令位置不当

`import`、`migrate`、`upgrade-guide` 是项目管理/迁移工具，不属于核心 SDD 工作流：
- `import`: 一次性迁移工具（从 OpenSpec 导入）
- `migrate`: 一次性迁移工具（格式迁移）
- `upgrade-guide`: 文档生成工具

这些命令应该在 `sdd project` 子组下，与 `init`、`update-skills` 等命令一起。

#### 问题 5：缺少项目状态概览

当前没有快速查看项目整体状态的命令。用户需要运行多个命令：
- `llman sdd list` 查看变更数量
- `llman sdd validate --all` 检查是否有问题
- `llman sdd list --specs` 查看 specs 数量

#### 问题 6：`delta add-op` 的 `--op` 参数设计

```rust
pub struct DeltaAddOpArgs {
    op: String,  // "add_requirement|modify_requirement|remove_requirement|rename_requirement"
    req_id: String,
    title: Option<String>,      // 仅 add/modify 需要
    statement: Option<String>,  // 仅 add/modify 需要
    from: Option<String>,       // 仅 rename 需要
    to: Option<String>,         // 仅 rename 需要
    name: Option<String>,       // 仅 remove 可选
}
```

**问题**：
- 根据 `op` 值，大部分参数都是可选的
- 无法在 clap 层面强制要求特定参数组合
- 运行时才能验证参数完整性

## 设计方案

### 方案 A：渐进式重构（推荐）

保持向后兼容，分阶段迁移：

#### 阶段 1：添加 `status` 命令

```rust
#[derive(Subcommand)]
pub enum SddCommands {
    // ... existing commands

    /// Show project status overview
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
```

实现逻辑：
```rust
fn run_status(json: bool) -> Result<()> {
    let changes = list_changes()?;
    let specs = list_specs()?;
    let validation = validate_all(true)?;

    if json {
        // 输出 JSON
    } else {
        // 输出表格
        println!("Active changes: {}", changes.len());
        println!("  Draft: {}", changes.iter().filter(|c| c.stage == "draft").count());
        println!("  Full: {}", changes.iter().filter(|c| c.stage == "full").count());
        println!("Pending validation: {}", validation.errors.len());
        println!("Specs: {}", specs.len());
    }
    Ok(())
}
```

#### 阶段 2：统一 `spec` 和 `delta` 命名

```rust
#[derive(Subcommand)]
pub enum SddSpecCommands {
    /// Generate a main spec skeleton
    Skeleton { ... },

    /// Add a requirement (renamed from add-requirement)
    #[command(alias = "add-requirement")]
    AddReq { ... },

    /// Add a scenario
    AddScenario { ... },
}

#[derive(Subcommand)]
pub enum SddDeltaCommands {
    /// Generate a delta spec skeleton
    Skeleton { ... },

    /// Add a new requirement (extracted from add-op)
    AddReq { ... },

    /// Modify an existing requirement (extracted from add-op)
    ModifyReq { ... },

    /// Remove a requirement (extracted from add-op)
    RemoveReq { ... },

    /// Rename a requirement (extracted from add-op)
    RenameReq { ... },

    /// Add a scenario
    AddScenario { ... },
}
```

#### 阶段 3：重组非核心命令

```rust
#[derive(Subcommand)]
pub enum SddCommands {
    // Core workflow
    Init { ... },
    Status { ... },
    List { ... },
    Show { ... },
    Validate { ... },
    Graph { ... },

    // Authoring
    Spec(SddSpecArgs),
    Delta(SddDeltaArgs),

    // Archive
    Archive(SddArchiveArgs),

    // Maintenance
    Orphans { ... },

    // Project management (moved)
    #[command(subcommand)]
    Project(SddProjectCommands),
}

#[derive(Subcommand)]
pub enum SddProjectCommands {
    /// Import specs from OpenSpec format
    Import { ... },

    /// Migrate specs to canonical format
    Migrate { ... },

    /// Output upgrade guide
    UpgradeGuide,

    /// Update skills from templates
    UpdateSkills,
}
```

#### 阶段 4：移除 archive legacy 别名

```rust
#[derive(Args)]
pub struct SddArchiveArgs {
    #[command(subcommand)]
    command: ArchiveSubcommand,  // 不再是 Option，必须指定子命令
}
```

### 方案 B：激进重构（一次性）

一次性重新设计所有子命令，包括：

```
llman sdd
├── init
├── status
├── list
├── show
├── validate
├── graph
├── create                    # 统一创建 change
│   ├── change
│   ├── spec
│   └── delta
├── modify                    # 统一修改
│   ├── spec
│   └── delta
├── archive
│   ├── run
│   ├── freeze
│   └── thaw
└── project
    ├── import
    ├── migrate
    ├── upgrade-guide
    └── update-skills
```

**缺点**：破坏性变更太多，迁移成本高

## 推荐方案

**方案 A（渐进式重构）**，理由：
1. 每个阶段都是独立的、可合并的 PR
2. 通过 `alias` 保持向后兼容
3. 用户有时间迁移
4. 可以在每个阶段后收集反馈

## 实施顺序

1. **Phase 1**: 添加 `status` 命令（无破坏性变更）
2. **Phase 2**: 统一 `spec`/`delta` 命名（添加 alias）
3. **Phase 3**: 重组非核心命令到 `project` 子组
4. **Phase 4**: 移除 archive legacy 别名（需要迁移期）

## 验证方式

每个阶段完成后：
```bash
# 1. 编译通过
cargo +nightly build

# 2. 测试通过
cargo +nightly test

# 3. Help 输出正确
llman sdd --help
llman sdd <subcommand> --help

# 4. 向后兼容性（Phase 2-3）
llman sdd spec add-requirement ...  # 应该通过 alias 工作
```
