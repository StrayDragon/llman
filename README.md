# llman - LLM 规则管理工具

[![Crates.io](https://img.shields.io/crates/v/llman?style=flat-square)](https://crates.io/crates/llman)
[![Downloads](https://img.shields.io/crates/d/llman?style=flat-square)](https://crates.io/crates/llman)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://github.com/StrayDragon/llman/blob/main/LICENSE)
[![CI](https://github.com/StrayDragon/llman/actions/workflows/ci.yaml/badge.svg)](https://github.com/StrayDragon/llman/actions/workflows/ci.yaml)


一个用于管理 LLM 应用（如 Cursor）规则文件的命令行工具。 `llman` 旨在简化和标准化您的开发项目规则配置流程。

## 🌟 功能特性

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

## 🛠️ 使用示例

### Prompt管理

```bash
# 更新(增加)prompt规则
llman prompt upsert --app cursor --name rust --content "This is example rules of rust"

# 生成新的prompt规则
llman prompt gen --app cursor --template rust

# 交互式生成
llman prompt gen -i # --interactive

# 列出所有规则
llman prompt list

# 列出特定应用的规则
llman prompt list --app cursor
```

### Cursor对话导出

```bash
# 交互式导出对话
llman x cursor export -i # --interactive
```


## 🛠️ 开发与贡献

0. 确保安装了 [Rust](https://www.rust-lang.org) 和 [just](https://github.com/casey/just) 工具
1. 拉取该仓库
2. 查看 [justfile](./justfile) 中 搜索 "(dev)" 相关的命令进行开发
