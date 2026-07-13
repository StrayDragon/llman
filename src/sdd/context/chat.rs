//! async-openai adapter for the pageindex agentic retrieval.
//!
//! [`ChatConfig`] resolves the chat-model configuration separately from the
//! embedding model (per the sdd-context spec: `LLMAN_SDD_INDEX_CHAT_*` env vars
//! fall back to `LLMAN_SDD_INDEX_OPENAI_*`). [`OpenAiInvoker`] implements
//! [`crate::sdd::context::retrieve::ChatInvoker`] by mapping our lightweight
//! protocol types onto `async-openai`'s chat / function-calling request types.

use crate::sdd::context::retrieve::{ChatInvoker, ChatTurn, Msg, ToolCall, ToolSchema};
use anyhow::{Context as _, Result};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent,
    ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageContent,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent, ChatCompletionTool,
    ChatCompletionToolType, CreateChatCompletionRequestArgs, FunctionCall, FunctionObject,
};

/// Configuration for the chat model used by pageindex retrieval.
#[derive(Debug, Clone)]
pub struct ChatConfig {
    pub api_host: String,
    pub api_key: String,
    pub model: String,
}

impl ChatConfig {
    /// Resolve chat config from the environment.
    ///
    /// Priority: `LLMAN_SDD_INDEX_CHAT_*` → fall back to `LLMAN_SDD_INDEX_OPENAI_*`
    /// (host/key) → hardcoded host default. The chat model has no default — it
    /// must support tool/function calling and be set via `LLMAN_SDD_INDEX_CHAT_MODEL`.
    pub fn from_env() -> Result<Self> {
        let api_host = env_or("LLMAN_SDD_INDEX_CHAT_API_HOST")
            .or_else(|| env_or("LLMAN_SDD_INDEX_OPENAI_API_HOST"))
            .unwrap_or_default();
        let api_key = env_or("LLMAN_SDD_INDEX_CHAT_API_KEY")
            .or_else(|| env_or("LLMAN_SDD_INDEX_OPENAI_API_KEY"))
            .unwrap_or_default();
        let model = env_or("LLMAN_SDD_INDEX_CHAT_MODEL").ok_or_else(|| {
            anyhow::anyhow!(
                "LLMAN_SDD_INDEX_CHAT_MODEL is required for the pageindex backend \
                 (agentic retrieval needs a chat model that supports tool/function calling)"
            )
        })?;
        if api_host.is_empty() {
            anyhow::bail!(
                "LLMAN_SDD_INDEX_CHAT_API_HOST (or LLMAN_SDD_INDEX_OPENAI_API_HOST) is required "
            );
        }
        Ok(Self {
            api_host,
            api_key,
            model,
        })
    }
}

fn env_or(var: &str) -> Option<String> {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

/// async-openai-backed [`ChatInvoker`].
pub struct OpenAiInvoker {
    client: Client<OpenAIConfig>,
    model: String,
    api_host: String,
}

impl OpenAiInvoker {
    pub fn new(cfg: &ChatConfig) -> Self {
        let config = OpenAIConfig::new()
            .with_api_base(cfg.api_host.trim_end_matches('/'))
            .with_api_key(cfg.api_key.clone());
        Self {
            client: Client::with_config(config),
            model: cfg.model.clone(),
            api_host: cfg.api_host.clone(),
        }
    }
}

impl ChatInvoker for OpenAiInvoker {
    async fn chat_turn(&self, messages: &[Msg], tools: &[ToolSchema]) -> Result<ChatTurn> {
        let req_messages: Vec<ChatCompletionRequestMessage> = messages
            .iter()
            .map(convert_message)
            .collect::<Result<_>>()?;

        let tools_aoi: Vec<ChatCompletionTool> = tools
            .iter()
            .map(|t| ChatCompletionTool {
                r#type: ChatCompletionToolType::Function,
                function: FunctionObject {
                    name: t.name.to_string(),
                    description: Some(t.description.to_string()),
                    parameters: Some(t.parameters.clone()),
                    strict: None,
                },
            })
            .collect();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(req_messages)
            .tools(tools_aoi)
            .build()
            .context("failed to build chat completion request")?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .with_context(|| format!("chat completion via {} failed", self.api_host))?;

        let msg = response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| anyhow::anyhow!("chat response had no choices"))?;

        let tool_calls = msg
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| ToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: tc.function.arguments,
            })
            .collect();

        Ok(ChatTurn {
            content: msg.content,
            tool_calls,
        })
    }
}

/// Convert our protocol [`Msg`] into an async-openai request message.
fn convert_message(msg: &Msg) -> Result<ChatCompletionRequestMessage> {
    Ok(match msg {
        Msg::System(s) => {
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(s.clone()),
                name: None,
            })
        }
        Msg::User(s) => ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(s.clone()),
            name: None,
        }),
        Msg::Assistant {
            content,
            tool_calls,
        } => ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
            content: content
                .clone()
                .map(ChatCompletionRequestAssistantMessageContent::Text),
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(
                    tool_calls
                        .iter()
                        .map(|tc| ChatCompletionMessageToolCall {
                            id: tc.id.clone(),
                            r#type: ChatCompletionToolType::Function,
                            function: FunctionCall {
                                name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                            },
                        })
                        .collect(),
                )
            },
            ..Default::default()
        }),
        Msg::Tool {
            tool_call_id,
            content,
        } => ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
            content: ChatCompletionRequestToolMessageContent::Text(content.clone()),
            tool_call_id: tool_call_id.clone(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestProcess;

    #[test]
    fn test_chat_config_requires_model() {
        let mut proc = TestProcess::new();
        proc.remove_var("LLMAN_SDD_INDEX_CHAT_MODEL");
        let res = ChatConfig::from_env();
        assert!(res.is_err(), "chat config without a chat model must error");
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("LLMAN_SDD_INDEX_CHAT_MODEL"));
    }
}
