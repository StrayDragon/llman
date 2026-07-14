//! Thin wrapper: `llmanspec` ≡ `llman sdd`
//!
//! Synthesises argv as `<prog> sdd <user-args>` and delegates to `llman::cli::Cli`.
//! Supports all `sdd` subcommands directly:
//!   `llmanspec status`  ≡  `llman sdd status`
//!   `llmanspec init`    ≡  `llman sdd init`
//!   ...

#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use anyhow::Result;
use clap::{CommandFactory, Parser};

fn main() {
    llman::init_locale();

    if let Err(e) = run() {
        // If it's a clap error, print its message and use clap's exit code.
        if let Some(ce) = e.downcast_ref::<clap::Error>() {
            let code = ce.exit_code();
            // clap::Error::print() formats the help / error message.
            let _ = ce.print();
            std::process::exit(code);
        }

        let message = e
            .downcast_ref::<llman::error::LlmanError>()
            .map(llman::error::LlmanError::display_localized)
            .unwrap_or_else(|| e.to_string());
        eprintln!("{}", t!("messages.error", error = message));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let raw: Vec<String> = std::env::args().collect();
    let program = &raw[0];
    let rest: Vec<&str> = raw[1..].iter().map(String::as_str).collect();

    // Synthesise argv as: <program> sdd <user-args...>
    // This makes `llmanspec status` parse as if `llman sdd status` was typed.
    let argv: Vec<&str> = if rest.is_empty() {
        // Bare `llmanspec` → show sdd subcommand help and exit 0.
        vec![program.as_str(), "sdd", "--help"]
    } else {
        std::iter::once(program.as_str())
            .chain(std::iter::once("sdd"))
            .chain(rest.iter().copied())
            .collect()
    };

    let cli = llman::cli::Cli::try_parse_from(&argv)?;

    match cli.command {
        Some(llman::cli::Commands::Sdd(sdd_args)) => llman::sdd::command::run(&sdd_args),
        _ => {
            // Unreachable in practice – the injected "sdd" ensures this arm is never hit
            // for well-formed invocations.
            let mut cmd = llman::cli::Cli::command();
            cmd.print_help()?;
            Ok(())
        }
    }
}
