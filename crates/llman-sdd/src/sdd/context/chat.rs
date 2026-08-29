//! Blocking OpenAI-compatible chat adapter for the pageindex agentic retrieval.
//!
//! [`ChatConfig`] resolves the chat-model configuration separately from the
//! embedding model (per the sdd-context spec: `LLMAN_SDD_INDEX_CHAT_*` env vars
//! fall back to `LLMAN_SDD_INDEX_OPENAI_*`). [`OpenAiInvoker`] implements
//! [`crate::sdd::context::retrieve::ChatInvoker`] by posting our lightweight
//! protocol types to the `/chat/completions` endpoint via `ureq` (blocking,
//! rustls) — one JSON round-trip per turn, no async runtime needed.

use crate::sdd::context::retrieve::{ChatInvoker, ChatTurn, Msg, ToolCall, ToolSchema};
use anyhow::{Context as _, Result};
use serde_json::{Value, json};

/// Configuration for the chat model used by pageindex retrieval.
#[derive(Debug, Clone)]
pub(crate) struct ChatConfig {
    pub(crate) api_host: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

impl ChatConfig {
    /// Resolve chat config from the environment.
    ///
    /// Priority: `LLMAN_SDD_INDEX_CHAT_*` → fall back to `LLMAN_SDD_INDEX_OPENAI_*`
    /// (host/key) → hardcoded host default. The chat model has no default — it
    /// must support tool/function calling and be set via `LLMAN_SDD_INDEX_CHAT_MODEL`.
    pub(crate) fn from_env() -> Result<Self> {
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

/// Blocking OpenAI-compatible [`ChatInvoker`].
pub(crate) struct OpenAiInvoker {
    api_host: String,
    api_key: String,
    model: String,
}

impl OpenAiInvoker {
    pub(crate) fn new(cfg: &ChatConfig) -> Self {
        Self {
            api_host: cfg.api_host.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
            model: cfg.model.clone(),
        }
    }

    fn post_chat_completions(&self, body: &Value) -> Result<Value> {
        // One shared agent with `http_status_as_error(false)`: non-2xx comes
        // back as a normal response so the server's error body can be surfaced.
        static AGENT: std::sync::LazyLock<ureq::Agent> = std::sync::LazyLock::new(|| {
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build()
                .new_agent()
        });

        let url = format!("{}/chat/completions", self.api_host);
        let mut request = AGENT.post(&url).header("Content-Type", "application/json");
        if !self.api_key.is_empty() {
            request = request.header("Authorization", &format!("Bearer {}", self.api_key));
        }
        let mut response = request
            .send_json(body)
            .with_context(|| format!("chat completion via {} failed", self.api_host))?;

        let status = response.status().as_u16();
        let payload = response.body_mut().read_json::<Value>().with_context(|| {
            format!(
                "chat completion via {} returned invalid JSON (HTTP {status})",
                self.api_host
            )
        })?;

        if !(200..300).contains(&status) {
            let snippet = serde_json::to_string_pretty(&payload).unwrap_or_default();
            let snippet: String = snippet.chars().take(300).collect();
            anyhow::bail!(
                "chat completion via {} failed: HTTP {status}\n{snippet}",
                self.api_host
            );
        }
        Ok(payload)
    }
}

impl ChatInvoker for OpenAiInvoker {
    fn chat_turn(&self, messages: &[Msg], tools: &[ToolSchema]) -> Result<ChatTurn> {
        let body = json!({
            "model": self.model,
            "messages": messages.iter().map(message_to_json).collect::<Vec<_>>(),
            "tools": tools.iter().map(tool_to_json).collect::<Vec<_>>(),
        });

        let payload = self.post_chat_completions(&body)?;

        let message = payload
            .pointer("/choices/0/message")
            .ok_or_else(|| anyhow::anyhow!("chat response had no choices"))?;

        let tool_calls = message
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|calls| calls.iter().filter_map(parse_tool_call).collect())
            .unwrap_or_default();

        Ok(ChatTurn {
            content: message
                .get("content")
                .and_then(Value::as_str)
                .map(str::to_string),
            tool_calls,
        })
    }
}

/// Convert our protocol [`Msg`] into the wire JSON for the chat endpoint.
fn message_to_json(msg: &Msg) -> Value {
    match msg {
        Msg::System(s) => json!({ "role": "system", "content": s }),
        Msg::User(s) => json!({ "role": "user", "content": s }),
        Msg::Assistant {
            content,
            tool_calls,
        } => {
            let mut m = json!({
                "role": "assistant",
                "content": content,
            });
            if !tool_calls.is_empty() {
                m["tool_calls"] = json!(
                    tool_calls
                        .iter()
                        .map(|tc| json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments,
                            },
                        }))
                        .collect::<Vec<_>>()
                );
            }
            m
        }
        Msg::Tool {
            tool_call_id,
            content,
        } => json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": content,
        }),
    }
}

fn tool_to_json(t: &ToolSchema) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": t.name,
            "description": t.description,
            "parameters": t.parameters,
        },
    })
}

fn parse_tool_call(call: &Value) -> Option<ToolCall> {
    let function = call.get("function")?;
    Some(ToolCall {
        id: call.get("id")?.as_str()?.to_string(),
        name: function.get("name")?.as_str()?.to_string(),
        arguments: function.get("arguments")?.as_str()?.to_string(),
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

    #[test]
    fn message_to_json_matches_openai_wire_format() {
        let msgs = [
            Msg::System("sys".to_string()),
            Msg::User("hi".to_string()),
            Msg::Assistant {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "c1".to_string(),
                    name: "lookup".to_string(),
                    arguments: "{}".to_string(),
                }],
            },
            Msg::Tool {
                tool_call_id: "c1".to_string(),
                content: "result".to_string(),
            },
        ];
        let wire: Vec<Value> = msgs.iter().map(message_to_json).collect();
        assert_eq!(wire[0]["role"], "system");
        assert_eq!(wire[1]["role"], "user");
        assert_eq!(wire[2]["role"], "assistant");
        assert_eq!(wire[2]["content"], Value::Null);
        assert_eq!(wire[2]["tool_calls"][0]["id"], "c1");
        assert_eq!(wire[2]["tool_calls"][0]["function"]["name"], "lookup");
        assert_eq!(wire[3]["role"], "tool");
        assert_eq!(wire[3]["tool_call_id"], "c1");
    }

    #[test]
    fn parse_tool_call_reads_openai_shape() {
        let call = json!({
            "id": "call_1",
            "type": "function",
            "function": { "name": "search", "arguments": "{\"q\":1}" },
        });
        let tc = parse_tool_call(&call).expect("parses");
        assert_eq!(tc.name, "search");
        assert_eq!(tc.arguments, "{\"q\":1}");
        assert!(parse_tool_call(&json!({"id": "x"})).is_none());
    }
}
