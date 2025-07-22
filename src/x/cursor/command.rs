use crate::x::cursor::database::CursorDatabase;
use crate::x::cursor::models::{ConversationExport, ConversationSummary, ConversationType};
use anyhow::{Result, anyhow};
use chrono::Utc;
use clap::{Args, Subcommand};
use inquire::{MultiSelect, Select, Text};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Commands for interacting with Cursor"
)]
pub struct CursorArgs {
    #[command(subcommand)]
    pub command: CursorCommands,
}

#[derive(Subcommand)]
pub enum CursorCommands {
    /// Export conversations from Cursor
    Export(ExportArgs),
}

#[derive(Args)]
pub struct ExportArgs {
    /// Use interactive mode
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub interactive: bool,

    /// Path to the Cursor database file
    #[arg(long)]
    pub db_path: Option<PathBuf>,

    /// Directory of the workspace to export from
    #[arg(long)]
    pub workspace_dir: Option<PathBuf>,

    /// ID of the composer to export
    #[arg(long)]
    pub composer_id: Option<String>,

    /// Output mode
    #[arg(long, value_parser = ["console", "file", "single-file"], default_value = "console")]
    pub output_mode: String,

    /// Output file name
    #[arg(long)]
    pub output_file: Option<String>,

    /// Enable debug mode
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub debug: bool,
}

pub fn run(args: &CursorArgs) -> Result<()> {
    match &args.command {
        CursorCommands::Export(export_args) => {
            if export_args.interactive
                || export_args.composer_id.is_none() && export_args.workspace_dir.is_none()
            {
                export_interactive_with_path(
                    export_args.db_path.as_deref().and_then(|p| p.to_str()),
                )
            } else {
                export_non_interactive(export_args)
            }
        }
    }
}

fn export_non_interactive(args: &ExportArgs) -> Result<()> {
    if args.debug {
        println!("[DEBUG] 非交互式导出模式");
        println!("[DEBUG] workspace_dir: {:?}", args.workspace_dir);
        println!("[DEBUG] composer_id: {:?}", args.composer_id);
        println!("[DEBUG] output_mode: {:?}", args.output_mode);
        println!("[DEBUG] output_file: {:?}", args.output_file);
    }

    // 暂时使用当前工作区的全局数据库
    let db = CursorDatabase::new(None)?;

    if let Some(id) = &args.composer_id {
        // 直接导出指定的composer
        return export_composer_by_id(
            &db,
            id,
            &args.output_mode,
            args.output_file.as_deref(),
            args.debug,
        );
    }

    // 如果指定了workspace_dir，实现工作区选择逻辑
    // 目前先用默认逻辑
    export_interactive_with_path(None)
}

fn export_composer_by_id(
    db: &CursorDatabase,
    composer_id: &str,
    output_mode: &str,
    output_file: Option<&str>,
    debug: bool,
) -> Result<()> {
    if debug {
        println!("[DEBUG] 正在导出composer: {}", composer_id);
    }
    let all_conversations = db.get_all_conversations_mixed()?;

    let target_conversation = all_conversations.iter().find(|c| {
        if let ConversationExport::Composer(composer) = c {
            composer.composer_data.composer_id == composer_id
        } else {
            false
        }
    });

    if let Some(conversation) = target_conversation {
        let conversations = vec![conversation];
        match output_mode {
            "console" => export_to_console(&conversations),
            "single-file" => {
                let filename = output_file.unwrap_or("cursor_export.md");
                if debug {
                    println!("[DEBUG] 导出到文件: {}", filename);
                }
                export_to_single_file_with_name(&conversations, filename)?;
            }
            _ => println!("不支持的输出模式: {}", output_mode),
        }
    } else {
        println!("未找到composer ID: {}", composer_id);
    }

    Ok(())
}

fn export_interactive_with_path(db_path: Option<&str>) -> Result<()> {
    println!("{}", t!("cursor.export.title"));

    let selected_db_path = if let Some(path) = db_path {
        path.to_string()
    } else {
        select_workspace()?
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

    let selected_conversations = select_conversations(&summaries, &db)?;

    if selected_conversations.is_empty() {
        println!("{}", t!("cursor.export.no_selection"));
        return Ok(());
    }

    let all_conversations = db.get_all_conversations_mixed()?;
    let selected_exports: Vec<&ConversationExport> = all_conversations
        .iter()
        .enumerate()
        .filter(|(i, _)| selected_conversations.contains(i))
        .map(|(_, export)| export)
        .collect();

    handle_export(&selected_exports)?;
    Ok(())
}

fn select_workspace() -> Result<String> {
    println!("{}", t!("cursor.workspace.scanning_workspaces"));
    let workspaces = CursorDatabase::find_all_workspaces()?;
    if workspaces.is_empty() {
        return Err(anyhow!(t!("cursor.workspace.no_workspaces")));
    }

    let workspaces_with_chat: Vec<_> = workspaces.iter().filter(|w| w.has_chat_data).collect();
    let target_workspaces = if workspaces_with_chat.is_empty() {
        workspaces.iter().collect()
    } else {
        workspaces_with_chat
    };

    if target_workspaces.len() == 1 {
        let workspace = target_workspaces[0];
        println!(
            "{}: {}",
            t!("cursor.workspace.auto_select"),
            workspace.display_name()
        );
        return Ok(workspace.db_path.to_string_lossy().to_string());
    }

    let options: Vec<_> = target_workspaces.iter().map(|w| w.display_name()).collect();
    let selected = Select::new(&t!("cursor.workspace.select_workspace"), options).prompt()?;

    target_workspaces
        .iter()
        .find(|w| w.display_name() == selected)
        .map(|w| w.db_path.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("Selected workspace not found"))
}

fn select_conversations(
    summaries: &[ConversationSummary],
    db: &CursorDatabase,
) -> Result<Vec<usize>> {
    let mut options = Vec::new();
    let recent_count = std::cmp::min(5, summaries.len());
    for summary in summaries.iter().take(recent_count) {
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
    if summaries.len() > recent_count {
        options.push(t!("cursor.export.search_more").to_string());
    }

    let selections = MultiSelect::new(&t!("cursor.export.select_conversations"), options)
        .with_help_message(&t!("cursor.export.select_help"))
        .prompt()?;

    let mut selected_indices = Vec::new();
    for selection in selections {
        if selection == t!("cursor.export.search_more") {
            selected_indices.extend(search_conversations(db)?);
        } else if let Some(index) = summaries.iter().position(|summary| {
            let type_indicator = match &summary.conversation_type {
                ConversationType::Traditional => "(chat)",
                ConversationType::Composer(_) => "(composer)",
            };
            let option_text = format!(
                "📝 {} {} - {} - {}",
                summary.title,
                type_indicator,
                if summary.message_count > 0 {
                    t!("cursor.export.message_count", count = summary.message_count).to_string()
                } else {
                    t!("cursor.export.composer_chat").to_string()
                },
                summary.last_message_time.format("%m-%d %H:%M")
            );
            option_text == selection
        }) {
            selected_indices.push(index);
        }
    }
    Ok(selected_indices)
}

fn search_conversations(db: &CursorDatabase) -> Result<Vec<usize>> {
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

    let options: Vec<_> = search_results
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
    let selected =
        MultiSelect::new(&t!("cursor.export.select_search_results"), options.clone()).prompt()?;

    let selected_indices = selected
        .iter()
        .filter_map(|s| options.iter().position(|opt| opt == s))
        .collect();
    Ok(selected_indices)
}

fn handle_export(conversations: &[&ConversationExport]) -> Result<()> {
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
        x if x == t!("cursor.export.export_to_console") => export_to_console(conversations),
        x if x == t!("cursor.export.export_to_files") => export_to_files(conversations)?,
        x if x == t!("cursor.export.export_to_single_file") => {
            export_to_single_file(conversations)?
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn export_to_console(conversations: &[&ConversationExport]) {
    println!("\n============================================================");
    println!("{}", t!("cursor.export.exported_content_title"));
    println!("============================================================\n");
    for (i, conversation) in conversations.iter().enumerate() {
        println!("{}", conversation.to_markdown());
        if i < conversations.len() - 1 {
            println!("\n----------------------------------------\n");
        }
    }
}

fn export_to_files(conversations: &[&ConversationExport]) -> Result<()> {
    let output_dir = Text::new(&t!("cursor.export.input_output_dir"))
        .with_default("./cursor_exports")
        .prompt()?;
    let output_path = PathBuf::from(output_dir);
    if !output_path.exists() {
        fs::create_dir_all(&output_path)?;
    }

    for (i, conversation) in conversations.iter().enumerate() {
        let filename = format!(
            "{:02}_{}.md",
            i + 1,
            sanitize_filename(&conversation.get_title())
        );
        let file_path = output_path.join(&filename);
        fs::write(&file_path, conversation.to_markdown())?;
        println!(
            "{}",
            t!(
                "cursor.export.export_success_file",
                path = file_path.display()
            )
        );
    }
    Ok(())
}

fn export_to_single_file(conversations: &[&ConversationExport]) -> Result<()> {
    let filename = Text::new(&t!("cursor.export.input_filename"))
        .with_default("cursor_conversations.md")
        .prompt()?;
    let mut content = String::new();
    content.push_str(&format!("# {}\n\n", t!("cursor.export.single_file_title")));
    content.push_str(&format!(
        "**{}**: {}\n",
        t!("cursor.export.export_time_label"),
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    content.push_str(&format!(
        "**{}**: {}\n\n",
        t!("cursor.export.conversation_count_label"),
        conversations.len()
    ));
    content.push_str("---\n\n");

    for conversation in conversations {
        content.push_str(&conversation.to_markdown());
        content.push_str("\n\n---\n\n");
    }

    fs::write(&filename, content)?;
    println!(
        "{}",
        t!("cursor.export.export_success_single", filename = filename)
    );
    Ok(())
}

fn export_to_single_file_with_name(
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

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_' || *c == '-')
        .collect::<String>()
        .trim()
        .replace(' ', "_")
}
