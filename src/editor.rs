use crate::arg_utils::{SplitShellArgsError, split_shell_args};

pub fn select_editor_raw() -> String {
    let visual = std::env::var("VISUAL").ok();
    let editor = std::env::var("EDITOR").ok();
    select_editor_from_env(visual.as_deref(), editor.as_deref())
}

pub fn select_editor_from_env(visual: Option<&str>, editor: Option<&str>) -> String {
    visual
        .filter(|value| !value.trim().is_empty())
        .or_else(|| editor.filter(|value| !value.trim().is_empty()))
        .unwrap_or("vi")
        .to_string()
}

pub fn parse_editor_command(raw: &str) -> Result<(String, Vec<String>), SplitShellArgsError> {
    let parts = split_shell_args(raw)?;
    match parts.split_first() {
        Some((cmd, args)) if !cmd.trim().is_empty() => Ok((cmd.clone(), args.to_vec())),
        _ => Ok(("vi".to_string(), Vec::new())),
    }
}
