use crate::x::claude_code::config::ConfigGroup;
use anyhow::{Result, bail};
use rust_i18n::t;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EnvSyntax {
    PosixExport,
    PowerShell,
}

pub fn env_syntax_for_current_platform() -> EnvSyntax {
    if cfg!(windows) {
        EnvSyntax::PowerShell
    } else {
        EnvSyntax::PosixExport
    }
}

pub fn render_env_injection_lines(group: &ConfigGroup, syntax: EnvSyntax) -> Result<Vec<String>> {
    let mut invalid_keys: Vec<&str> = group
        .keys()
        .filter(|key| !is_valid_env_key(key))
        .map(String::as_str)
        .collect();
    invalid_keys.sort_unstable();

    if !invalid_keys.is_empty() {
        bail!(t!(
            "claude_code.account.env_invalid_keys",
            keys = invalid_keys.join(", ")
        ));
    }

    let mut keys: Vec<&str> = group.keys().map(String::as_str).collect();
    keys.sort_unstable();

    let mut lines = Vec::with_capacity(keys.len());
    for key in keys {
        let value = group
            .get(key)
            .expect("key collected from group keys must exist");

        let quoted_value = match syntax {
            EnvSyntax::PosixExport => quote_posix_single(value),
            EnvSyntax::PowerShell => quote_powershell_single(value),
        };

        let line = match syntax {
            EnvSyntax::PosixExport => format!("export {key}={quoted_value}"),
            EnvSyntax::PowerShell => format!("$env:{key}={quoted_value}"),
        };

        lines.push(line);
    }

    Ok(lines)
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }

    true
}

fn quote_posix_single(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn quote_powershell_single(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn quote_posix_single_escapes_single_quote() {
        assert_eq!(quote_posix_single("a'b"), "'a'\\''b'");
    }

    #[test]
    fn quote_posix_single_handles_empty() {
        assert_eq!(quote_posix_single(""), "''");
    }

    #[test]
    fn quote_posix_single_preserves_spaces() {
        assert_eq!(quote_posix_single("hello world"), "'hello world'");
    }

    #[test]
    fn quote_powershell_single_escapes_single_quote() {
        assert_eq!(quote_powershell_single("a'b"), "'a''b'");
    }

    #[test]
    fn quote_powershell_single_handles_empty() {
        assert_eq!(quote_powershell_single(""), "''");
    }

    #[test]
    fn quote_powershell_single_preserves_spaces() {
        assert_eq!(quote_powershell_single("hello world"), "'hello world'");
    }

    #[test]
    fn render_env_injection_lines_sorts_keys() {
        let group: ConfigGroup = HashMap::from([
            ("B".to_string(), "2".to_string()),
            ("A".to_string(), "1".to_string()),
        ]);

        let lines = render_env_injection_lines(&group, EnvSyntax::PosixExport).expect("lines");
        assert_eq!(lines, vec!["export A='1'", "export B='2'"]);
    }

    #[test]
    fn render_env_injection_lines_rejects_invalid_keys() {
        let group: ConfigGroup = HashMap::from([("BAD-KEY".to_string(), "1".to_string())]);
        let err =
            render_env_injection_lines(&group, EnvSyntax::PosixExport).expect_err("should fail");
        assert!(
            err.to_string().contains("Invalid environment variable key"),
            "unexpected error: {err}"
        );
    }
}
