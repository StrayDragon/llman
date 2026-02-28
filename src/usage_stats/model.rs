use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    Codex,
    ClaudeCode,
    Cursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for SessionId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Total tokens for the session, if known.
    pub total: Option<u64>,
    /// Input tokens, if known (tool-specific semantics).
    pub input: Option<u64>,
    /// Output tokens, if known (tool-specific semantics).
    pub output: Option<u64>,
    /// Cached input tokens, if known (tool-specific semantics).
    pub cache: Option<u64>,
    /// Reasoning output tokens, if known (tool-specific semantics).
    pub reasoning: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub tool: ToolKind,
    pub id: SessionId,
    pub cwd: PathBuf,
    pub title: Option<String>,
    pub start_ts: Option<DateTime<Utc>>,
    pub end_ts: DateTime<Utc>,
    pub token_usage: TokenUsage,
    /// Tool-specific marker for sidechain/subagent sessions (Claude Code).
    pub is_sidechain: Option<bool>,
}
