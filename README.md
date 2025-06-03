# llman - LLM 规则管理工具

[![Crates.io](https://img.shields.io/crates/v/llman?style=flat-square)](https://crates.io/crates/llman)
[![Downloads](https://img.shields.io/crates/d/llman?style=flat-square)](https://crates.io/crates/llman)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://github.com/StrayDragon/llman/blob/main/LICENSE)
[![CI](https://github.com/StrayDragon/llman/actions/workflows/ci.yaml/badge.svg)](https://github.com/StrayDragon/llman/actions/workflows/ci.yaml)


一个用于管理 LLM 应用（如 Cursor）规则文件的命令行工具。 `llman` 旨在简化和标准化您的开发项目规则配置流程。

## 🌟 功能特性

- 🚀 **交互式生成**: 使用 `inquire` 提供友好的交互式界面，引导您轻松创建规则文件。
- 📁 **统一管理**: 在用户配置目录中集中存储和管理所有 LLM 应用的规则模板。
- 🎯 **智能注入**: 自动检测项目类型并在项目目录中生成特定应用的规则文件。
- 🔧 **多应用支持**: 灵活设计，轻松扩展以支持不同 LLM 应用的规则格式。
- 🛡️ **安全检查**: 内置安全机制，防止在家目录或非项目目录中意外生成或修改文件。
- ⚙️ **环境配置**: 支持通过环境变量 `LLMAN_CONFIG_DIR` 自定义配置目录，满足个性化需求。

### Prompt管理
- 生成和管理prompt规则文件
- 支持多种模板和应用类型
- 交互式界面便于操作

### x Cursor

#### 对话导出 (new)
导出和管理Cursor编辑器的AI对话记录，同时支持 Chat 和 Composer 两种模式的历史：

- 🔍 **智能搜索**: 在对话标题和内容中搜索
- 📝 **多种导出格式**: 控制台输出、单独文件、合并文件
- 🎯 **交互式选择**: 友好的界面选择要导出的对话
- 📁 **自动检测**: 自动找到最新的Cursor工作区数据
- 💾 **Markdown格式**: 导出为可读性良好的Markdown文档

## 📦 安装

### 从 crates.io 安装

```bash
cargo install llman
```

### 从代码安装

```bash
git clone https://github.com/StrayDragon/llman.git
cd llman
cargo install --path .
```

### 从仓库地址安装

```bash
cargo install --git https://github.com/StrayDragon/llman.git
```


## 🛠️ 开发与贡献

0. 确保安装了 [Rust](https://www.rust-lang.org) 和 [just](https://github.com/casey/just) 工具
1. 拉取该仓库
2. 查看 [justfile](./justfile) 中 搜索 "(dev)" 相关的命令进行开发


## 🛠️ 使用方法

### Prompt管理

```bash
# 生成新的prompt规则
llman prompt gen --app cursor --template rust

# 交互式生成
llman prompt gen --interactive

# 列出所有规则
llman prompt list

# 列出特定应用的规则
llman prompt list --app cursor
```

### Cursor对话导出

```bash
# 交互式导出对话
llman x cursor export --interactive

# 或者直接使用默认交互模式
llman x cursor export
```

#### 导出功能演示：

1. **启动导出工具**：自动扫描Cursor工作区，显示找到的对话数量
2. **选择对话**：
   - 显示最近的5个对话供快速选择
   - 提供搜索功能查找历史对话
   - 支持多选，使用空格键选择/取消选择
3. **选择导出方式**：
   - 输出到控制台：直接在终端显示
   - 保存为Markdown文件：每个对话保存为单独文件
   - 保存为单个合并文件：所有对话合并到一个文件

### 项目管理 (使用justfile)

```bash
# 查看所有可用命令
just

# 检查代码
just check

# 运行测试
just test

# 格式化代码
just fmt

# 构建项目
just build

# 测试cursor功能
just test-cursor
```

## 🛠️ 技术架构

### 依赖关系
- **clap**: CLI框架，支持子命令嵌套
- **inquire**: 交互式用户界面
- **diesel**: SQLite数据库ORM
- **serde**: JSON序列化/反序列化
- **chrono**: 时间处理
- **glob**: 文件匹配

### Cursor数据存储
Cursor将对话数据存储在SQLite数据库中：
- 路径：`~/.config/Cursor/User/workspaceStorage/*/state.vscdb`
- 表：`ItemTable`
- 键：`workbench.panel.aichat.view.aichat.chatdata`
- 格式：JSON格式的对话数据

## 🛠️ 开发

```bash
# 克隆项目
git clone <repository-url>
cd llman

# 安装依赖并构建
cargo build

# 运行测试
cargo test

# 开发模式检查
just dev
```

## 📜 许可证

MIT License

