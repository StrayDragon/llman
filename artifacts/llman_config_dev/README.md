# Codex 配置管理指南

欢迎使用 llman 的 Codex 配置管理功能！

## 快速开始

### 1. 创建配置

选择一个适合你使用场景的配置模板：

```bash
# 开发环境 - 宽松设置，适合日常开发
llman x codex account upsert dev --template development

# 生产环境 - 严格设置，适合重要项目
llman x codex account upsert prod --template production
```

### 2. 配置你的 API

编辑刚创建的配置文件，填入你的实际配置：

```bash
# 查看配置文件位置
llman x codex account show dev

# 编辑配置文件（替换占位符）
# 文件位置：~/.config/llman/codex/profiles/dev.toml
```

### 3. 使用配置

```bash
# 切换到开发配置
llman x codex account use dev

# 使用当前配置运行 Codex
llman x codex run -- "帮我分析这个代码"
```

## 配置模板说明

### development（开发环境）
- ✅ 无需批准：`approval_policy = "never"`
- ✅ 网络访问：启用
- ✅ 所有功能：图片查看、网页搜索等
- ✅ 开发变量：`NODE_ENV`、`RUST_LOG`、`PYTHONPATH` 等

### production（生产环境）
- 🔒 需要批准：`approval_policy = "on-request"`
- 🔒 网络禁用：更安全
- 🔒 功能限制：仅启用基本功能
- 🔒 最小变量：只有 `PATH`、`HOME`、`LANG`

## 常用 API 提供商配置

在配置文件中，替换 `[model_providers.your-provider]` 部分：

### OpenAI
```toml
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"
```

### Anthropic Claude
```toml
[model_providers.claude]
name = "Anthropic Claude"
base_url = "https://api.anthropic.com/v1"
env_key = "ANTHROPIC_API_KEY"
wire_api = "chat"
```

### 本地模型 (Ollama)
```toml
[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
env_key = "API_KEY"  # 可选
wire_api = "chat"
```

## 高级用法

### 环境变量管理

```bash
# OpenAI
export OPENAI_API_KEY="your-openai-key"

# Claude
export ANTHROPIC_API_KEY="your-claude-key"
```

### 配置备份

```bash
# 完整备份所有 llman 配置
cp -r ~/.config/llman/ /backup/llman-backup/

# 迁移到新机器
scp -r ~/.config/llman/ new-machine:~/
```

### 常用命令

```bash
# 列出所有配置
llman x codex account list

# 切换配置
llman x codex account use <配置名>

# 查看当前配置详情
llman x codex account show

# 创建新配置
llman x codex account upsert <配置名> --template <模板>
```

## 配置文件位置

- 配置目录：`~/.config/llman/codex/`
- 配置文件：`~/.config/llman/codex/profiles/`
- 当前激活：`~/.config/llman/codex/current_profile`
- 导出使用：`~/.codex/config.toml`

## 故障排除

### 问题：找不到 Codex CLI
```bash
# 安装 Codex CLI
npm install -g @openai/codex

# 验证安装
codex --version
```

### 问题：配置不生效
```bash
# 重新导出当前配置
llman x codex account use <当前配置>
```

### 问题：API 密钥错误
```bash
# 检查环境变量
echo $OPENAI_API_KEY
echo $ANTHROPIC_API_KEY

# 重新设置
export OPENAI_API_KEY="正确的密钥"
```

## 更多帮助

- 官方文档：https://developers.openai.com/codex
- 配置参考：查看配置文件中的详细注释

祝使用愉快！🚀
