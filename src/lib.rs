#[macro_use]
extern crate rust_i18n;

i18n!("locales");

pub mod cli;
pub mod config;
pub mod error;
pub mod path_utils;
pub mod prompt;
pub mod tool;
pub mod x;

#[cfg(test)]
pub mod test_utils;

pub fn init_locale() {
    rust_i18n::set_locale("en");
}
