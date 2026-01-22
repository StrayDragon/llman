#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use llman::cli;
use llman::error::LlmanError;
use llman::init_locale;

fn main() {
    init_locale();

    if let Err(e) = cli::run() {
        let message = e
            .downcast_ref::<LlmanError>()
            .map(LlmanError::display_localized)
            .unwrap_or_else(|| e.to_string());
        eprintln!("{}", t!("messages.error", error = message));
        std::process::exit(1);
    }
}
