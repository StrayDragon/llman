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
- 🔧 **多应用支持**(TODO): 灵活设计，轻松扩展以支持不同 LLM 应用的规则格式。
- 🛡️ **安全检查**: 内置安全机制，防止在家目录或非项目目录中意外生成或修改文件。
- ⚙️ **环境配置**: 支持通过环境变量 `LLMAN_CONFIG_DIR` 自定义配置目录，满足个性化需求。
- ✨ **模板化**(TODO): 使用模板简化常用规则集的创建和复用。

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

