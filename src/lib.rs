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

use std::env;

use config::ENV_LANG;

pub fn init_locale() {
    let locale = match env::var(ENV_LANG) {
        Ok(lang) if lang == "zh-CN" || lang == "zh" => "zh-CN",
        _ => "en",
    };
    rust_i18n::set_locale(locale);
}
