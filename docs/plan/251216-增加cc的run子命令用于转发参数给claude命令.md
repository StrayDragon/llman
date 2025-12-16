# 实现计划：为 llman x cc 增加 run 子命令用于转发参数给 claude 命令

## 概述

为 `llman x cc` (claude-code) 命令添加一个新的 `run` 子命令，该子命令将结合当前主命令的交互式配置选择功能和 `account use` 命令的参数传递功能。

## 当前实现分析

### 现有命令结构
- `llman x cc` - 交互式选择配置并运行 claude（不支持参数传递）
- `llman x cc account use <name> [args...]` - 使用指定配置并传递参数给 claude

### 需要实现的功能
- `llman x cc run -i` - 交互模式：询问配置选择和参数传递
- `llman x cc run --group <name> -- [args...]` - 非交互模式：直接使用指定配置并传递参数

## 实现方案

### 1. 修改命令定义 (src/x/claude_code/command.rs)

在 `ClaudeCodeCommands` 枚举中添加 `Run` 变体：

```rust
#[derive(Subcommand)]
pub enum ClaudeCodeCommands {
    /// Account management commands for handling multiple API configurations
    #[command(alias = "a")]
    Account {
        #[command(subcommand)]
        action: Option<AccountAction>,
    },
    /// Run claude with configuration selection
    #[command(about = "Run claude with configuration")]
    Run {
        /// Interactive mode: prompt for configuration and arguments
        #[arg(short = 'i', long, help = "Interactive mode: prompt for configuration and arguments")]
        interactive: bool,

        /// Configuration group name to use (required in non-interactive mode)
        #[arg(long = "group", help = "Configuration group name to use")]
        group: Option<String>,

        /// Arguments to pass to claude command (use -- to separate)
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments to pass to claude (use -- to separate from run options)"
        )]
        args: Vec<String>,
    },
}
```

### 2. 更新主运行函数

修改 `run` 函数以处理新的 `Run` 命令：

```rust
pub fn run(args: &ClaudeCodeArgs) -> Result<()> {
    match &args.command {
        Some(ClaudeCodeCommands::Account { action }) => {
            handle_account_command(action.as_ref())?;
        }
        Some(ClaudeCodeCommands::Run { interactive, group, args }) => {
            handle_run_command(*interactive, group.as_deref(), args.clone())?;
        }
        None => {
            handle_main_command()?;
        }
    }
    Ok(())
}
```

### 3. 实现 run 命令处理函数

创建 `handle_run_command` 函数来处理 run 命令逻辑，支持交互和非交互两种模式：

```rust
fn handle_run_command(interactive: bool, group_name: Option<&str>, args: Vec<String>) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    if config.is_empty() {
        println!("{}", t!("claude_code.main.no_configs_found"));
        println!();
        println!("{}", t!("claude_code.main.suggestion_import"));
        println!("  {}", t!("claude_code.main.command_import"));
        println!();
        println!("{}:", t!("claude_code.main.alternative_config"));
        println!(
            "  {}",
            Config::config_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        );
        return Ok(());
    }

    // 验证参数组合
    if !interactive && group_name.is_none() {
        eprintln!("{}", t!("claude_code.run.error.group_required_non_interactive"));
        eprintln!("{}", t!("claude_code.run.error.use_i_or_group"));
        return Ok(());
    }

    let (selected_group, claude_args) = if interactive {
        // 交互模式：询问配置和参数
        handle_interactive_mode(&config)?
    } else {
        // 非交互模式：使用指定的配置
        let group = group_name.unwrap().to_string();
        (group, args)
    };

    // 执行 claude 命令
    if let Some(env_vars) = config.get_group(&selected_group) {
        println!("{}", t!("claude_code.run.using_config", name = selected_group));

        let mut cmd = Command::new("claude");

        // 注入环境变量
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        // 添加传递的参数
        for arg in claude_args {
            cmd.arg(arg);
        }

        let status = cmd.status().context("Failed to execute claude command")?;

        if !status.success() {
            eprintln!("{}", t!("claude_code.error.failed_claude_command"));
        }
    } else {
        println!("{}", t!("claude_code.account.group_not_found", name = selected_group));
        println!("{}", t!("claude_code.account.use_list_command"));
    }

    Ok(())
}

/// 处理交互模式：选择配置和输入参数
fn handle_interactive_mode(config: &Config) -> Result<(String, Vec<String>)> {
    // 选择配置组
    let selected_group = interactive::select_config_group(config)?
        .ok_or_else(|| anyhow::anyhow!("No configuration selected"))?;

    // 询问是否需要传递参数给 claude
    let use_args = inquire::Confirm::new(&t!("claude_code.run.interactive.prompt_args"))
        .with_default(false)
        .prompt()
        .context("Failed to prompt for arguments")?;

    let claude_args = if use_args {
        let args_text = inquire::Text::new(&t!("claude_code.run.interactive.enter_args"))
            .with_help_text(&t!("claude_code.run.interactive.args_help"))
            .prompt()
            .context("Failed to get claude arguments")?;

        // 简单的参数分割（可以用更复杂的方式处理引号等）
        args_text.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };

    Ok((selected_group, claude_args))
}
```

### 4. 国际化支持 (locales/app.yml)

添加新的国际化消息：

```yaml
claude_code:
  run:
    interactive:
      prompt_args:
        en: "Do you want to pass arguments to claude?"
        zh-CN: "是否要传递参数给 claude？"
      enter_args:
        en: "Enter arguments for claude command:"
        zh-CN: "输入 claude 命令的参数："
      args_help:
        en: "e.g., --version --help project create"
        zh-CN: "例如：--version --help project create"
    using_config:
      en: "Using configuration: %{name}"
      zh-CN: "使用配置: %{name}"
    error:
      group_required_non_interactive:
        en: "Error: In non-interactive mode, --group is required"
        zh-CN: "错误：非交互模式下必须指定 --group 参数"
      use_i_or_group:
        en: "Use -i for interactive mode or specify --group <name>"
        zh-CN: "请使用 -i 进入交互模式，或指定 --group <名称>"
```

## 使用示例

实现后的使用方式：

```bash
# 交互模式：询问配置选择和参数
llman x cc run -i

# 非交互模式：指定配置并传递参数
llman x cc run --group production -- --version
llman x cc run --group production -- --help
llman x cc run -g staging -- project create

# 非交互模式：指定配置但不传递参数
llman x cc run --group development

# 错误用法（会提示错误）
llman x cc run  # 缺少 -i 或 --group
llman x cc run -- --version  # 缺少 -i 或 --group
```

## 命令行为对比

| 命令 | 模式 | 配置选择 | 参数传递 |
|------|------|----------|----------|
| `llman x cc` | 交互 | 交互选择 | 不支持 |
| `llman x cc account use <name> [args...]` | 非交互 | 指定 | 支持 |
| `llman x cc run -i` | 交互 | 交互选择 | 交互询问 |
| `llman x cc run --group <name> -- [args...]` | 非交互 | 指定 | 支持 |

## 实现优势

1. **代码复用**：充分利用现有的配置加载、交互式选择和命令执行逻辑
2. **一致性**：遵循现有代码模式和错误处理方式
3. **向后兼容**：不影响现有命令的行为
4. **灵活性**：提供多种配置选择和参数传递方式

## 需要修改的文件

1. `/home/l8ng/Projects/__straydragon__/llman/src/x/claude_code/command.rs` - 主要实现文件
2. `/home/l8ng/Projects/__straydragon__/llman/locales/app.yml` - 国际化消息

## 实现步骤

1. 修改 `ClaudeCodeCommands` 枚举添加 `Run` 变体
2. 更新 `run` 函数处理新的命令分支
3. 实现 `handle_run_command` 函数
4. 添加国际化消息
5. 测试所有使用场景