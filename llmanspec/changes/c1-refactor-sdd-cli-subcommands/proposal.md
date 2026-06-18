# Proposal: Refactor `llman sdd` CLI Subcommands

## Why

当前 `llman sdd` 子命令存在以下问题：
1. **职责重叠**：`archive` 的 legacy 别名导致解析歧义
2. **命名不一致**：`spec` vs `delta` 子命令的动词不同（`add-requirement` vs `add-op`）
3. **缺少交互入口**：直接运行 `llman sdd` 没有交互式菜单
4. **选项过多**：`show` 有 10+ 选项，学习成本高
5. **分组不合理**：`import`、`migrate`、`upgrade-guide` 不属于核心 SDD 工作流
6. **功能缺失**：缺少 `status` 快速查看项目状态
7. **代码冗余**：`Archive` 结构体与 `ArchiveSubcommand::Run` 重复定义参数

## What Changes

### 1. 统一 `spec` 和 `delta` 子命令命名

| 当前 | 建议 | 说明 |
|------|------|------|
| `spec add-requirement` | `spec add-req` | 统一缩写 |
| `delta add-op` | `delta add-req` | 对齐 spec 命名 |
| `delta add-op` (rename) | `delta rename-req` | 分离 rename 操作 |

### 2. 移除 `archive` 的 legacy 别名

```bash
# 当前（有歧义）
llman sdd archive c1           # 是 archive run c1 还是 archive c1？
llman sdd archive run c1       # 推荐方式

# 建议
llman sdd archive run c1       # 唯一方式
llman sdd archive freeze       # 冻结
llman sdd archive thaw         # 解冻
```

### 3. 添加 `status` 子命令

```bash
llman sdd status
# 输出：
# Active changes: 3 (2 draft, 1 full)
# Pending validation: 1
# Pending archive: 0
# Specs: 15
```

### 4. 重组子命令分组

```
llman sdd
├── init              # 初始化
├── status            # [NEW] 项目状态概览
├── list              # 列出 changes/specs
├── show              # 查看详情
├── validate          # 校验
├── graph             # 依赖图
├── spec              # Spec 编辑
│   ├── skeleton
│   ├── add-req       # [RENAMED] from add-requirement
│   └── add-scenario
├── delta             # Delta 编辑
│   ├── skeleton
│   ├── add-req       # [RENAMED] from add-op (add)
│   ├── rename-req    # [NEW] from add-op --op rename
│   ├── remove-req    # [NEW] from add-op --op remove
│   └── add-scenario
├── archive           # 归档
│   ├── run           # 执行归档
│   ├── freeze        # 冻结
│   └── thaw          # 解冻
└── orphans           # 检测孤立 items
```

### 5. 移动非核心命令

```bash
# 移动到 llman sdd project 子组
llman sdd project import       # 从 OpenSpec 导入
llman sdd project migrate      # 格式迁移
llman sdd project upgrade-guide # 升级指南
llman sdd project update-skills # 刷新技能
```

### 6. 简化 `show` 命令选项

```bash
# 当前（10+ 选项）
llman sdd show c1 --json --compact-json --meta-only --deltas-only --requirements --no-scenarios

# 建议：使用组合选项
llman sdd show c1 --output json,deltas    # 多选
llman sdd show c1 --output json,reqs-only # 排除 scenarios
```

## Capabilities

- `cli`: CLI 子命令定义与解析
- `sdd-workflow`: SDD 工作流命令

## Impact

- **Breaking**: `archive <change-id>` 将不再工作，需迁移到 `archive run <change-id>`
- **Breaking**: `spec add-requirement` 改名为 `spec add-req`
- **Breaking**: `delta add-op` 拆分为 `add-req`/`rename-req`/`remove-req`
- **Breaking**: `import`/`migrate`/`upgrade-guide` 移动到 `sdd project` 下
- **New**: 添加 `status` 子命令
