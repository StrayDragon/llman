use crate::error::Result;
use crate::x::cursor::database::CursorDatabase;
use crate::x::cursor::models::{
    ConversationExport, ConversationSummary, ConversationType, WorkspaceInfo,
};
use chrono::Utc;
use inquire::{MultiSelect, Select, Text};
use std::fs;
use std::path::PathBuf;

/// Cursor命令处理器
pub struct CursorCommand;

impl CursorCommand {
    /// 创建新的命令处理器
    pub fn new(_config_dir: Option<&str>) -> Self {
        Self
    }

    /// 非交互式导出对话
    ///
    /// # 参数
    /// * `workspace_dir` - 可选的工作区目录
    /// * `composer_id` - 可选的composer ID
    /// * `output_mode` - 输出模式
    /// * `output_file` - 输出文件名
    /// * `debug` - 是否开启调试模式
    pub fn export_non_interactive(
        &self,
        workspace_dir: Option<&str>,
        composer_id: Option<&str>,
        output_mode: Option<&str>,
        output_file: Option<&str>,
        debug: bool,
    ) -> Result<()> {
        if debug {
            println!("[DEBUG] 非交互式导出模式");
            println!("[DEBUG] workspace_dir: {:?}", workspace_dir);
            println!("[DEBUG] composer_id: {:?}", composer_id);
            println!("[DEBUG] output_mode: {:?}", output_mode);
            println!("[DEBUG] output_file: {:?}", output_file);
        }

        // 暂时使用当前工作区的全局数据库
        let db = CursorDatabase::new(None)?;

        if let Some(id) = composer_id {
            // 直接导出指定的composer
            return self.export_composer_by_id(&db, id, output_mode, output_file, debug);
        }

        // 如果指定了workspace_dir，实现工作区选择逻辑
        // 目前先用默认逻辑
        self.export_interactive_with_path(None)
    }

    /// 根据composer ID导出
    fn export_composer_by_id(
        &self,
        db: &CursorDatabase,
        composer_id: &str,
        output_mode: Option<&str>,
        output_file: Option<&str>,
        debug: bool,
    ) -> Result<()> {
        if debug {
            println!("[DEBUG] 正在导出composer: {}", composer_id);
        }
        // let summaries = db.get_conversation_summaries()?;
        let all_conversations = db.get_all_conversations_mixed()?;

        // 找到匹配的composer对话
        let mut target_conversation = None;
        for (i, conversation) in all_conversations.iter().enumerate() {
            if let ConversationExport::Composer(composer_with_bubbles) = conversation {
                if composer_with_bubbles.composer_data.composer_id == composer_id {
                    target_conversation = Some(conversation);
                    if debug {
                        println!("[DEBUG] 找到目标composer，索引: {}", i);
                        println!(
                            "[DEBUG] Composer名称: {}",
                            composer_with_bubbles.composer_data.get_title()
                        );
                        println!(
                            "[DEBUG] Bubble数量: {}",
                            composer_with_bubbles.bubbles.len()
                        );
                    }
                    break;
                }
            }
        }

        if let Some(conversation) = target_conversation {
            let conversations = vec![conversation];

            // 根据output_mode决定输出方式
            match output_mode.unwrap_or("console") {
                "console" => {
                    self.export_to_console(&conversations);
                }
                "single-file" => {
                    let filename = output_file.unwrap_or("cursor_export.md");
                    if debug {
                        println!("[DEBUG] 导出到文件: {}", filename);
                    }
                    self.export_to_single_file_with_name(&conversations, filename)?;
                }
                _ => {
                    println!("不支持的输出模式: {}", output_mode.unwrap_or("console"));
                }
            }
        } else {
            println!("未找到composer ID: {}", composer_id);
        }

        Ok(())
    }

    /// 导出到指定文件名的单个文件
    fn export_to_single_file_with_name(
        &self,
        conversations: &[&ConversationExport],
        filename: &str,
    ) -> Result<()> {
        let mut content = String::new();

        for conversation in conversations {
            content.push_str(&conversation.to_markdown());
            content.push_str("\n\n");
        }

        fs::write(filename, content)?;
        println!("✅ 导出成功: {}", filename);

        Ok(())
    }

    /// 交互式导出对话
    ///
    /// # 参数
    /// * `db_path` - 可选的数据库路径，None时会进行workspace选择
    pub fn export_interactive_with_path(&self, db_path: Option<&str>) -> Result<()> {
        println!("{}", t!("cursor.export.title"));

        // 选择workspace（如果未指定路径）
        let selected_db_path = if let Some(path) = db_path {
            path.to_string()
        } else {
            self.select_workspace()?
        };

        println!("{}\n", t!("cursor.export.scanning"));

        let db = CursorDatabase::new(Some(&selected_db_path))?;
        let summaries = db.get_conversation_summaries()?;

        if summaries.is_empty() {
            println!("{}", t!("cursor.export.no_conversations"));
            return Ok(());
        }

        println!(
            "{}",
            t!("cursor.export.found_conversations", count = summaries.len())
        );

        // 选择要导出的对话
        let selected_conversations = self.select_conversations(&summaries, &db)?;

        if selected_conversations.is_empty() {
            println!("{}", t!("cursor.export.no_selection"));
            return Ok(());
        }

        // 获取对话数据并导出
        let all_conversations = db.get_all_conversations_mixed()?;
        let selected_exports: Vec<&ConversationExport> = all_conversations
            .iter()
            .enumerate()
            .filter(|(i, _)| selected_conversations.contains(i))
            .map(|(_, export)| export)
            .collect();

        self.handle_export(&selected_exports)?;
        Ok(())
    }

    // ====== 私有方法 ======

    /// 选择workspace
    fn select_workspace(&self) -> Result<String> {
        println!("{}", t!("cursor.workspace.scanning_workspaces"));

        let workspaces = CursorDatabase::find_all_workspaces()?;

        if workspaces.is_empty() {
            return Err(crate::error::LlmanError::Custom(
                t!("cursor.workspace.no_workspaces").to_string(),
            ));
        }

        // 过滤出包含聊天数据的workspace
        let workspaces_with_chat: Vec<&WorkspaceInfo> =
            workspaces.iter().filter(|w| w.has_chat_data).collect();

        let target_workspaces = if workspaces_with_chat.is_empty() {
            workspaces.iter().collect()
        } else {
            workspaces_with_chat
        };

        println!(
            "{}",
            t!(
                "cursor.workspace.found_workspaces",
                count = target_workspaces.len()
            )
        );

        // 如果只有一个workspace，自动选择
        if target_workspaces.len() == 1 {
            let workspace = target_workspaces[0];
            println!(
                "{}: {}",
                t!("cursor.workspace.auto_select"),
                workspace.display_name()
            );
            return Ok(workspace.db_path.to_string_lossy().to_string());
        }

        // 多个workspace时进行选择
        let options: Vec<String> = target_workspaces.iter().map(|w| w.display_name()).collect();

        let selected = Select::new(&t!("cursor.workspace.select_workspace"), options).prompt()?;

        // 找到对应的workspace
        for workspace in target_workspaces {
            if workspace.display_name() == selected {
                return Ok(workspace.db_path.to_string_lossy().to_string());
            }
        }

        unreachable!("Selected workspace should always be found")
    }

    /// 选择要导出的对话
    fn select_conversations(
        &self,
        summaries: &[ConversationSummary],
        db: &CursorDatabase,
    ) -> Result<Vec<usize>> {
        let mut options = Vec::new();

        // 显示最近的对话
        let recent_count = std::cmp::min(5, summaries.len());
        for summary in summaries.iter().take(recent_count) {
            // 根据对话类型生成更清晰的标识
            let type_indicator = match &summary.conversation_type {
                ConversationType::Traditional => "(chat)",
                ConversationType::Composer(_) => "(composer)",
            };

            options.push(format!(
                "📝 {} {} - {} - {}",
                summary.title,
                type_indicator,
                if summary.message_count > 0 {
                    t!("cursor.export.message_count", count = summary.message_count).to_string()
                } else {
                    t!("cursor.export.composer_chat").to_string()
                },
                summary.last_message_time.format("%m-%d %H:%M")
            ));
        }

        // 添加搜索选项
        if summaries.len() > recent_count {
            options.push(t!("cursor.export.search_more").to_string());
        }

        let selected = MultiSelect::new(&t!("cursor.export.select_conversations"), options)
            .with_help_message(&t!("cursor.export.select_help"))
            .prompt()?;

        let mut selected_indices = Vec::new();

        for selection in selected {
            if selection == t!("cursor.export.search_more") {
                // 进入搜索模式
                let search_results = self.search_conversations(db)?;
                selected_indices.extend(search_results);
            } else {
                // 找到对应的索引
                for (i, summary) in summaries.iter().enumerate() {
                    let type_indicator = match &summary.conversation_type {
                        ConversationType::Traditional => "(chat)",
                        ConversationType::Composer(_) => "(composer)",
                    };

                    let option_text = format!(
                        "📝 {} {} - {} - {}",
                        summary.title,
                        type_indicator,
                        if summary.message_count > 0 {
                            t!("cursor.export.message_count", count = summary.message_count)
                                .to_string()
                        } else {
                            t!("cursor.export.composer_chat").to_string()
                        },
                        summary.last_message_time.format("%m-%d %H:%M")
                    );
                    if selection == option_text {
                        selected_indices.push(i);
                        break;
                    }
                }
            }
        }

        Ok(selected_indices)
    }

    /// 搜索对话
    fn search_conversations(&self, db: &CursorDatabase) -> Result<Vec<usize>> {
        let search_text = Text::new(&t!("cursor.export.search_keyword"))
            .with_help_message(&t!("cursor.export.search_help"))
            .prompt()?;

        if search_text.trim().is_empty() {
            return Ok(vec![]);
        }

        let search_results = db.search_conversations(&search_text)?;

        if search_results.is_empty() {
            println!("{}", t!("cursor.export.search_no_results"));
            return Ok(vec![]);
        }

        println!(
            "{}",
            t!("cursor.export.search_found", count = search_results.len())
        );

        // 创建搜索结果选项
        let options: Vec<String> = search_results
            .iter()
            .map(|tab| {
                format!(
                    "📝 {} ({}) - {}",
                    tab.get_title(),
                    t!("cursor.export.message_count", count = tab.bubbles.len()),
                    tab.get_last_send_time().format("%m-%d %H:%M")
                )
            })
            .collect();

        let selected = MultiSelect::new(&t!("cursor.export.select_search_results"), options)
            .with_help_message(&t!("cursor.export.select_help"))
            .prompt()?;

        // 返回搜索结果的索引
        let selected_indices: Vec<usize> = (0..selected.len()).collect();
        Ok(selected_indices)
    }

    /// 处理导出
    fn handle_export(&self, conversations: &[&ConversationExport]) -> Result<()> {
        let export_option = Select::new(
            &t!("cursor.export.select_export_method"),
            vec![
                t!("cursor.export.export_to_console").to_string(),
                t!("cursor.export.export_to_files").to_string(),
                t!("cursor.export.export_to_single_file").to_string(),
            ],
        )
        .prompt()?;

        match export_option.as_str() {
            x if x == t!("cursor.export.export_to_console") => {
                self.export_to_console(conversations)
            }
            x if x == t!("cursor.export.export_to_files") => self.export_to_files(conversations)?,
            x if x == t!("cursor.export.export_to_single_file") => {
                self.export_to_single_file(conversations)?
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    /// 导出到控制台
    fn export_to_console(&self, conversations: &[&ConversationExport]) {
        println!("{}", "\n".to_string() + &"=".repeat(60));
        println!("{}", t!("cursor.export.exported_content_title"));
        println!("{}", "=".repeat(60));

        for (i, conversation) in conversations.iter().enumerate() {
            println!("\n{}", conversation.to_markdown());

            if i < conversations.len() - 1 {
                println!("{}", "\n".to_string() + &"-".repeat(40));
            }
        }
    }

    /// 导出到多个文件
    fn export_to_files(&self, conversations: &[&ConversationExport]) -> Result<()> {
        let output_dir = Text::new(&t!("cursor.export.input_output_dir"))
            .with_default("./cursor_exports")
            .prompt()?;

        let output_path = PathBuf::from(output_dir);

        if !output_path.exists() {
            fs::create_dir_all(&output_path)?;
            println!(
                "{}",
                t!(
                    "cursor.export.output_dir_created",
                    path = output_path.display()
                )
            );
        }

        for (i, conversation) in conversations.iter().enumerate() {
            let filename = format!(
                "{:02}_{}.md",
                i + 1,
                sanitize_filename(&conversation.get_title())
            );

            let file_path = output_path.join(filename);
            fs::write(&file_path, conversation.to_markdown())?;

            println!(
                "{}",
                t!(
                    "cursor.export.export_success_file",
                    path = file_path.display()
                )
            );
        }

        println!(
            "{}",
            t!(
                "cursor.export.export_success_summary",
                count = conversations.len(),
                path = output_path.display()
            )
        );

        Ok(())
    }

    /// 导出到单个文件
    fn export_to_single_file(&self, conversations: &[&ConversationExport]) -> Result<()> {
        let filename = Text::new(&t!("cursor.export.input_filename"))
            .with_default("cursor_conversations.md")
            .prompt()?;

        let mut content = String::new();
        content.push_str(&t!("cursor.export.single_file_title"));
        content.push_str(&t!(
            "cursor.export.export_time",
            time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        content.push_str(&t!(
            "cursor.export.conversation_count",
            count = conversations.len()
        ));
        content.push_str("---\n\n");

        for conversation in conversations {
            content.push_str(&conversation.to_markdown());
        }

        fs::write(&filename, content)?;
        println!(
            "{}",
            t!("cursor.export.export_success_single", filename = filename)
        );

        Ok(())
    }
}

/// 清理文件名中的非法字符
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
