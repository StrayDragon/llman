# 实现计划：重新设计 llman x codex 功能以支持 OpenAI Codex 配置系统

## 概述

重新设计 `llman x codex` 功能，采用软链接方式管理 OpenAI Codex 配置，让 llman 作为配置管理的便捷入口，但配置文件本体由 OpenAI Codex 管理。无需考虑向后兼容性，专注于提供良好的交互式配置管理体验。

## 当前实现问题

### 1. 架构设计不当
- 当前试图管理环境变量，而不是 OpenAI Codex 的原生配置系统
- 配置文件位置和管理方式与 OpenAI Codex 不一致

### 2. 功能缺失
- 无法管理 OpenAI Codex 的 model_providers、profiles、features 等核心配置
- 缺乏对 OpenAI Codex 原生命令的集成

### 3. 用户体验不佳
- 没有便捷的交互式配置管理
- 无法快速切换和预览配置

## OpenAI Codex 配置系统分析

### 配置文件位置
- **标准位置**: `~/.codex/config.toml`
- **CLI 集成**: 通过 `--profile <name>` 选择配置文件

### 核心配置结构
```toml
# 顶级配置选项
model = "gpt-5"
model_provider = "openai"
approval_policy = "on-request"
sandbox_mode = "workspace-write"

# 模型提供商定义
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

# 配置文件定义
[profiles.deep-review]
model = "gpt-5-pro"
model_reasoning_effort = "high"
approval_policy = "never"

# 功能标志
[features]
streamable_shell = true
web_search_request = true
```

## 重新设计方案：软链接管理

### 1. 核心设计理念
- **不管理配置内容**：让 OpenAI Codex 自己管理配置文件
- **提供便捷入口**：llman 作为配置管理的便捷交互界面
- **软链接管理**：通过管理软链接来实现配置切换
- **简化架构**：专注于用户体验，而不是重复实现配置管理

### 2. 架构设计

```
~/.codex/
├── config.toml                     # OpenAI Codex 主配置文件（由 Codex 管理）
├── configs/                        # llman 管理的配置文件目录
│   ├── default.toml                # 默认配置
│   ├── development.toml            # 开发环境配置
│   ├── production.toml             # 生产环境配置
│   └── custom.toml                 # 自定义配置
└── active                          # 指向当前激活配置的软链接
    └── config.toml -> ../configs/development.toml
```

**工作原理**：
1. OpenAI Codex 读取 `~/.codex/config.toml`
2. llman 通过管理软链接 `~/.codex/config.toml` 来切换配置
3. 实际配置文件存储在 `~/.codex/configs/` 目录下
4. 用户通过 llman 交互式管理这些配置文件

### 3. 简化的命令结构

```
llman x codex
├── init                          # 初始化配置管理环境
├── list                          # 列出所有可用配置
├── create <name>                 # 创建新配置（交互式）
├── edit <name>                   # 编辑配置
├── delete <name>                 # 删除配置
├── use <name>                    # 切换到指定配置
├── show                          # 显示当前配置信息
└── run <codex_args>...           # 使用当前配置运行 codex
```

### 4. 核心实现：软链接管理器

**简化的数据结构 (`src/x/codex/config.rs`)**:
```rust
use std::path::PathBuf;

pub struct CodexManager {
    codex_dir: PathBuf,
    configs_dir: PathBuf,
    active_config: PathBuf,
}

impl CodexManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot find home directory"))?;
        let codex_dir = home.join(".codex");
        let configs_dir = codex_dir.join("configs");
        let active_config = codex_dir.join("config.toml");

        Ok(Self {
            codex_dir,
            configs_dir,
            active_config,
        })
    }

    pub fn init(&self) -> Result<()> {
        // 创建目录结构
        fs::create_dir_all(&self.configs_dir)?;

        // 如果不存在主配置文件，创建默认配置
        if !self.active_config.exists() {
            let default_config = self.configs_dir.join("default.toml");
            if !default_config.exists() {
                self.create_default_config(&default_config)?;
            }
            self.create_symlink(&default_config)?;
        }

        Ok(())
    }

    pub fn list_configs(&self) -> Result<Vec<String>> {
        let mut configs = Vec::new();
        for entry in fs::read_dir(&self.configs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    configs.push(name.to_string());
                }
            }
        }
        configs.sort();
        Ok(configs)
    }

    pub fn create_config(&self, name: &str, template: Option<&str>) -> Result<PathBuf> {
        let config_path = self.configs_dir.join(format!("{}.toml", name));
        if config_path.exists() {
            anyhow::bail!("Configuration '{}' already exists", name);
        }

        let content = if let Some(template) = template {
            self.get_template(template)?
        } else {
            self.get_default_template()?
        };

        fs::write(&config_path, content)?;
        Ok(config_path)
    }

    pub fn use_config(&self, name: &str) -> Result<()> {
        let config_path = self.configs_dir.join(format!("{}.toml", name));
        if !config_path.exists() {
            anyhow::bail!("Configuration '{}' not found", name);
        }

        self.create_symlink(&config_path)?;
        println!("✅ Switched to configuration: {}", name);
        Ok(())
    }

    pub fn get_current_config(&self) -> Result<Option<String>> {
        if !self.active_config.exists() {
            return Ok(None);
        }

        let target = fs::read_link(&self.active_config)?;
        if let Some(name) = target.file_stem().and_then(|s| s.to_str()) {
            Ok(Some(name.to_string()))
        } else {
            Ok(None)
        }
    }

    fn create_symlink(&self, target: &PathBuf) -> Result<()> {
        // 删除现有链接
        if self.active_config.exists() {
            fs::remove_file(&self.active_config)?;
        }

        // 创建新的软链接
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &self.active_config)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, &self.active_config)?;
        }

        Ok(())
    }

    fn create_default_config(&self, path: &PathBuf) -> Result<()> {
        let template = self.get_default_template()?;
        fs::write(path, template)?;
        Ok(())
    }

    fn get_default_template(&self) -> Result<String> {
        Ok(r#"# Default OpenAI Codex Configuration
# For full documentation, see: https://developers.openai.com/codex

# Model settings
model = "gpt-4o"
model_provider = "openai"

# Approval policy: untrusted, on-failure, on-request, never
approval_policy = "on-request"

# Sandbox mode: read-only, workspace-write, danger-full-access
sandbox_mode = "workspace-write"

# OpenAI Provider
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

# llman specific configuration
[llman]
# Automatically created by llman: true
# Template used: default
# Created at: 2025-01-16T00:00:00Z
auto_created = true
template = "default"
version = "1.0"

# llman managed profiles (for future use)
[llman.profiles]
# This section is reserved for llman-specific metadata

# Optional: Add custom profiles
[profiles.development]
model = "gpt-4o"
approval_policy = "on-request"

[profiles.production]
model = "gpt-4o"
approval_policy = "never"

# Optional: Enable features
[features]
# streamable_shell = true
# web_search_request = true
"#.to_string())
    }

    fn get_template(&self, template_name: &str) -> Result<String> {
        match template_name {
            "openai" => Ok(r#"# OpenAI Configuration
model = "gpt-4o"
model_provider = "openai"
approval_policy = "on-request"

[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"
"#.to_string()),
            "ollama" => Ok(r#"# Ollama Configuration
model = "llama3"
model_provider = "ollama"
approval_policy = "never"

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
wire_api = "chat"
"#.to_string()),
            "minimal" => Ok(r#"# Minimal Configuration
model = "gpt-4o"
model_provider = "openai"

[model_providers.openai]
env_key = "OPENAI_API_KEY"
"#.to_string()),
            _ => anyhow::bail!("Unknown template: {}", template_name),
        }
    }
}
```

### 5. 智能配置检测和初始化

**自动检测和处理现有配置 (`src/x/codex/config.rs`)**:
```rust
impl CodexManager {
    pub fn init_or_detect(&self) -> Result<ConfigStatus> {
        // 创建目录结构
        fs::create_dir_all(&self.configs_dir)?;

        // 检测现有配置状态
        if !self.active_config.exists() {
            // 没有主配置文件
            return if self.has_existing_codex_config() {
                // 发现现有 Codex 配置，导入它
                self.import_existing_config()
            } else {
                // 创建默认配置
                self.create_default_setup()
            };
        }

        // 检查是否为软链接
        match fs::read_link(&self.active_config) {
            Ok(target) => {
                // 已是软链接，正常状态
                Ok(ConfigStatus::SymlinkActive)
            }
            Err(_) => {
                // 是普通文件，需要转换为软链接系统
                self.migrate_to_symlink()
            }
        }
    }

    fn has_existing_codex_config(&self) -> bool {
        self.active_config.exists()
    }

    fn import_existing_config(&self) -> Result<ConfigStatus> {
        println!("🔄 检测到现有 OpenAI Codex 配置，正在导入...");

        // 将现有配置作为默认配置保存
        let default_config = self.configs_dir.join("default.toml");
        fs::copy(&self.active_config, &default_config)?;

        // 创建软链接
        self.create_symlink(&default_config)?;

        println!("✅ 现有配置已导入为 'default'");
        println!("💡 原配置已备份为: {}", default_config.display());

        Ok(ConfigStatus::Imported)
    }

    fn create_default_setup(&self) -> Result<ConfigStatus> {
        println!("🚀 首次使用，创建默认配置...");

        let default_config = self.configs_dir.join("default.toml");
        self.create_default_config(&default_config)?;
        self.create_symlink(&default_config)?;

        println!("✅ 默认配置已创建");
        Ok(ConfigStatus::Created)
    }

    fn migrate_to_symlink(&self) -> Result<ConfigStatus> {
        println!("🔄 检测到传统配置文件，正在迁移到软链接系统...");

        // 备份现有配置
        let backup_path = self.active_config.with_extension("toml.llman.backup");
        fs::copy(&self.active_config, &backup_path)?;

        // 将其作为默认配置
        let default_config = self.configs_dir.join("default.toml");
        fs::copy(&self.active_config, &default_config)?;

        // 创建软链接
        self.create_symlink(&default_config)?;

        println!("✅ 配置已迁移到软链接系统");
        println!("💾 原配置已备份到: {}", backup_path.display());

        Ok(ConfigStatus::Migrated)
    }
}

#[derive(Debug)]
pub enum ConfigStatus {
    SymlinkActive,    // 软链接已激活
    Imported,         // 导入了现有配置
    Created,          // 创建了新配置
    Migrated,         // 迁移了现有配置
}
```

### 6. 命令实现 (`src/x/codex/command.rs`)

**简化的命令结构**:
```rust
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CodexArgs {
    #[command(subcommand)]
    pub command: Option<CodexCommands>,
}

#[derive(Subcommand)]
pub enum CodexCommands {
    /// Initialize configuration management
    Init,
    /// List all available configurations
    List,
    /// Create a new configuration interactively
    Create {
        /// Configuration name
        name: String,
        /// Use a template (openai, ollama, minimal)
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Edit a configuration
    Edit {
        /// Configuration name (defaults to current)
        name: Option<String>,
    },
    /// Delete a configuration
    Delete {
        /// Configuration name
        name: String,
    },
    /// Switch to a configuration
    Use {
        /// Configuration name
        name: String,
    },
    /// Show current configuration
    Show,
    /// Run codex with current configuration
    Run {
        /// Arguments to pass to codex
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

pub fn run(args: &CodexArgs) -> Result<()> {
    let manager = CodexManager::new()?;

    match &args.command {
        Some(CodexCommands::Init) => {
            let status = manager.init_or_detect()?;
            match status {
                ConfigStatus::SymlinkActive => println!("✅ 配置管理系统已就绪"),
                ConfigStatus::Imported => println!("✅ 配置已导入"),
                ConfigStatus::Created => println!("✅ 默认配置已创建"),
                ConfigStatus::Migrated => println!("✅ 配置已迁移"),
            }
        }
        Some(CodexCommands::List) => {
            list_configurations(&manager)?;
        }
        Some(CodexCommands::Create { name, template }) => {
            create_configuration(&manager, name, template.as_deref())?;
        }
        Some(CodexCommands::Edit { name }) => {
            edit_configuration(&manager, name.as_deref())?;
        }
        Some(CodexCommands::Delete { name }) => {
            delete_configuration(&manager, name)?;
        }
        Some(CodexCommands::Use { name }) => {
            manager.use_config(name)?;
        }
        Some(CodexCommands::Show) => {
            show_current_config(&manager)?;
        }
        Some(CodexCommands::Run { args }) => {
            run_codex(&manager, args.clone())?;
        }
        None => {
            // 默认行为：显示状态或进入交互模式
            show_status_or_interactive(&manager)?;
        }
    }

    Ok(())
}

fn show_status_or_interactive(manager: &CodexManager) -> Result<()> {
    let status = manager.init_or_detect()?;

    match status {
        ConfigStatus::SymlinkActive => {
            // 显示当前状态
            if let Some(current) = manager.get_current_config()? {
                println!("📋 当前配置: {}", current);
                println!("💡 使用 'llman x codex list' 查看所有配置");
                println!("💡 使用 'llman x codex create <name>' 创建新配置");
            } else {
                println!("❌ 未找到激活的配置");
            }
        }
        _ => {
            println!("✅ 配置管理系统已初始化");
            println!("💡 使用 'llman x codex' 查看状态");
        }
    }

    Ok(())
}
```

### 7. llman 配置命名空间

**配置模板中的 llman 特定节**:
```rust
impl CodexManager {
    fn get_default_template(&self) -> Result<String> {
        Ok(r#"# Default OpenAI Codex Configuration
# For full documentation, see: https://developers.openai.com/codex

# Model settings
model = "gpt-4o"
model_provider = "openai"

# Approval policy: untrusted, on-failure, on-request, never
approval_policy = "on-request"

# Sandbox mode: read-only, workspace-write, danger-full-access
sandbox_mode = "workspace-write"

# OpenAI Provider
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

# llman specific configuration
[llman]
# Automatically created by llman: true
# Template used: default
# Created at: 2025-01-16T00:00:00Z
auto_created = true
template = "default"
version = "1.0"

# llman managed profiles (for future use)
[llman.profiles]
# This section is reserved for llman-specific metadata

# Optional: Add custom profiles
[profiles.development]
model = "gpt-4o"
approval_policy = "on-request"

[profiles.production]
model = "gpt-4o"
approval_policy = "never"

# Optional: Enable features
[features]
# streamable_shell = true
# web_search_request = true
"#.to_string())
    }

    fn enhance_config_with_llman_metadata(&self, config_path: &PathBuf, template: &str) -> Result<()> {
        let mut content = fs::read_to_string(config_path)?;

        // 添加或更新 llman 节
        let llman_section = format!(r#"
# llman specific configuration
[llman]
# Managed by llman configuration manager
auto_created = true
template = "{}"
created_at = "{}"
version = "1.0"

[llman.profiles]
# This section is reserved for llman-specific metadata"#,
            template,
            chrono::Utc::now().to_rfc3339()
        );

        if !content.contains("[llman]") {
            content.push_str(&llman_section);
        } else {
            // 更新现有 llman 节
            content = regex::Regex::new(r"\[llman\].*?(?=\n\[|\n#|$)")
                .unwrap()
                .replace(&content, &llman_section.trim())
                .to_string();
        }

        fs::write(config_path, content)?;
        Ok(())
    }
}
```

## 实施阶段

### 第一阶段：核心软链接管理器
- 在 `src/x/codex/config.rs` 中实现 `CodexManager`
- 添加智能配置检测和自动初始化功能
- 实现软链接创建和管理逻辑

### 第二阶段：命令接口
- 在 `src/x/codex/command.rs` 中实现简化命令结构
- 添加配置列表、创建、编辑、删除、切换功能
- 实现默认状态显示

### 第三阶段：交互式配置创建
- 在 `src/x/codex/interactive.rs` 中实现友好的配置向导
- 添加模板选择和自定义配置功能
- 实现配置文件自动增强（添加 llman 元数据）

### 第四阶段：集成和测试
- 更新主 CLI 集成
- 添加国际化支持
- 测试各种配置场景

## 关键文件列表

### 需要完全重写的文件
1. **`src/x/codex/config.rs`** - 核心软链接管理器
2. **`src/x/codex/command.rs`** - 命令接口实现
3. **`src/x/codex/interactive.rs`** - 交互式配置创建

### 需要部分更新的文件
4. **`src/cli.rs`** - 主 CLI 集成
5. **`locales/app.yml`** - 国际化消息

### 新增依赖项
6. **`Cargo.toml`** - 添加 `chrono`（时间戳）、`regex`（文本处理）

## 使用示例

### 初始化和使用
```bash
# 初始化（自动检测现有配置）
llman x codex init

# 列出所有配置
llman x codex list

# 创建新配置（交互式）
llman x codex create development

# 使用模板创建配置
llman x codex create ollama --template ollama

# 切换配置
llman x codex use development

# 查看当前配置
llman x codex show

# 编辑当前配置
llman x codex edit

# 使用当前配置运行 codex
llman x codex run -- --help
```

### 自动配置检测流程
1. **首次运行**：自动检测是否存在 `~/.codex/config.toml`
2. **发现现有配置**：导入为 `default` 配置并创建软链接
3. **发现普通文件**：备份并转换为软链接系统
4. **无配置**：创建默认配置

### 配置文件结构
```
~/.codex/
├── config.toml -> configs/development.toml  # 软链接到当前配置
├── configs/
│   ├── default.toml                          # 默认配置
│   ├── development.toml                      # 开发配置
│   └── production.toml                       # 生产配置
└── backup/                                  # 自动备份目录
    └── config.toml.llman.backup             # 原配置备份
```

## 配置文件示例

### 带有 llman 元数据的配置文件
```toml
# OpenAI Codex Configuration
model = "gpt-4o"
model_provider = "openai"
approval_policy = "on-request"

[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

# llman specific configuration
[llman]
auto_created = true
template = "openai"
created_at = "2025-01-16T10:30:00Z"
version = "1.0"

[llman.profiles]
# This section is reserved for llman-specific metadata
```

## 预期收益

### 1. 简化的用户体验
- 一键初始化和自动配置检测
- 友好的交互式配置创建
- 直观的配置切换

### 2. 与 OpenAI Codex 完美集成
- 不干扰 OpenAI Codex 的原生配置管理
- 支持所有原生配置选项
- 标准的配置文件位置和格式

### 3. 智能配置管理
- 自动检测和迁移现有配置
- 安全的配置切换（带备份）
- 可靠的软链接管理

### 4. 可扩展性
- 支持配置模板系统
- 预留 llman 命名空间
- 易于添加新功能

这个软链接管理方案提供了一个简洁、可靠的配置管理解决方案，专注于提供良好的用户体验，同时完全兼容 OpenAI Codex 的原生配置系统。
