use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 分组消息（将连续的同类型消息合并）
#[derive(Debug)]
pub struct GroupedMessage {
    pub is_user: bool,
    pub content: String,
    pub bubble_count: usize,
}

/// 聊天数据根结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatData {
    pub tabs: Vec<ChatTab>,
}

/// 单个聊天标签页
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatTab {
    #[serde(rename = "tabId")]
    pub tab_id: Option<String>,
    #[serde(rename = "chatTitle")]
    pub chat_title: Option<String>,
    #[serde(rename = "lastSendTime")]
    pub last_send_time: Option<i64>,
    pub bubbles: Vec<ChatBubble>,
}

/// 聊天气泡消息
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatBubble {
    #[serde(rename = "type")]
    pub bubble_type: String,
    pub id: Option<String>,
    #[serde(rename = "messageType")]
    pub message_type: Option<u32>,
    #[serde(rename = "terminalSelections")]
    pub terminal_selections: Option<Vec<serde_json::Value>>,
    #[serde(rename = "fileSelections")]
    pub file_selections: Option<Vec<serde_json::Value>>,
    pub text: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

/// Composer数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerData {
    #[serde(rename = "allComposers")]
    pub all_composers: Vec<ComposerItem>,
    #[serde(rename = "selectedComposerIds")]
    pub selected_composer_ids: Option<Vec<String>>,
}

/// 单个Composer项目
#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerItem {
    #[serde(rename = "composerId")]
    pub composer_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "lastUpdatedAt")]
    pub last_updated_at: Option<i64>,
    #[serde(rename = "unifiedMode")]
    pub unified_mode: String,
    pub name: Option<String>,
}

/// 对话摘要信息
#[derive(Debug)]
pub struct ConversationSummary {
    pub title: String,
    pub last_message_time: DateTime<Utc>,
    pub message_count: usize,
    pub conversation_type: ConversationType,
}

/// 对话类型枚举
#[derive(Debug, Clone)]
pub enum ConversationType {
    Traditional, // 传统聊天对话
    #[allow(dead_code)]
    Composer(ComposerMode), // Composer对话
}

/// Composer气泡数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerBubble {
    #[serde(rename = "bubbleId", default)]
    pub bubble_id: Option<String>,
    #[serde(rename = "type", default)]
    pub bubble_type: Option<i32>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "codeBlocks", default)]
    pub code_blocks: Option<Vec<serde_json::Value>>,
    #[serde(rename = "assistantSuggestedDiffs", default)]
    pub assistant_suggested_diffs: Option<Vec<serde_json::Value>>,
    #[serde(rename = "humanChanges", default)]
    pub human_changes: Option<Vec<serde_json::Value>>,
    #[serde(rename = "toolResults", default)]
    pub tool_results: Option<Vec<serde_json::Value>>,
    #[serde(rename = "richText", default)]
    pub rich_text: Option<serde_json::Value>,
    #[serde(rename = "contextPieces", default)]
    pub context_pieces: Option<Vec<serde_json::Value>>,
    #[serde(rename = "attachedCodeChunks", default)]
    pub attached_code_chunks: Option<Vec<serde_json::Value>>,
    #[serde(rename = "relevantFiles", default)]
    pub relevant_files: Option<Vec<serde_json::Value>>,
    #[serde(rename = "suggestedCodeBlocks", default)]
    pub suggested_code_blocks: Option<Vec<serde_json::Value>>,
    #[serde(rename = "gitDiffs", default)]
    pub git_diffs: Option<Vec<serde_json::Value>>,
    // 其他字段都使用flatten捕获
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 包含bubble数据的Composer对话
#[derive(Debug)]
pub struct ComposerWithBubbles {
    pub composer_data: ComposerItem,
    pub bubbles: Vec<ComposerBubble>,
}

/// Composer模式枚举
#[derive(Debug, Clone)]
pub enum ComposerMode {
    Chat,  // 聊天模式
    Agent, // 代理模式
    Edit,  // 编辑模式
}

/// 统一的对话导出数据
#[derive(Debug)]
pub enum ConversationExport {
    Traditional(ChatTab),
    Composer(ComposerWithBubbles),
}

/// Workspace信息
#[derive(Debug)]
pub struct WorkspaceInfo {
    pub db_path: std::path::PathBuf,
    pub project_path: Option<std::path::PathBuf>,
    pub project_name: String,
    pub has_chat_data: bool,
}

/// Workspace元数据（用于解析workspace.json）
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    pub folder: String,
}

// ====== 实现方法 ======

impl ChatTab {
    /// 获取聊天标题
    pub fn get_title(&self) -> String {
        self.chat_title
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| t!("cursor.export.untitled_conversation").to_string())
    }

    /// 获取最后发送时间
    pub fn get_last_send_time(&self) -> DateTime<Utc> {
        let timestamp = self.last_send_time.unwrap_or(0);
        DateTime::from_timestamp_millis(timestamp).unwrap_or_else(Utc::now)
    }

    /// 转换为Markdown格式
    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();

        markdown.push_str(&format!("# {}\n\n", self.get_title()));
        markdown.push_str(&t!(
            "cursor.export.last_updated",
            time = self.get_last_send_time().format("%Y-%m-%d %H:%M:%S")
        ));

        for (i, bubble) in self.bubbles.iter().enumerate() {
            let content = self.extract_bubble_content(bubble);
            if !content.trim().is_empty() {
                let speaker = if bubble.bubble_type == "user" {
                    t!("cursor.export.user_message")
                } else {
                    t!("cursor.export.ai_message")
                };
                markdown.push_str(&format!(
                    "## {} ({}): \n\n{}\n\n",
                    speaker,
                    t!("cursor.export.message_number", number = i + 1),
                    content
                ));
            }
        }

        markdown.push_str("---\n\n");
        markdown
    }

    /// 从气泡中提取文本内容
    fn extract_bubble_content(&self, bubble: &ChatBubble) -> String {
        if let Some(text) = &bubble.text {
            if !text.trim().is_empty() {
                return text.clone();
            }
        }

        if let Some(terminal_selections) = &bubble.terminal_selections {
            let mut content = Vec::new();
            for selection in terminal_selections {
                if let Some(text) = selection.get("text") {
                    if let Some(text_str) = text.as_str() {
                        if !text_str.trim().is_empty() {
                            content.push(text_str);
                        }
                    }
                }
            }
            if !content.is_empty() {
                return content.join("\n");
            }
        }

        "*empty message*".to_string()
    }
}

impl ComposerItem {
    /// 获取标题
    pub fn get_title(&self) -> String {
        if let Some(name) = &self.name {
            if !name.trim().is_empty() {
                return name.clone();
            }
        }

        // 如果没有设置name，使用默认标题
        format!(
            "{} - {}",
            t!("cursor.export.composer_conversation"),
            self.composer_id.chars().take(8).collect::<String>()
        )
    }

    /// 获取最后更新时间
    pub fn get_last_updated_time(&self) -> DateTime<Utc> {
        let timestamp = self.last_updated_at.unwrap_or(self.created_at);
        DateTime::from_timestamp_millis(timestamp).unwrap_or_else(Utc::now)
    }

    /// 获取Composer模式
    pub fn get_composer_mode(&self) -> ComposerMode {
        match self.unified_mode.as_str() {
            "chat" => ComposerMode::Chat,
            "agent" => ComposerMode::Agent,
            "edit" => ComposerMode::Edit,
            _ => ComposerMode::Chat, // 默认为聊天模式
        }
    }
}

impl ConversationExport {
    /// 转换为Markdown格式
    pub fn to_markdown(&self) -> String {
        match self {
            ConversationExport::Traditional(tab) => tab.to_markdown(),
            ConversationExport::Composer(composer) => composer.to_markdown(),
        }
    }

    /// 获取对话标题
    pub fn get_title(&self) -> String {
        match self {
            ConversationExport::Traditional(tab) => tab.get_title(),
            ConversationExport::Composer(composer) => composer.composer_data.get_title(),
        }
    }
}

impl WorkspaceInfo {
    /// 获取显示名称
    pub fn display_name(&self) -> String {
        if self.has_chat_data {
            if self.project_path.is_some() {
                format!(
                    "🌟 {} ({})",
                    self.project_name,
                    self.project_path.as_ref().unwrap().display()
                )
            } else {
                format!("🌟 {} [Unknown path]", self.project_name)
            }
        } else if self.project_path.is_some() {
            format!(
                "{} ({})",
                self.project_name,
                self.project_path.as_ref().unwrap().display()
            )
        } else {
            format!("{} [Unknown path]", self.project_name)
        }
    }
}

impl ComposerBubble {
    /// 获取显示内容
    pub fn get_display_content(&self) -> String {
        let mut content = Vec::new();

        // 根据bubble类型处理内容
        if self.is_user_message() {
            // 用户消息：优先使用text字段
            if let Some(text) = &self.text {
                if !text.trim().is_empty() {
                    content.push(text.clone());
                }
            }

            // 如果text为空，尝试从richText中提取
            if content.is_empty() {
                if let Some(rich_text) = &self.rich_text {
                    if let Ok(parsed) =
                        serde_json::from_value::<serde_json::Value>(rich_text.clone())
                    {
                        if let Some(extracted) = self.extract_text_from_rich_text(&parsed) {
                            if !extracted.trim().is_empty() {
                                content.push(extracted);
                            }
                        }
                    }
                }
            }
        } else {
            // AI消息：检查text字段
            if let Some(text) = &self.text {
                if !text.trim().is_empty() {
                    content.push(text.clone());
                }
            }

            // 如果text为空，检查工具调用结果
            if content.is_empty() {
                if let Some(tool_data) = self.extra.get("toolFormerData") {
                    if let Some(tool_summary) = self.extract_tool_summary(tool_data) {
                        content.push(tool_summary);
                    }
                }
            }
        }

        // 处理代码块（以折叠形式显示）
        if let Some(code_blocks) = &self.code_blocks {
            if !code_blocks.is_empty() {
                content.push(format!(
                    "<details>\n<summary>📄 代码块 ({})</summary>\n\n*内容已折叠*\n\n</details>",
                    code_blocks.len()
                ));
            }
        }

        // 处理AI建议的差异（以折叠形式显示）
        if let Some(assistant_suggested_diffs) = &self.assistant_suggested_diffs {
            if !assistant_suggested_diffs.is_empty() {
                content.push(format!("<details>\n<summary>🤖 AI建议差异 ({})</summary>\n\n*内容已折叠*\n\n</details>", assistant_suggested_diffs.len()));
            }
        }

        if content.is_empty() {
            format!("*空消息 (type: {:?})*", self.bubble_type)
        } else {
            content.join("\n\n")
        }
    }

    /// 从richText中提取纯文本内容
    fn extract_text_from_rich_text(&self, rich_text: &serde_json::Value) -> Option<String> {
        fn extract_text_recursive(value: &serde_json::Value) -> String {
            match value {
                serde_json::Value::Object(obj) => {
                    if let Some(text) = obj.get("text") {
                        if let Some(text_str) = text.as_str() {
                            return text_str.to_string();
                        }
                    }
                    if let Some(children) = obj.get("children") {
                        if let Some(children_array) = children.as_array() {
                            return children_array
                                .iter()
                                .map(extract_text_recursive)
                                .collect::<Vec<_>>()
                                .join("");
                        }
                    }
                    String::new()
                }
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(extract_text_recursive)
                    .collect::<Vec<_>>()
                    .join(""),
                _ => String::new(),
            }
        }

        let extracted = extract_text_recursive(rich_text);
        if extracted.trim().is_empty() {
            None
        } else {
            Some(extracted)
        }
    }

    /// 从工具调用数据中提取摘要信息
    fn extract_tool_summary(&self, tool_data: &serde_json::Value) -> Option<String> {
        if let Some(tool_obj) = tool_data.as_object() {
            let tool_name = tool_obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("未知工具");

            let status = tool_obj
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("未知状态");

            // 尝试从result中提取有用信息
            if let Some(result_str) = tool_obj.get("result").and_then(|v| v.as_str()) {
                if let Ok(result_obj) = serde_json::from_str::<serde_json::Value>(result_str) {
                    // 对于不同的工具类型提取不同的信息
                    match tool_name {
                        "read_file" => {
                            if let Some(contents) =
                                result_obj.get("contents").and_then(|v| v.as_str())
                            {
                                let preview = if contents.len() > 200 {
                                    format!("{}...", contents.chars().take(200).collect::<String>())
                                } else {
                                    contents.to_string()
                                };
                                return Some(format!(
                                    "🔍 **读取文件**: {status}\n\n```\n{preview}\n```"
                                ));
                            }
                        }
                        "run_terminal_cmd" => {
                            if let Some(output) = result_obj.get("output").and_then(|v| v.as_str())
                            {
                                let preview = if output.len() > 300 {
                                    format!("{}...", output.chars().take(300).collect::<String>())
                                } else {
                                    output.to_string()
                                };
                                return Some(format!(
                                    "💻 **执行命令**: {status}\n\n```\n{preview}\n```"
                                ));
                            }
                        }
                        _ => {
                            // 其他工具的通用处理
                            return Some(format!("🔧 **{tool_name}**: {status}"));
                        }
                    }
                }
            }

            return Some(format!("🔧 **{tool_name}**: {status}"));
        }
        None
    }

    /// 判断是否为用户消息
    pub fn is_user_message(&self) -> bool {
        // 最关键的判断：type=1是用户消息，type=2是AI回复
        if let Some(bubble_type) = self.bubble_type {
            return bubble_type == 1;
        }

        // 如果没有type字段，则使用其他逻辑
        // 如果有toolFormerData，那是AI的工具调用或回复
        if self.extra.contains_key("toolFormerData") {
            return false;
        }

        // 如果有其他AI特征，那是AI回复
        if self.extra.contains_key("usageUuid") {
            return false;
        }

        // 如果有代码块，那是AI回复
        if let Some(code_blocks) = &self.code_blocks {
            if !code_blocks.is_empty() {
                return false;
            }
        }

        // 如果有AI建议的差异，那是AI回复
        if let Some(assistant_suggested_diffs) = &self.assistant_suggested_diffs {
            if !assistant_suggested_diffs.is_empty() {
                return false;
            }
        }

        // 如果有工具结果，那是AI回复
        if let Some(tool_results) = &self.tool_results {
            if !tool_results.is_empty() {
                return false;
            }
        }

        // 默认认为是用户消息
        true
    }
}

impl ComposerWithBubbles {
    /// 获取分组后的消息（基于元信息智能分段）
    pub fn get_grouped_messages(&self) -> Vec<GroupedMessage> {
        let mut grouped_messages = Vec::new();
        let mut current_group: Option<GroupedMessage> = None;

        for bubble in self.bubbles.iter() {
            let is_user = bubble.is_user_message();
            let content = bubble.get_display_content();

            // 跳过空内容的消息
            if content.trim().is_empty() {
                continue;
            }

            match &mut current_group {
                Some(group) => {
                    if is_user {
                        // 遇到用户消息，结束当前组并开始新组
                        grouped_messages.push(current_group.take().unwrap());
                        current_group = Some(GroupedMessage {
                            is_user,
                            content,
                            bubble_count: 1,
                        });
                    } else if group.is_user {
                        // 从用户消息切换到AI消息，开始新组
                        grouped_messages.push(current_group.take().unwrap());
                        current_group = Some(GroupedMessage {
                            is_user,
                            content,
                            bubble_count: 1,
                        });
                    } else {
                        // 都是AI消息，检查是否应该分段
                        let should_split = self.should_split_ai_messages(group, bubble);
                        if should_split {
                            // 分段，结束当前组并开始新组
                            grouped_messages.push(current_group.take().unwrap());
                            current_group = Some(GroupedMessage {
                                is_user,
                                content,
                                bubble_count: 1,
                            });
                        } else {
                            // 合并到当前组
                            if !group.content.trim().is_empty() && !content.trim().is_empty() {
                                group.content.push_str("\n\n");
                            }
                            group.content.push_str(&content);
                            group.bubble_count += 1;
                        }
                    }
                }
                None => {
                    // 开始第一个组
                    current_group = Some(GroupedMessage {
                        is_user,
                        content,
                        bubble_count: 1,
                    });
                }
            }
        }

        // 添加最后一个组
        if let Some(group) = current_group {
            grouped_messages.push(group);
        }

        grouped_messages
    }

    /// 判断是否应该分割AI消息（根据元信息）
    fn should_split_ai_messages(
        &self,
        _current_group: &GroupedMessage,
        _bubble: &ComposerBubble,
    ) -> bool {
        // 对于AI消息，目前简单地合并到一起
        // 后续可以根据其他字段来判断是否分段
        false
    }

    /// 转换为Markdown格式（使用分组消息）
    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();

        let mode_display = match self.composer_data.get_composer_mode() {
            ComposerMode::Chat => t!("cursor.export.composer_mode_chat"),
            ComposerMode::Agent => t!("cursor.export.composer_mode_agent"),
            ComposerMode::Edit => t!("cursor.export.composer_mode_edit"),
        };

        markdown.push_str(&format!(
            "# {} [Composer {}]\n\n",
            self.composer_data.get_title(),
            mode_display
        ));
        markdown.push_str(&t!(
            "cursor.export.last_updated",
            time = self
                .composer_data
                .get_last_updated_time()
                .format("%Y-%m-%d %H:%M:%S")
        ));

        markdown.push_str(&format!(
            "\n**Composer ID:** {}\n",
            self.composer_data.composer_id
        ));
        markdown.push_str(&format!(
            "**{}:** {}\n\n",
            t!("cursor.export.composer_mode_label"),
            self.composer_data.unified_mode
        ));

        // 使用分组消息来生成内容
        let grouped_messages = self.get_grouped_messages();
        for (i, group) in grouped_messages.iter().enumerate() {
            let speaker = if group.is_user {
                t!("cursor.export.user_message")
            } else {
                t!("cursor.export.ai_message")
            };

            let message_info = if group.bubble_count > 1 {
                format!(
                    "{} ({}, {} bubbles)",
                    speaker,
                    t!("cursor.export.message_number", number = i + 1),
                    group.bubble_count
                )
            } else {
                format!(
                    "{} ({})",
                    speaker,
                    t!("cursor.export.message_number", number = i + 1)
                )
            };

            markdown.push_str(&format!("## {}: \n\n{}\n\n", message_info, group.content));
        }

        markdown.push_str("---\n\n");
        markdown
    }
}
