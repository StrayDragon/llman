use crate::error::Result;
use crate::x::cursor::models::*;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use dirs;
use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};

// 定义数据库表结构
diesel::table! {
    #[sql_name = "ItemTable"]
    item_table (key) {
        key -> Text,
        value -> Binary,
    }
}

// 定义全局数据库的cursorDiskKV表结构
diesel::table! {
    #[sql_name = "cursorDiskKV"]
    cursor_disk_kv (key) {
        key -> Text,
        value -> Binary,
    }
}

// 用于接收sql_query结果的结构体
#[derive(QueryableByName, Debug)]
pub struct BubbleRowData {
    #[allow(dead_code)]
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub rowid: i32,
    #[allow(dead_code)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub key: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Binary>)]
    pub value: Option<Vec<u8>>,
}

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
        let db_path = if let Some(path) = custom_path {
            PathBuf::from(path)
        } else {
            Self::find_latest_workspace_db()?
        };

        // 获取全局数据库路径
        let global_db_path = Self::get_global_db_path().ok();

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
    pub fn connect(&self) -> Result<SqliteConnection> {
        let database_url = format!("file:{}?mode=ro", self.db_path.display());
        Ok(SqliteConnection::establish(&database_url)?)
    }

    /// 连接到全局数据库
    pub fn connect_global(&self) -> Result<Option<SqliteConnection>> {
        if let Some(ref global_path) = self.global_db_path {
            let database_url = format!("file:{}?mode=ro", global_path.display());
            Ok(Some(SqliteConnection::establish(&database_url)?))
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
                    title: composer.get_title(),
                    last_message_time: composer.get_last_updated_time(),
                    message_count: bubble_count,
                    conversation_type: ConversationType::Composer(composer.get_composer_mode()),
                });
            }
        }

        // 按时间排序，最新的在前面
        summaries.sort_by(|a, b| b.last_message_time.cmp(&a.last_message_time));

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

        db_files_with_metadata.sort_by(|a, b| b.1.cmp(&a.1));

        let chat_db = db_files_with_metadata
            .iter()
            .find(|(path, _)| Self::has_chat_data(path).unwrap_or(false))
            .map(|(path, _)| path.clone());

        Ok(chat_db.unwrap_or_else(|| db_files_with_metadata[0].0.clone()))
    }

    /// 检查数据库是否包含聊天数据
    fn has_chat_data(db_path: &Path) -> Result<bool> {
        let database_url = format!("file:{}?mode=ro", db_path.display());
        let mut connection = match SqliteConnection::establish(&database_url) {
            Ok(conn) => conn,
            Err(_) => return Ok(false),
        };

        use crate::x::cursor::database::item_table::dsl::*;

        let traditional_chat = item_table
            .select(value)
            .filter(key.eq("workbench.panel.aichat.view.aichat.chatdata"))
            .first::<Vec<u8>>(&mut connection)
            .optional()
            .unwrap_or(None)
            .and_then(|bytes| String::from_utf8(bytes).ok());

        let composer_chat = item_table
            .select(value)
            .filter(key.eq("composer.composerData"))
            .first::<Vec<u8>>(&mut connection)
            .optional()
            .unwrap_or(None)
            .and_then(|bytes| String::from_utf8(bytes).ok());

        let has_traditional = traditional_chat
            .and_then(|json_str| serde_json::from_str::<ChatData>(&json_str).ok())
            .is_some_and(|data| !data.tabs.is_empty());

        let has_composer = composer_chat
            .and_then(|json_str| serde_json::from_str::<ComposerData>(&json_str).ok())
            .is_some_and(|data| !data.all_composers.is_empty());

        Ok(has_traditional || has_composer)
    }

    /// 获取Cursor工作区路径
    fn get_cursor_workspace_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
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
        let mut connection = self.connect()?;

        use crate::x::cursor::database::item_table::dsl::*;

        let chat_json = item_table
            .select(value)
            .filter(key.eq("workbench.panel.aichat.view.aichat.chatdata"))
            .first::<Vec<u8>>(&mut connection)
            .optional()?
            .and_then(|bytes| String::from_utf8(bytes).ok());

        if let Some(json_str) = chat_json {
            let data: ChatData = serde_json::from_str(&json_str)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// 获取Composer数据
    fn get_composer_data(&self) -> Result<Option<ComposerData>> {
        let mut connection = self.connect()?;

        use crate::x::cursor::database::item_table::dsl::*;

        let composer_json = item_table
            .select(value)
            .filter(key.eq("composer.composerData"))
            .first::<Vec<u8>>(&mut connection)
            .optional()?
            .and_then(|bytes| String::from_utf8(bytes).ok());

        if let Some(json_str) = composer_json {
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
                .unwrap_or("Unknown")
                .to_string();
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
    fn get_global_db_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
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
        if let Some(mut connection) = self.connect_global()? {
            use crate::x::cursor::database::cursor_disk_kv::dsl::*;

            let pattern = format!("bubbleId:{composer_id}:%");
            match cursor_disk_kv
                .filter(key.like(pattern))
                .count()
                .get_result::<i64>(&mut connection)
            {
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
        if let Some(mut connection) = self.connect_global()? {
            let pattern = format!("bubbleId:{composer_id}:%");

            // 使用diesel的sql_query功能来查询rowid
            let query =
                "SELECT rowid, key, value FROM cursorDiskKV WHERE key LIKE ?1 ORDER BY rowid";
            let bubble_rows: Vec<BubbleRowData> = sql_query(query)
                .bind::<diesel::sql_types::Text, _>(&pattern)
                .load(&mut connection)?;

            let mut bubbles = Vec::new();

            for row in bubble_rows {
                if let Ok(json_str) = String::from_utf8(row.value.unwrap_or_default())
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
