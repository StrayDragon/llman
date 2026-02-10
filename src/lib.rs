#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use std::sync::OnceLock;

pub mod arg_utils;
pub mod cli;
pub mod config;
pub mod config_schema;
pub mod error;
pub mod path_utils;
pub mod prompt;
pub mod sdd;
pub mod self_command;
pub mod skills;
pub mod tool;
pub mod x;

#[cfg(test)]
pub mod test_utils;

static LOCALE_INIT: OnceLock<()> = OnceLock::new();

pub fn init_locale() {
    LOCALE_INIT.get_or_init(|| {
        rust_i18n::set_locale("en");
    });
}
