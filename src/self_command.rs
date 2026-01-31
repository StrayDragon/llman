use crate::config_schema::{
    ApplyResult, GLOBAL_SCHEMA_URL, LLMANSPEC_SCHEMA_URL, PROJECT_SCHEMA_URL, SchemaPaths,
    apply_schema_header, format_schema_errors, global_config_path, llmanspec_config_path,
    project_config_path, schema_paths, write_schema_files,
};
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use jsonschema::JSONSchema;
use serde_json::Value;
use std::fs;

#[derive(Parser)]
pub struct SelfArgs {
    #[command(subcommand)]
    pub command: SelfCommands,
}

#[derive(Subcommand)]
pub enum SelfCommands {
    /// Manage llman schemas and headers
    Schema(SchemaArgs),
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

pub fn run(args: &SelfArgs) -> Result<()> {
    match &args.command {
        SelfCommands::Schema(schema) => run_schema(schema),
    }
}

fn run_schema(args: &SchemaArgs) -> Result<()> {
    match args.command {
        SchemaCommands::Generate => run_generate(),
        SchemaCommands::Apply => run_apply(),
        SchemaCommands::Check => run_check(),
    }
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
        serde_json::to_value(crate::sdd::SddConfig::default())?,
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
