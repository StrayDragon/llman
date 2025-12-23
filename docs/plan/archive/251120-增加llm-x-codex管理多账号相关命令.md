与当前的 `llman x claude-code` 子命令类似, 请模仿相关的命令结构和交互

结合

MiniMax集成参考例子

安装 Codex CLI

    使用 npm 全局安装 Codex CLI

Report incorrect code
Copy

npm i -g @openai/codex

​
在 Codex CLI 中配置 MiniMax API
重要提示：使用前请先清除 OpenAI 环境变量在配置前，请确保清除以下 OpenAI 相关的环境变量，以免影响 MiniMax API 的正常使用：

    OPENAI_API_KEY
    OPENAI_BASE_URL

    编辑 Codex 的配置文件，路径为 .codex/config.toml，将以下配置添加到配置文件中。

    base_url 需根据地理位置设置：国内用户使用 https://api.minimaxi.com/v1，国际用户使用 https://api.minimax.io/v1

Report incorrect code
Copy

[model_providers.minimax]
name = "MiniMax Chat Completions API"
base_url = "https://api.minimaxi.com/v1"
env_key = "MINIMAX_API_KEY"
wire_api = "chat"
requires_openai_auth = false
request_max_retries = 4
stream_max_retries = 10
stream_idle_timeout_ms = 300000

[profiles.m2]
model = "codex-MiniMax-M2"
model_provider = "minimax"

    出于安全考虑，请在当前终端会话中通过环境变量设置 API Key，其中，需要将 MINIMAX_API_KEY 替换为从 MiniMax 开发者平台 (国际用户可访问 MiniMax Developer Platform) 获取的 API Key

Report incorrect code
Copy

export MINIMAX_API_KEY="<MINIMAX_API_KEY>"

    使用指定的配置文件启动 Codex CLI。

Report incorrect code
Copy

codex --profile m2


构建合理的管理和理解多codex profile方式
