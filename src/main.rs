#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use anyhow::Result;
use std::env;

mod cli;
mod config;
mod error;
mod prompt;

use cli::Cli;
use config::ENV_LANG;

fn main() -> Result<()> {
    let locale = match env::var(ENV_LANG) {
        Ok(lang) if lang == "zh-CN" || lang == "zh" => "zh-CN",
        _ => "en",
    };
    rust_i18n::set_locale(locale);

    let cli = Cli::new();

    if let Err(e) = cli.run() {
        eprintln!("{}", t!("messages.error", error = e.to_string()));
        std::process::exit(1);
    }

    Ok(())
}
