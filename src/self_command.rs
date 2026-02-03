use crate::cli::Cli;
use crate::config_schema::{
    ApplyResult, GLOBAL_SCHEMA_URL, LLMANSPEC_SCHEMA_URL, PROJECT_SCHEMA_URL, SchemaPaths,
    apply_schema_header, format_schema_errors, global_config_path, llmanspec_config_path,
    project_config_path, schema_paths, write_schema_files,
};
use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::generate;
use inquire::Confirm;
use jsonschema::JSONSchema;
use serde_json::Value;
use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

#[derive(Parser)]
pub struct SelfArgs {
    #[command(subcommand)]
    pub command: SelfCommands,
}

#[derive(Subcommand)]
pub enum SelfCommands {
    /// Manage llman schemas and headers
    Schema(SchemaArgs),
    /// Generate or install shell completions
    Completion(CompletionArgs),
}

#[derive(Parser)]
pub struct SchemaArgs {
    #[command(subcommand)]
    pub command: SchemaCommands,
}

#[derive(Subcommand)]
pub enum SchemaCommands {
    /// Generate JSON schema files
    Generate,
    /// Apply YAML LSP schema headers to config files
    Apply,
    /// Validate schema files against sample configs
    Check,
}

#[derive(Parser)]
pub struct CompletionArgs {
    /// Target shell for completion generation
    #[arg(long, value_enum)]
    pub shell: CompletionShell,
    /// Install completion block into shell rc/profile
    #[arg(long)]
    pub install: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CompletionShell {
    #[value(name = "bash")]
    Bash,
    #[value(name = "zsh")]
    Zsh,
    #[value(name = "fish")]
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    #[value(name = "elvish")]
    Elvish,
}

impl CompletionShell {
    fn as_clap_shell(self) -> clap_complete::Shell {
        match self {
            Self::Bash => clap_complete::Shell::Bash,
            Self::Zsh => clap_complete::Shell::Zsh,
            Self::Fish => clap_complete::Shell::Fish,
            Self::PowerShell => clap_complete::Shell::PowerShell,
            Self::Elvish => clap_complete::Shell::Elvish,
        }
    }
}

pub fn run(args: &SelfArgs) -> Result<()> {
    match &args.command {
        SelfCommands::Schema(schema) => run_schema(schema),
        SelfCommands::Completion(completion) => run_completion(completion),
    }
}

fn run_schema(args: &SchemaArgs) -> Result<()> {
    match args.command {
        SchemaCommands::Generate => run_generate(),
        SchemaCommands::Apply => run_apply(),
        SchemaCommands::Check => run_check(),
    }
}

fn run_completion(args: &CompletionArgs) -> Result<()> {
    if args.install {
        install_completion(args.shell)
    } else {
        generate_completion(args.shell)
    }
}

fn generate_completion(shell: CompletionShell) -> Result<()> {
    let mut command = Cli::command();
    let name = command.get_name().to_string();
    let mut stdout = io::stdout();
    generate(shell.as_clap_shell(), &mut command, name, &mut stdout);
    Ok(())
}

fn install_completion(shell: CompletionShell) -> Result<()> {
    let profile_path = shell_profile_path(shell)?;
    if !confirm_install(&profile_path)? {
        println!("{}", t!("messages.operation_cancelled"));
        return Ok(());
    }
    let snippet = completion_snippet(shell);
    update_completion_block(&profile_path, snippet)?;
    println!("{}", completion_block(shell));
    Ok(())
}

fn confirm_install(path: &Path) -> Result<bool> {
    if !io::stdin().is_terminal() {
        return Err(anyhow!(t!(
            "self.completion.non_interactive",
            path = path.display()
        )));
    }
    let prompt = t!("self.completion.install_prompt", path = path.display());
    let help = t!("self.completion.install_help");
    let confirmed = Confirm::new(&prompt)
        .with_default(false)
        .with_help_message(&help)
        .prompt()
        .map_err(|e| anyhow!(t!("errors.inquire_error", error = e)))?;
    Ok(confirmed)
}

fn completion_snippet(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => "source <(llman self completion --shell bash)",
        CompletionShell::Zsh => "source <(llman self completion --shell zsh)",
        CompletionShell::Fish => "llman self completion --shell fish | source",
        CompletionShell::PowerShell => {
            "llman self completion --shell powershell | Out-String | Invoke-Expression"
        }
        CompletionShell::Elvish => "eval (llman self completion --shell elvish)",
    }
}

fn completion_block(shell: CompletionShell) -> String {
    format!(
        "{start}\n{body}\n{end}",
        start = COMPLETION_MARKER_START,
        body = completion_snippet(shell),
        end = COMPLETION_MARKER_END
    )
}

fn shell_profile_path(shell: CompletionShell) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!(t!("errors.home_dir_missing")))?;
    match shell {
        CompletionShell::Bash => Ok(bash_profile_path(&home)),
        CompletionShell::Zsh => Ok(home.join(".zshrc")),
        CompletionShell::Fish => Ok(home.join(".config/fish/config.fish")),
        CompletionShell::PowerShell => match env::var("PROFILE") {
            Ok(profile) if !profile.trim().is_empty() => Ok(PathBuf::from(profile)),
            _ => Ok(home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")),
        },
        CompletionShell::Elvish => Ok(home.join(".elvish/rc.elv")),
    }
}

fn bash_profile_path(home: &Path) -> PathBuf {
    let bashrc = home.join(".bashrc");
    if bashrc.exists() {
        return bashrc;
    }
    let bash_profile = home.join(".bash_profile");
    if bash_profile.exists() {
        return bash_profile;
    }
    let profile = home.join(".profile");
    if profile.exists() {
        return profile;
    }
    bashrc
}

const COMPLETION_MARKER_START: &str = "# >>> llman completion >>>";
const COMPLETION_MARKER_END: &str = "# <<< llman completion <<<";

fn is_marker_on_own_line(content: &str, marker_index: usize, marker_len: usize) -> bool {
    let bytes = content.as_bytes();
    let mut left = marker_index as isize - 1;
    while left >= 0 {
        let ch = bytes[left as usize] as char;
        if ch == '\n' {
            break;
        }
        if ch != ' ' && ch != '\t' && ch != '\r' {
            return false;
        }
        left -= 1;
    }

    let mut right = marker_index + marker_len;
    while right < bytes.len() {
        let ch = bytes[right] as char;
        if ch == '\n' {
            break;
        }
        if ch != ' ' && ch != '\t' && ch != '\r' {
            return false;
        }
        right += 1;
    }

    true
}

fn find_marker_index(content: &str, marker: &str, from_index: usize) -> Option<usize> {
    let mut search_index = from_index;
    while let Some(pos) = content[search_index..].find(marker) {
        let idx = search_index + pos;
        if is_marker_on_own_line(content, idx, marker.len()) {
            return Some(idx);
        }
        search_index = idx + marker.len();
        if search_index >= content.len() {
            break;
        }
    }
    None
}

fn update_completion_block(path: &Path, body: &str) -> Result<()> {
    let mut content = if path.exists() {
        fs::read_to_string(path).map_err(|e| {
            anyhow!(t!(
                "self.completion.read_failed",
                path = path.display(),
                error = e
            ))
        })?
    } else {
        String::new()
    };

    if content.is_empty() {
        content = format!(
            "{start}\n{body}\n{end}\n",
            start = COMPLETION_MARKER_START,
            body = body,
            end = COMPLETION_MARKER_END
        );
    } else {
        let start_index = find_marker_index(&content, COMPLETION_MARKER_START, 0);
        let end_index = start_index
            .and_then(|start| {
                find_marker_index(
                    &content,
                    COMPLETION_MARKER_END,
                    start + COMPLETION_MARKER_START.len(),
                )
            })
            .or_else(|| find_marker_index(&content, COMPLETION_MARKER_END, 0));

        match (start_index, end_index) {
            (Some(start), Some(end)) => {
                if end < start {
                    return Err(anyhow!(t!(
                        "self.completion.invalid_marker",
                        path = path.display()
                    )));
                }
                let before = &content[..start];
                let after = &content[end + COMPLETION_MARKER_END.len()..];
                content = format!(
                    "{before}{start_marker}\n{body}\n{end_marker}{after}",
                    start_marker = COMPLETION_MARKER_START,
                    end_marker = COMPLETION_MARKER_END
                );
            }
            (None, None) => {
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(COMPLETION_MARKER_START);
                content.push('\n');
                content.push_str(body);
                content.push('\n');
                content.push_str(COMPLETION_MARKER_END);
                content.push('\n');
            }
            _ => {
                return Err(anyhow!(t!(
                    "self.completion.invalid_marker",
                    path = path.display()
                )));
            }
        }
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content).map_err(|e| {
        anyhow!(t!(
            "self.completion.write_failed",
            path = path.display(),
            error = e
        ))
    })?;
    Ok(())
}

fn run_generate() -> Result<()> {
    println!("{}", t!("self.schema.generate_start"));
    let paths = write_schema_files()?;
    print_written(&paths)?;
    Ok(())
}

fn run_apply() -> Result<()> {
    println!("{}", t!("self.schema.apply_start"));
    let global_path = global_config_path()?;
    let project_path = project_config_path()?;
    let llmanspec_path = llmanspec_config_path()?;

    apply_and_report(&global_path, GLOBAL_SCHEMA_URL)?;
    apply_and_report(&project_path, PROJECT_SCHEMA_URL)?;
    apply_and_report(&llmanspec_path, LLMANSPEC_SCHEMA_URL)?;
    Ok(())
}

fn run_check() -> Result<()> {
    println!("{}", t!("self.schema.check_start"));
    let paths = schema_paths();
    let global_schema = load_schema(&paths.global)?;
    let project_schema = load_schema(&paths.project)?;
    let llmanspec_schema = load_schema(&paths.llmanspec)?;

    validate_schema(
        "llman-config",
        &global_schema,
        serde_json::to_value(crate::config_schema::GlobalConfig::default())?,
    )?;
    validate_schema(
        "llman-project-config",
        &project_schema,
        serde_json::to_value(crate::config_schema::ProjectConfig::default())?,
    )?;
    validate_schema(
        "llmanspec-config",
        &llmanspec_schema,
        serde_json::to_value(crate::sdd::project::config::SddConfig::default())?,
    )?;

    println!("{}", t!("self.schema.check_ok"));
    Ok(())
}

fn print_written(paths: &SchemaPaths) -> Result<()> {
    println!(
        "{}",
        t!(
            "self.schema.generate_written",
            path = paths.global.display()
        )
    );
    println!(
        "{}",
        t!(
            "self.schema.generate_written",
            path = paths.project.display()
        )
    );
    println!(
        "{}",
        t!(
            "self.schema.generate_written",
            path = paths.llmanspec.display()
        )
    );
    Ok(())
}

fn apply_and_report(path: &std::path::Path, schema_url: &str) -> Result<()> {
    match apply_schema_header(path, schema_url)? {
        ApplyResult::Updated => {
            println!("{}", t!("self.schema.apply_updated", path = path.display()))
        }
        ApplyResult::Unchanged => println!(
            "{}",
            t!("self.schema.apply_unchanged", path = path.display())
        ),
        ApplyResult::Missing => {
            println!("{}", t!("self.schema.apply_skipped", path = path.display()))
        }
    }
    Ok(())
}

fn load_schema(path: &std::path::Path) -> Result<Value> {
    if !path.exists() {
        return Err(anyhow!(t!(
            "self.schema.check_missing",
            path = path.display()
        )));
    }
    let content = fs::read_to_string(path).map_err(|e| {
        anyhow!(t!(
            "self.schema.read_failed",
            path = path.display(),
            error = e
        ))
    })?;
    serde_json::from_str(&content).map_err(|e| {
        anyhow!(t!(
            "self.schema.check_invalid",
            path = path.display(),
            error = e
        ))
    })
}

fn validate_schema(name: &str, schema: &Value, instance: Value) -> Result<()> {
    let compiled = JSONSchema::compile(schema)
        .map_err(|e| anyhow!(t!("self.schema.check_invalid", path = name, error = e)))?;
    if let Err(errors) = compiled.validate(&instance) {
        let first = format_schema_errors(errors.map(|e| e.to_string()));
        return Err(anyhow!(t!(
            "self.schema.check_failed",
            name = name,
            error = first
        )));
    }
    Ok(())
}
