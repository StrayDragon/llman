# Design: refactor-config-dir-guard

## Context

当前 `cli::run()` 在所有子命令执行前都调用 `determine_config_dir()`，这会触发 dev-project 检测。但只有部分子命令（如 `prompts`、`skills`、`x`、`tool`、`self`）需要全局配置，而 `sdd` 子命令只需要项目级配置。

## Decision

使用 trait 模式来声明子命令对全局配置的依赖：

```rust
trait RequiresGlobalConfig {
    fn requires_global_config(&self) -> bool;
}

impl RequiresGlobalConfig for Commands {
    fn requires_global_config(&self) -> bool {
        match self {
            Commands::Sdd(_) => false,
            _ => true,
        }
    }
}
```

在 `run()` 中按需调用配置目录校验：

```rust
let _config_dir_guard = if command.requires_global_config() {
    let config_dir = determine_config_dir(config_dir.as_ref())?;
    let guard = override_runtime_config_dir(config_dir.clone());
    ensure_global_sample_config(&config_dir)?;
    Some(guard)
} else {
    None
};
```

## Alternatives Considered

1. **子命令级别校验**：每个子命令内部自行校验配置目录
   - 缺点：重复代码多，不易维护

2. **白名单方式**：列出不需要全局配置的子命令
   - 优点：与当前方案类似
   - 缺点：不如 trait 显式

## Consequences

- 正面：开发者在项目目录下运行 `sdd` 子命令不再需要指定 `--config-dir`
- 正面：通过 trait 显式声明配置依赖，代码更清晰
- 风险：`sdd` 子命令未来如果需要全局配置，需要更新 trait 实现
