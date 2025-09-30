#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use llman::init_locale;
use llman::cli;

fn main() {
    init_locale();

    if let Err(e) = cli::run() {
        eprintln!("{}", t!("messages.error", error = e.to_string()));
        std::process::exit(1);
    }
}
