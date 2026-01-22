use crate::path_utils::validate_path_str;
use crate::x::cursor::database::CursorDatabase;
use crate::x::cursor::models::{ConversationExport, ConversationSummary, ConversationType};
use anyhow::{Result, anyhow};
use chrono::Utc;
use clap::{Args, Subcommand};
use inquire::{MultiSelect, Select, Text};
use std::fs;
use std::path::{Path, PathBuf};

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
            if export_args.interactive {
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
        println!("{}", t!("cursor.export.debug_non_interactive"));
        println!(
            "{}",
            t!(
                "cursor.export.debug_workspace_dir",
                path = display_option_path(&args.workspace_dir)
            )
        );
        println!(
            "{}",
            t!(
                "cursor.export.debug_composer_id",
                id = display_option_string(&args.composer_id)
            )
        );
        println!(
            "{}",
            t!(
                "cursor.export.debug_output_mode",
                mode = args.output_mode.as_str()
            )
        );
        println!(
            "{}",
            t!(
                "cursor.export.debug_output_file",
                file = display_option_string(&args.output_file)
            )
        );
    }

    let db = resolve_database(args)?;

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

    let all_conversations = db.get_all_conversations_mixed()?;
    if all_conversations.is_empty() {
        println!("{}", t!("cursor.export.no_conversations"));
        return Ok(());
    }

    let selected_exports: Vec<&ConversationExport> = all_conversations.iter().collect();
    export_by_mode(
        &selected_exports,
        &args.output_mode,
        args.output_file.as_deref(),
        args.debug,
    )
}

fn export_composer_by_id(
    db: &CursorDatabase,
    composer_id: &str,
    output_mode: &str,
    output_file: Option<&str>,
    debug: bool,
) -> Result<()> {
    if debug {
        println!(
            "{}",
            t!("cursor.export.debug_exporting_composer", id = composer_id)
        );
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
        export_by_mode(&conversations, output_mode, output_file, debug)?;
    } else {
        return Err(anyhow!(t!(
            "cursor.export.composer_not_found",
            id = composer_id
        )));
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
        .ok_or_else(|| anyhow!(t!("cursor.workspace.selected_not_found")))
}

fn select_conversations(
    summaries: &[ConversationSummary],
    db: &CursorDatabase,
) -> Result<Vec<usize>> {
    let mut options = Vec::new();
    let recent_count = std::cmp::min(5, summaries.len());
    for summary in summaries.iter().take(recent_count) {
        let type_indicator = match &summary.conversation_type {
            ConversationType::Traditional => t!("cursor.export.conversation_type_chat"),
            ConversationType::Composer(_) => t!("cursor.export.conversation_type_composer"),
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
                ConversationType::Traditional => t!("cursor.export.conversation_type_chat"),
                ConversationType::Composer(_) => t!("cursor.export.conversation_type_composer"),
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

fn export_by_mode(
    conversations: &[&ConversationExport],
    output_mode: &str,
    output_file: Option<&str>,
    debug: bool,
) -> Result<()> {
    match output_mode {
        "console" => {
            export_to_console(conversations);
            Ok(())
        }
        "file" => {
            let output_dir = resolve_output_dir(output_file)?;
            export_to_files_with_dir(conversations, &output_dir)
        }
        "single-file" => {
            let filename = output_file.unwrap_or("cursor_conversations.md");
            if debug {
                println!(
                    "{}",
                    t!("cursor.export.debug_exporting_file", filename = filename)
                );
            }
            export_to_single_file_at(conversations, filename)
        }
        _ => Err(anyhow!(t!(
            "cursor.export.unsupported_output_mode",
            mode = output_mode
        ))),
    }
}

fn export_to_console(conversations: &[&ConversationExport]) {
    println!(
        "\n{}",
        t!("cursor.export.console_separator_char").repeat(60)
    );
    println!("{}", t!("cursor.export.exported_content_title"));
    println!(
        "{}\n",
        t!("cursor.export.console_separator_char").repeat(60)
    );
    for (i, conversation) in conversations.iter().enumerate() {
        println!("{}", conversation.to_markdown());
        if i < conversations.len() - 1 {
            println!(
                "\n{}\n",
                t!("cursor.export.console_item_separator_char").repeat(40)
            );
        }
    }
}

fn export_to_files(conversations: &[&ConversationExport]) -> Result<()> {
    let output_dir = Text::new(&t!("cursor.export.input_output_dir"))
        .with_default("./cursor_exports")
        .prompt()?;

    let output_path = resolve_output_dir(Some(&output_dir))?;
    export_to_files_with_dir(conversations, &output_path)
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

    export_to_single_file_with_content(&content, &filename)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_' || *c == '-')
        .collect::<String>()
        .trim()
        .replace(' ', "_")
}

fn export_to_files_with_dir(
    conversations: &[&ConversationExport],
    output_dir: &Path,
) -> Result<()> {
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    for (i, conversation) in conversations.iter().enumerate() {
        let filename = format!(
            "{:02}_{}.md",
            i + 1,
            sanitize_filename(&conversation.get_title())
        );
        let file_path = output_dir.join(&filename);
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

fn export_to_single_file_at(conversations: &[&ConversationExport], filename: &str) -> Result<()> {
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

    export_to_single_file_with_content(&content, filename)
}

fn export_to_single_file_with_content(content: &str, filename: &str) -> Result<()> {
    fs::write(filename, content)?;
    println!(
        "{}",
        t!("cursor.export.export_success_single", filename = filename)
    );
    Ok(())
}

fn resolve_output_dir(output_dir: Option<&str>) -> Result<PathBuf> {
    let output_dir = output_dir.unwrap_or("./cursor_exports");
    validate_path_str(output_dir).map_err(|e| {
        anyhow!(t!(
            "cursor.export.invalid_output_dir",
            path = output_dir,
            error = e
        ))
    })?;
    Ok(PathBuf::from(output_dir))
}

fn resolve_database(args: &ExportArgs) -> Result<CursorDatabase> {
    if let Some(db_path) = args.db_path.as_deref() {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow!(t!("cursor.export.invalid_db_path")))?;
        return Ok(CursorDatabase::new(Some(db_path_str))?);
    }

    if let Some(workspace_dir) = args.workspace_dir.as_deref() {
        let db_path = resolve_workspace_db_path(workspace_dir)?;
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow!(t!("cursor.export.invalid_workspace_db_path")))?;
        return Ok(CursorDatabase::new(Some(db_path_str))?);
    }

    Ok(CursorDatabase::new(None)?)
}

fn resolve_workspace_db_path(workspace_dir: &Path) -> Result<PathBuf> {
    if !workspace_dir.exists() {
        return Err(anyhow!(t!(
            "cursor.export.workspace_dir_not_exist",
            path = workspace_dir.display()
        )));
    }
    let workspaces = CursorDatabase::find_all_workspaces()?;
    find_workspace_db_path(workspace_dir, &workspaces).ok_or_else(|| {
        anyhow!(t!(
            "cursor.export.workspace_not_found",
            path = workspace_dir.display()
        ))
    })
}

fn find_workspace_db_path(
    workspace_dir: &Path,
    workspaces: &[crate::x::cursor::models::WorkspaceInfo],
) -> Option<PathBuf> {
    let target = normalize_path(workspace_dir);
    for workspace in workspaces {
        let project_path = workspace.project_path.as_ref()?;
        if normalize_path(project_path) == target {
            return Some(workspace.db_path.clone());
        }
    }
    None
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn display_option_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|value| value.display().to_string())
        .unwrap_or_else(|| t!("cursor.export.none").to_string())
}

fn display_option_string(value: &Option<String>) -> String {
    value
        .as_ref()
        .cloned()
        .unwrap_or_else(|| t!("cursor.export.none").to_string())
}

#[cfg(test)]
mod tests {
    use super::{find_workspace_db_path, resolve_output_dir};
    use crate::x::cursor::models::WorkspaceInfo;
    use tempfile::TempDir;

    #[test]
    fn test_find_workspace_db_path_matches_project_path() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();
        let db_path = temp.path().join("state.vscdb");

        let workspaces = vec![WorkspaceInfo {
            db_path: db_path.clone(),
            project_path: Some(project_dir.clone()),
            project_name: "project".to_string(),
            has_chat_data: false,
        }];

        let resolved = find_workspace_db_path(&project_dir, &workspaces).unwrap();
        assert_eq!(resolved, db_path);
    }

    #[test]
    fn test_find_workspace_db_path_missing_returns_none() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("project");
        let other_dir = temp.path().join("other");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::create_dir_all(&other_dir).unwrap();
        let db_path = temp.path().join("state.vscdb");

        let workspaces = vec![WorkspaceInfo {
            db_path,
            project_path: Some(project_dir),
            project_name: "project".to_string(),
            has_chat_data: false,
        }];

        let resolved = find_workspace_db_path(&other_dir, &workspaces);
        assert!(resolved.is_none());
    }

    #[test]
    fn test_resolve_output_dir_defaults() {
        let resolved = resolve_output_dir(None).unwrap();
        assert_eq!(resolved, std::path::PathBuf::from("./cursor_exports"));
    }
}
