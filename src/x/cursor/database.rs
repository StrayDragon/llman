use crate::error::Result;
use crate::x::cursor::models::*;
use glob::glob;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Cursor数据库操作类
pub struct CursorDatabase {
    db_path: PathBuf,
    global_db_path: Option<PathBuf>,
}

impl CursorDatabase {
    /// 创建数据库实例
    ///
    /// # 参数
    /// * `custom_path` - 自定义数据库路径，None时自动查找最新的workspace数据库
    pub fn new(custom_path: Option<&str>) -> Result<Self> {
        Self::new_with_options(custom_path, true)
    }

    pub fn new_without_global(custom_path: Option<&str>) -> Result<Self> {
        Self::new_with_options(custom_path, false)
    }

    fn new_with_options(custom_path: Option<&str>, include_global_db: bool) -> Result<Self> {
        let db_path = if let Some(path) = custom_path {
            PathBuf::from(path)
        } else {
            Self::find_latest_workspace_db()?
        };

        // 获取全局数据库路径
        let global_db_path = if include_global_db {
            Self::get_global_db_path().ok()
        } else {
            None
        };

        println!(
            "{}: {}\n",
            t!("cursor.database.using_db"),
            db_path.display()
        );
        if let Some(ref global_path) = global_db_path {
            println!(
                "{}: {}\n",
                t!("cursor.database.using_global_db"),
                global_path.display()
            );
        }

        Ok(Self {
            db_path,
            global_db_path,
        })
    }

    /// 连接到数据库
    ///
    /// # Safety
    /// Uses `SQLITE_OPEN_NO_MUTEX` — the connection is read-only and used from
    /// a single thread (cursor export processes one conversation at a time).
    /// Do NOT add `SQLITE_OPEN_FULL_MUTEX` unless the connection is shared
    /// across threads.
    pub fn connect(&self) -> Result<Connection> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        Ok(Connection::open_with_flags(&self.db_path, flags)?)
    }

    /// 连接到全局数据库
    ///
    /// # Safety
    /// Same single-threaded, read-only guarantee as [`Self::connect`].
    pub fn connect_global(&self) -> Result<Option<Connection>> {
        if let Some(ref global_path) = self.global_db_path {
            let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
            Ok(Some(Connection::open_with_flags(global_path, flags)?))
        } else {
            Ok(None)
        }
    }

    /// 获取对话摘要列表
    ///
    /// 包含传统聊天对话和Composer对话，按时间排序
    pub fn get_conversation_summaries(&self) -> Result<Vec<ConversationSummary>> {
        let chat_data = self.get_chat_data()?;
        let composer_data = self.get_composer_data()?;

        let mut summaries = Vec::new();

        // 添加传统聊天对话
        if let Some(data) = chat_data {
            for tab in data.tabs {
                summaries.push(ConversationSummary {
                    key: tab.conversation_key(),
                    title: tab.get_title(),
                    last_message_time: tab.get_last_send_time(),
                    message_count: tab.bubbles.len(),
                    conversation_type: ConversationType::Traditional,
                });
            }
        }

        // 添加Composer对话
        if let Some(data) = composer_data {
            for composer in data.all_composers {
                // 从全局数据库获取bubble数量
                let bubble_count = self.get_composer_bubble_count(&composer.composer_id)?;

                summaries.push(ConversationSummary {
                    key: composer.conversation_key(),
                    title: composer.get_title(),
                    last_message_time: composer.get_last_updated_time(),
                    message_count: bubble_count,
                    conversation_type: ConversationType::Composer(composer.get_composer_mode()),
                });
            }
        }

        // 按时间排序，最新的在前面
        summaries.sort_by_key(|summary| Reverse(summary.last_message_time));

        Ok(summaries)
    }

    /// 获取所有对话的混合导出数据
    pub fn get_all_conversations_mixed(&self) -> Result<Vec<ConversationExport>> {
        let chat_data = self.get_chat_data()?;
        let composer_data = self.get_composer_data()?;

        let mut conversations = Vec::new();

        // 添加传统聊天对话
        if let Some(data) = chat_data {
            for tab in data.tabs {
                conversations.push(ConversationExport::Traditional(tab));
            }
        }

        // 添加Composer对话
        if let Some(data) = composer_data {
            for composer in data.all_composers {
                // 从全局数据库获取完整的composer对话数据
                let bubbles = self.get_composer_bubbles(&composer.composer_id)?;
                let full_composer = ComposerWithBubbles {
                    composer_data: composer,
                    bubbles,
                };
                conversations.push(ConversationExport::Composer(full_composer));
            }
        }

        Ok(conversations)
    }

    pub fn get_conversation_exports_by_keys(
        &self,
        keys: &[ConversationKey],
    ) -> Result<Vec<ConversationExport>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut composer_ids = HashSet::new();
        let mut traditional_keys = HashSet::new();
        for key in keys {
            match key {
                ConversationKey::Composer(id) => {
                    composer_ids.insert(id.clone());
                }
                ConversationKey::Traditional(_) => {
                    traditional_keys.insert(key.clone());
                }
            }
        }

        let mut tabs_by_key: HashMap<ConversationKey, ChatTab> = HashMap::new();
        if !traditional_keys.is_empty()
            && let Some(chat_data) = self.get_chat_data()?
        {
            for tab in chat_data.tabs {
                let key = tab.conversation_key();
                if traditional_keys.contains(&key) {
                    tabs_by_key.insert(key, tab);
                }
            }
        }

        let mut composers_by_id: HashMap<String, ComposerItem> = HashMap::new();
        if !composer_ids.is_empty()
            && let Some(composer_data) = self.get_composer_data()?
        {
            for composer in composer_data.all_composers {
                if composer_ids.contains(&composer.composer_id) {
                    composers_by_id.insert(composer.composer_id.clone(), composer);
                }
            }
        }

        let mut exports = Vec::new();
        let mut seen = HashSet::new();
        for key in keys {
            if !seen.insert(key.clone()) {
                continue;
            }

            match key {
                ConversationKey::Traditional(_) => {
                    let tab = tabs_by_key.remove(key).ok_or_else(|| {
                        crate::error::LlmanError::Custom(
                            t!(
                                "cursor.export.selected_conversation_not_found",
                                id = key.short_id()
                            )
                            .to_string(),
                        )
                    })?;
                    exports.push(ConversationExport::Traditional(tab));
                }
                ConversationKey::Composer(composer_id) => {
                    let composer = composers_by_id.remove(composer_id).ok_or_else(|| {
                        crate::error::LlmanError::Custom(
                            t!(
                                "cursor.export.selected_conversation_not_found",
                                id = key.short_id()
                            )
                            .to_string(),
                        )
                    })?;
                    let bubbles = self.get_composer_bubbles(&composer.composer_id)?;
                    exports.push(ConversationExport::Composer(ComposerWithBubbles {
                        composer_data: composer,
                        bubbles,
                    }));
                }
            }
        }

        Ok(exports)
    }

    /// 搜索对话（仅支持传统聊天对话）
    pub fn search_conversations(&self, search_text: &str) -> Result<Vec<ChatTab>> {
        let chat_data = self.get_chat_data()?;

        if let Some(data) = chat_data {
            let search_lower = search_text.to_lowercase();
            let filtered_tabs =
                data.tabs
                    .into_iter()
                    .filter(|tab| {
                        // 搜索标题
                        if let Some(title) = &tab.chat_title
                            && title.to_lowercase().contains(&search_lower)
                        {
                            return true;
                        }

                        // 搜索消息内容
                        tab.bubbles.iter().any(|bubble| {
                            if let Some(text) = &bubble.text {
                                text.to_lowercase().contains(&search_lower)
                            } else if let Some(terminal_selections) = &bubble.terminal_selections {
                                terminal_selections.iter().any(|selection| {
                                    selection.get("text").and_then(|v| v.as_str()).is_some_and(
                                        |text| text.to_lowercase().contains(&search_lower),
                                    )
                                })
                            } else {
                                false
                            }
                        })
                    })
                    .collect();

            Ok(filtered_tabs)
        } else {
            Ok(vec![])
        }
    }

    /// 发现所有可用的workspace数据库
    pub fn find_all_workspaces() -> Result<Vec<WorkspaceInfo>> {
        let base_path = Self::get_cursor_workspace_path()?;

        if !base_path.exists() {
            return Err(crate::error::LlmanError::Custom(
                t!(
                    "cursor.database.workspace_path_not_exist",
                    path = base_path.display()
                )
                .to_string(),
            ));
        }

        let pattern = base_path.join("*").join("state.vscdb");
        let db_files: Vec<PathBuf> = glob(&pattern.to_string_lossy())?
            .filter_map(|entry| entry.ok())
            .collect();

        if db_files.is_empty() {
            return Err(crate::error::LlmanError::Custom(
                t!("cursor.database.no_workspace_db_found").to_string(),
            ));
        }

        let mut workspaces = Vec::new();

        for db_path in db_files {
            if let Some(parent) = db_path.parent()
                && let Some(_hash_id) = parent.file_name().and_then(|n| n.to_str())
            {
                let workspace_info =
                    Self::create_workspace_info(db_path.clone(), parent.to_path_buf())?;
                workspaces.push(workspace_info);
            }
        }

        // 获取当前工作目录
        let current_dir = std::env::current_dir().ok();

        // 分离当前目录对应的workspace和其他workspace
        let (mut current_workspaces, mut other_workspaces): (Vec<_>, Vec<_>) =
            workspaces.into_iter().partition(|w| {
                if let Some(ref project_path) = w.project_path
                    && let Some(ref current) = current_dir
                {
                    return project_path == current;
                }
                false
            });

        // 排序：聊天数据优先，然后按修改时间
        let sort_key = |w: &WorkspaceInfo| {
            let has_chat = if w.has_chat_data { 0 } else { 1 };
            let modified_time = fs::metadata(&w.db_path)
                .and_then(|m| m.modified())
                .map(std::cmp::Reverse)
                .unwrap_or(std::cmp::Reverse(std::time::UNIX_EPOCH));
            (has_chat, modified_time)
        };

        current_workspaces.sort_by_key(sort_key);
        other_workspaces.sort_by_key(sort_key);

        // 合并结果：当前目录workspace在前
        current_workspaces.extend(other_workspaces);
        Ok(current_workspaces)
    }

    // ====== 私有方法 ======

    /// 查找最新的workspace数据库
    fn find_latest_workspace_db() -> Result<PathBuf> {
        let base_path = Self::get_cursor_workspace_path()?;

        if !base_path.exists() {
            return Err(crate::error::LlmanError::Custom(
                t!(
                    "cursor.database.workspace_path_not_exist",
                    path = base_path.display()
                )
                .to_string(),
            ));
        }

        let pattern = base_path.join("*").join("state.vscdb");
        let db_files: Vec<PathBuf> = glob(&pattern.to_string_lossy())?
            .filter_map(|entry| entry.ok())
            .collect();

        if db_files.is_empty() {
            return Err(crate::error::LlmanError::Custom(
                t!("cursor.database.no_workspace_db_found").to_string(),
            ));
        }

        let mut db_files_with_metadata: Vec<(PathBuf, std::time::SystemTime)> = db_files
            .into_iter()
            .filter_map(|path| {
                fs::metadata(&path)
                    .and_then(|m| m.modified())
                    .map(|time| (path, time))
                    .ok()
            })
            .collect();

        db_files_with_metadata.sort_by_key(|entry| Reverse(entry.1));

        let chat_db = db_files_with_metadata
            .iter()
            .find(|(path, _)| Self::has_chat_data(path).unwrap_or(false))
            .map(|(path, _)| path.clone());

        Ok(chat_db.unwrap_or_else(|| db_files_with_metadata[0].0.clone()))
    }

    /// 检查数据库是否包含聊天数据
    ///
    /// # Safety
    /// Same single-threaded, read-only guarantee as [`Self::connect`].
    fn has_chat_data(db_path: &Path) -> Result<bool> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection = match Connection::open_with_flags(db_path, flags) {
            Ok(conn) => conn,
            Err(_) => return Ok(false),
        };

        let traditional_chat: Option<Vec<u8>> = connection
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?1",
                ["workbench.panel.aichat.view.aichat.chatdata"],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or(None);

        let composer_chat: Option<Vec<u8>> = connection
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?1",
                ["composer.composerData"],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or(None);

        let has_traditional = traditional_chat
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|json_str| serde_json::from_str::<ChatData>(&json_str).ok())
            .is_some_and(|data| !data.tabs.is_empty());

        let has_composer = composer_chat
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|json_str| serde_json::from_str::<ComposerData>(&json_str).ok())
            .is_some_and(|data| !data.all_composers.is_empty());

        Ok(has_traditional || has_composer)
    }

    /// 获取Cursor工作区路径
    fn get_cursor_workspace_path() -> Result<PathBuf> {
        let home_dir = crate::config::try_home_dir().ok_or_else(|| {
            crate::error::LlmanError::Custom(t!("cursor.database.home_dir_error").to_string())
        })?;

        #[cfg(target_os = "windows")]
        let cursor_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("Cursor")
            .join("User")
            .join("workspaceStorage");

        #[cfg(target_os = "macos")]
        let cursor_path = home_dir
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join("User")
            .join("workspaceStorage");

        #[cfg(target_os = "linux")]
        let cursor_path = home_dir
            .join(".config")
            .join("Cursor")
            .join("User")
            .join("workspaceStorage");

        if !cursor_path.exists() {
            return Err(crate::error::LlmanError::Custom(
                t!(
                    "cursor.database.cursor_dir_not_found",
                    path = cursor_path.display()
                )
                .to_string(),
            ));
        }

        Ok(cursor_path)
    }

    /// 获取传统聊天数据
    fn get_chat_data(&self) -> Result<Option<ChatData>> {
        let connection = self.connect()?;

        let chat_json: Option<Vec<u8>> = connection
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?1",
                ["workbench.panel.aichat.view.aichat.chatdata"],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(bytes) = chat_json {
            let json_str = String::from_utf8(bytes)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            let data: ChatData = serde_json::from_str(&json_str)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// 获取Composer数据
    fn get_composer_data(&self) -> Result<Option<ComposerData>> {
        let connection = self.connect()?;

        let composer_json: Option<Vec<u8>> = connection
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?1",
                ["composer.composerData"],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(bytes) = composer_json {
            let json_str = String::from_utf8(bytes)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            let data: ComposerData = serde_json::from_str(&json_str)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// 创建WorkspaceInfo实例
    fn create_workspace_info(db_path: PathBuf, workspace_dir: PathBuf) -> Result<WorkspaceInfo> {
        let has_chat_data = Self::has_chat_data(&db_path).unwrap_or(false);

        let workspace_json_path = workspace_dir.join("workspace.json");
        let project_info = if workspace_json_path.exists() {
            fs::read_to_string(&workspace_json_path)
                .ok()
                .and_then(|content| serde_json::from_str::<WorkspaceMetadata>(&content).ok())
                .and_then(|metadata| Self::map_workspace_folder_to_path(&metadata.folder))
        } else {
            None
        };

        let (project_path, project_name) = if let Some((path, name)) = project_info {
            (Some(path), name)
        } else {
            // 如果没有项目信息，使用workspace目录名作为默认名称
            let default_name = workspace_dir
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.to_string())
                .unwrap_or_else(|| t!("cursor.workspace.unknown_name").to_string());
            (None, default_name)
        };

        Ok(WorkspaceInfo {
            db_path,
            project_path,
            project_name,
            has_chat_data,
        })
    }

    /// 映射workspace目录到项目路径
    fn map_workspace_folder_to_path(folder: &str) -> Option<(PathBuf, String)> {
        if let Some(path_str) = folder.strip_prefix("file://") {
            let path = PathBuf::from(path_str);
            let name = path.file_name()?.to_str()?.to_string();
            Some((path, name))
        } else {
            None
        }
    }

    /// 获取全局数据库路径
    pub fn get_global_db_path() -> Result<PathBuf> {
        let home_dir = crate::config::try_home_dir().ok_or_else(|| {
            crate::error::LlmanError::Custom(t!("cursor.database.home_dir_error").to_string())
        })?;

        #[cfg(target_os = "windows")]
        let global_db_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("state.vscdb");

        #[cfg(target_os = "macos")]
        let global_db_path = home_dir
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("state.vscdb");

        #[cfg(target_os = "linux")]
        let global_db_path = home_dir
            .join(".config")
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("state.vscdb");

        if !global_db_path.exists() {
            return Err(crate::error::LlmanError::Custom(
                t!(
                    "cursor.database.global_db_not_found",
                    path = global_db_path.display()
                )
                .to_string(),
            ));
        }

        Ok(global_db_path)
    }

    /// 获取composer对话的bubble数量
    fn get_composer_bubble_count(&self, composer_id: &str) -> Result<usize> {
        if let Some(connection) = self.connect_global()? {
            let pattern = format!("bubbleId:{composer_id}:%");
            match connection.query_row(
                "SELECT COUNT(*) FROM cursorDiskKV WHERE key LIKE ?1",
                [&pattern],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(count) => Ok(count as usize),
                Err(_e) => {
                    // 如果查询失败，返回0而不是抛出错误
                    Ok(0)
                }
            }
        } else {
            Ok(0)
        }
    }

    /// 获取composer对话的所有bubble数据
    fn get_composer_bubbles(&self, composer_id: &str) -> Result<Vec<ComposerBubble>> {
        if let Some(connection) = self.connect_global()? {
            let pattern = format!("bubbleId:{composer_id}:%");

            let mut stmt = connection.prepare(
                "SELECT rowid, key, value FROM cursorDiskKV WHERE key LIKE ?1 ORDER BY rowid",
            )?;

            let bubble_rows: Vec<(i32, String, Option<Vec<u8>>)> = stmt
                .query_map([&pattern], |row| {
                    Ok((
                        row.get::<_, i32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<Vec<u8>>>(2)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();

            let mut bubbles = Vec::new();

            for (_rowid, _key, value) in bubble_rows {
                if let Ok(json_str) = String::from_utf8(value.unwrap_or_default())
                    && let Ok(bubble) = serde_json::from_str::<ComposerBubble>(&json_str)
                {
                    bubbles.push(bubble);
                }
            }

            Ok(bubbles)
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn write_item_table(db_path: &Path, chat_json: &str) {
        let conn = Connection::open(db_path).expect("open sqlite");
        conn.execute(
            "CREATE TABLE ItemTable (key TEXT PRIMARY KEY NOT NULL, value BLOB);",
            (),
        )
        .expect("create ItemTable");

        conn.execute(
            "INSERT INTO ItemTable (key, value) VALUES (?1, ?2);",
            (
                "workbench.panel.aichat.view.aichat.chatdata",
                chat_json.as_bytes().to_vec(),
            ),
        )
        .expect("insert chatdata");
    }

    #[test]
    fn selected_keys_map_to_correct_export_even_if_summary_is_sorted() {
        let dir = tempdir().expect("tempdir");

        let db_path = dir.path().join("state.vscdb");
        let chat_json = r#"{
  "tabs": [
    {
      "tabId": "tab-old",
      "chatTitle": "Old Chat",
      "lastSendTime": 1700000000000,
      "bubbles": [
        {"type": "user", "id": "b1", "messageType": 1, "text": "hi", "createdAt": 1700000000000}
      ]
    },
    {
      "tabId": "tab-new",
      "chatTitle": "New Chat",
      "lastSendTime": 1800000000000,
      "bubbles": []
    }
  ]
}"#;
        write_item_table(&db_path, chat_json);

        let db_path_str = db_path.to_string_lossy().to_string();
        let db = CursorDatabase::new_without_global(Some(&db_path_str)).expect("db");
        let summaries = db.get_conversation_summaries().expect("summaries");
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].title, "New Chat");

        let selected = vec![summaries[0].key.clone()];
        let exports = db
            .get_conversation_exports_by_keys(&selected)
            .expect("exports");
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].get_title(), "New Chat");
    }

    #[test]
    fn search_results_share_same_key_space_as_exports() {
        let dir = tempdir().expect("tempdir");

        let db_path = dir.path().join("state.vscdb");
        let chat_json = r#"{
  "tabs": [
    {
      "tabId": "tab-old",
      "chatTitle": "Old Chat",
      "lastSendTime": 1700000000000,
      "bubbles": [
        {"type": "user", "id": "b1", "messageType": 1, "text": "hi", "createdAt": 1700000000000}
      ]
    },
    {
      "tabId": "tab-new",
      "chatTitle": "New Chat",
      "lastSendTime": 1800000000000,
      "bubbles": []
    }
  ]
}"#;
        write_item_table(&db_path, chat_json);

        let db_path_str = db_path.to_string_lossy().to_string();
        let db = CursorDatabase::new_without_global(Some(&db_path_str)).expect("db");
        let results = db.search_conversations("old").expect("search");
        assert_eq!(results.len(), 1);
        let key = results[0].conversation_key();

        let exports = db
            .get_conversation_exports_by_keys(&[key])
            .expect("exports");
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].get_title(), "Old Chat");
    }
}
