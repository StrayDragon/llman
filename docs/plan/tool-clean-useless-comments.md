# Tool 命令：清理无用注释功能设计文档

## 概述

为 llman 添加一个新的 `tool` 命令组，包含 `clean-useless-comments` 子命令，用于智能清理代码中的过度注释。该功能旨在解决 LLM 生成的代码中常见的过度注释问题，保持代码简洁性。

## 问题背景

### 当前问题
1. **过度注释**：LLM 生成的代码常常包含过于详细的注释
2. **注释过时**：代码逻辑变更后，注释未同步更新导致不一致
3. **维护负担**：冗余注释形成"代码屎山"，增加维护成本
4. **可读性下降**：过多的注释反而降低了代码的可读性

### 解决方案
提供一个智能的注释清理工具，能够：
- 识别并移除冗余的注释
- 保留重要的文档和标记性注释（TODO、FIXME 等）
- 支持多种编程语言
- 提供可配置的清理策略
- 确保操作安全性和可回滚性

## 功能规格

### 命令结构
```bash
llman tool clean-useless-comments [OPTIONS]

# 选项
--config <PATH>          # 指定配置文件路径（默认：.llman/config.yaml）
--dry-run               # 预览模式，显示将要进行的更改但不实际执行
--yes                   # 确认写入（默认只 dry-run）
--interactive           # 交互模式，逐个确认更改
--backup <PATH>         # 备份目录路径（默认：.llman/backups，未实现）
--no-backup            # 禁用备份功能（未实现）
--force                # 强制执行，跳过确认提示
--verbose              # 详细输出模式
--git-only            # 仅处理 Git 已跟踪的文件

# 示例
llman tool clean-useless-comments                    # 使用默认配置
llman tool clean-useless-comments --dry-run          # 预览将要进行的更改
llman tool clean-useless-comments --interactive      # 交互式确认
llman tool clean-useless-comments -y                 # 实际写入
llman tool clean-useless-comments --config custom.yaml  # 使用自定义配置
```

### 配置文件格式

配置文件使用 YAML 格式，支持 JSON Schema 验证和 VSCode 自动补全。

```yaml
# .llman/config.yaml
version: "0.1"
tools:
  clean-useless-comments:
    # 文件范围配置
    scope:
      include:
        - "**/*.py"          # Python 文件
        - "**/*.js"          # JavaScript 文件
        - "**/*.ts"          # TypeScript 文件
        - "**/*.rs"          # Rust 文件
        - "**/*.go"          # Go 文件
      exclude:
        - "**/node_modules/**"
        - "**/target/**"
        - "**/.git/**"
        - "**/dist/**"
        - "**/build/**"

    # 语言特定规则
    lang-rules:
      python:
        # 启用的清理类型
        single-line-comments: true      # 清理单行注释
        multi-line-comments: true       # 清理多行注释
        docstrings: false               # 保留文档字符串

        # 保留模式（正则表达式）
        preserve-patterns:
          - "^\\s*#\\s*(TODO|FIXME|NOTE|HACK):\\s*.*"
          - "^\\s*#\\s*(type|param|return|raises):\\s*.*"
          - "^\\s*#\\s*(Copyright|License):\\s*.*"

        # 注释质量评估
        min-comment-length: 10          # 删除短于此长度的注释
        min-code-complexity: 3          # 代码复杂度低于此值时清理注释
        remove-duplicate-comments: true  # 删除重复的注释

      javascript:
        single-line-comments: true
        multi-line-comments: true
        jsdoc: false                    # 保留 JSDoc 注释

        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME|NOTE|HACK):\\s*.*"
          - "^\\s*/\\*\\*.*\\*/"         # 保留 JSDoc 块
          - "^\\s*//\\s*(type|param|return):\\s*.*"

        min-comment-length: 10
        min-code-complexity: 3
        remove-duplicate-comments: true

      rust:
        single-line-comments: true
        multi-line-comments: true
        doc-comments: false            # 保留文档注释 (///, //!)

        preserve-patterns:
          - "^\\s*///\\s*(TODO|FIXME|NOTE|HACK):\\s*.*"
          - "^\\s*//!\\s*(TODO|FIXME|NOTE|HACK):\\s*.*"
          - "^\\s*///\\s*(Examples|Safety|Panics):\\s*.*"

        min-comment-length: 8
        min-code-complexity: 2
        remove-duplicate-comments: true

      go:
        single-line-comments: true
        multi-line-comments: true
        godoc: false                   # 保留 GoDoc 注释

        preserve-patterns:
          - "^\\s*//\\s*(TODO|FIXME|NOTE|HACK):\\s*.*"
          - "^\\s*//\\s*(Package|Function|Return|Parameters):\\s*.*"

        min-comment-length: 10
        min-code-complexity: 3
        remove-duplicate-comments: true

    # 全局规则
    global-rules:
      preserve-empty-lines: true        # 保留被删除注释的空行
      remove-consecutive-empty-lines: true  # 删除连续的空行
      remove-duplicate-comments: true      # 删除重复注释
      max-comment-density: 0.3         # 注释密度超过此值时触发清理
      min-comment-length: 8           # 最小注释长度阈值
      min-code-complexity: 2           # 代码复杂度阈值

    # 安全设置
    safety:
      # 备份能力为规划项，当前未实现
      backup-enabled: true             # 启用备份（未实现）
      backup-dir: ".llman/backups"     # 备份目录（未实现）
      backup-compression: true         # 压缩备份文件（未实现）
      dry-run-first: true              # 首次运行时默认使用干运行
      git-aware: true                  # 仅处理 Git 已跟踪文件
      require-git-commit: true         # 要求 Git 提交后才能运行
      max-backup-age: "30d"            # 备份文件保留时间（未实现）

    # 输出设置
    output:
      show-changed-files: true         # 显示被修改的文件
      show-removed-comments: true      # 显示被删除的注释
      show-statistics: true            # 显示统计信息
      generate-report: true             # 生成处理报告
      report-format: "markdown"        # 报告格式 (markdown|json|yaml)
```

### JSON Schema

配置文件支持 JSON Schema 验证，提供 VSCode 自动补全功能：

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "LLMan Clean Useless Comments Configuration",
  "type": "object",
  "properties": {
    "version": {
      "type": "string",
      "description": "Configuration version"
    },
    "tools": {
      "type": "object",
      "properties": {
        "clean-useless-comments": {
          "type": "object",
          "properties": {
            "scope": {
              "type": "object",
              "properties": {
                "include": {
                  "type": "array",
                  "items": { "type": "string" },
                  "description": "File patterns to include"
                },
                "exclude": {
                  "type": "array",
                  "items": { "type": "string" },
                  "description": "File patterns to exclude"
                }
              }
            },
            "lang-rules": {
              "type": "object",
              "properties": {
                "python": {
                  "$ref": "#/definitions/LanguageRules"
                },
                "javascript": {
                  "$ref": "#/definitions/LanguageRules"
                },
                "rust": {
                  "$ref": "#/definitions/LanguageRules"
                },
                "go": {
                  "$ref": "#/definitions/LanguageRules"
                }
              }
            }
          }
        }
      }
    }
  },
  "definitions": {
    "LanguageRules": {
      "type": "object",
      "properties": {
        "single-line-comments": {
          "type": "boolean",
          "description": "Remove single-line comments"
        },
        "multi-line-comments": {
          "type": "boolean",
          "description": "Remove multi-line comments"
        },
        "preserve-patterns": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Regex patterns to preserve"
        }
      }
    }
  }
}
```

## 技术实现

### 核心依赖

```toml
[dependencies]
# 解析器
tree-sitter = "0.22"
tree-sitter-javascript = "0.21"
tree-sitter-python = "0.21"
tree-sitter-rust = "0.21"
tree-sitter-go = "0.21"

# 配置处理
schemars = { version = "0.8", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"

# 文件处理
notify = "6.1"
walkdir = "2.5"
ignore = "0.4"
glob = "0.3"

# 并行处理
rayon = "1.8"

# 文件比较和备份（备份相关为规划项，未实现）
similar = "2.4"
tempfile = "3.10"
tar = "0.4"
flate2 = "1.0"

# CLI 交互
inquire = "0.7"
indicatif = "0.17"

# 其他
anyhow = "1.0"
thiserror = "2.0"
regex = "1.10"
chrono = { version = "0.4", features = ["serde"] }
```

### 架构设计

```
src/
├── cli.rs                  # 添加 tool 命令
├── tool/
│   ├── mod.rs             # 工具命令主模块
│   ├── clean_comments.rs  # 注释清理主逻辑
│   ├── config.rs          # 配置管理
│   ├── backup.rs          # 备份功能（未实现）
│   └── processors/        # 语言处理器
│       ├── mod.rs
│       ├── python.rs
│       ├── javascript.rs
│       ├── rust.rs
│       └── go.rs
└── schemas/               # JSON Schema
    └── config-schema.json
```

### 注释处理策略

#### 1. 注释分类
- **文档注释**：JSDoc、GoDoc、Rust 文档注释（默认保留）
- **标记注释**：TODO、FIXME、NOTE、HACK（默认保留）
- **类型注释**：类型说明、参数说明（默认保留）
- **实现注释**：解释代码如何工作的注释（可移除）
- **冗余注释**：与代码明显重复的注释（移除）

#### 2. 清理算法
1. **文件过滤**：根据配置的 scope 规则过滤文件
2. **语法解析**：使用 Tree-sitter 解析代码结构
3. **注释提取**：提取所有注释并分类
4. **重要性评估**：根据规则评估每个注释的重要性
5. **安全移除**：仅移除被标记为可安全移除的注释
6. **格式保持**：保持代码格式和结构不变

#### 3. 质量评估指标
- **注释长度**：过短的注释可能是冗余的
- **代码复杂度**：简单代码不需要注释
- **注释密度**：过高的注释密度可能表明过度注释
- **重复度**：与代码或其它注释重复的注释

### 安全机制

#### 1. 备份系统（未实现）
当前决定不实现备份能力，以下为规划设计草案。
```rust
pub struct BackupManager {
    backup_dir: PathBuf,
    compression: bool,
    max_age: Duration,
}

impl BackupManager {
    pub fn create_backup(&self, files: &[PathBuf]) -> Result<BackupInfo>;
    pub fn restore_backup(&self, backup_id: &str) -> Result<()>;
    pub fn cleanup_old_backups(&self) -> Result<()>;
}
```

#### 2. Git 集成
- 检查工作区状态
- 仅处理已跟踪文件
- 提供回滚选项
- 生成变更报告

#### 3. 交互确认
```rust
pub struct InteractiveProcessor {
    allow_all: bool,
    changes: Vec<FileChange>,
}

impl InteractiveProcessor {
    pub fn review_changes(&self) -> Result<Vec<PathBuf>>;
    pub fn confirm_file_change(&self, file: &Path, changes: &CommentChanges) -> Result<bool>;
}
```

## 使用示例

### 基本使用

```bash
# 预览将要进行的更改
llman tool clean-useless-comments --dry-run

# 交互式确认
llman tool clean-useless-comments --interactive

# 使用自定义配置
llman tool clean-useless-comments --config .llman/custom-config.yaml
```

### 配置示例

#### 保守配置（推荐初学者）
```yaml
tools:
  clean-useless-comments:
    lang-rules:
      python:
        single-line-comments: false     # 不清理单行注释
        multi-line-comments: false      # 不清理多行注释
        docstrings: false               # 保留文档字符串
      javascript:
        single-line-comments: false
        multi-line-comments: false
        jsdoc: false
    global-rules:
      max-comment-density: 0.5          # 较高的密度阈值
```

#### 积极配置（适合经验用户）
```yaml
tools:
  clean-useless-comments:
    lang-rules:
      python:
        single-line-comments: true
        multi-line-comments: true
        docstrings: false
        min-comment-length: 15          # 较高的长度阈值
        min-code-complexity: 1          # 低复杂度也清理
    global-rules:
      max-comment-density: 0.2          # 较低的密度阈值
```

## 开发计划

### Phase 1: 基础架构
- [ ] 添加依赖项
- [ ] 实现 CLI 命令结构
- [ ] 创建配置系统
- [ ] 实现基本的文件处理

### Phase 2: 核心功能
- [ ] 实现 Tree-sitter 集成
- [ ] 创建语言处理器
- [ ] 实现注释分析算法
- [ ] 添加安全机制

### Phase 3: 用户体验
- [ ] 实现交互模式
- [ ] 添加备份功能
- [ ] 创建进度指示
- [ ] 生成统计报告

### Phase 4: 测试和优化
- [ ] 创建测试用例
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 文档完善

## 风险评估

### 技术风险
1. **解析准确性**：Tree-sitter 的解析准确性很高，风险较低
2. **性能问题**：大文件处理可能较慢，可通过并行处理缓解
3. **内存使用**：大文件可能占用较多内存，需要流式处理

### 业务风险
1. **误删注释**：通过保守策略和备份机制降低风险
2. **用户体验**：提供详细的预览和回滚功能
3. **兼容性**：支持主流语言和编辑器

### 缓解措施
1. **测试覆盖**：全面的测试用例和示例
2. **用户教育**：详细的文档和最佳实践
3. **渐进发布**：从保守配置开始，逐步增加功能

## 成功指标

### 功能指标
- [ ] 支持所有目标语言（Python、JavaScript、TypeScript、Rust、Go）
- [ ] 准确识别和分类注释
- [ ] 安全移除无用注释
- [ ] 保持代码格式和功能

### 性能指标
- [ ] 处理速度：每秒处理 1000+ 行代码
- [ ] 内存使用：保持在合理范围内
- [ ] 备份速度：快速创建和恢复备份

### 用户体验指标
- [ ] 提供清晰的进度反馈
- [ ] 支持预览和确认
- [ ] 易于配置和使用
- [ ] 完善的文档和示例

---

这个设计文档提供了一个全面的功能规格，确保实现过程有清晰的指导和标准。
