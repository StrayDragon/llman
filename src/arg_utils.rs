#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SplitShellArgsError {
    #[error("unclosed single quote")]
    UnclosedSingleQuote,
    #[error("unclosed double quote")]
    UnclosedDoubleQuote,
    #[error("trailing escape (\\)")]
    TrailingEscape,
}

/// Split a command-line string into argv-like tokens using simple shell-like rules.
///
/// Supported:
/// - Whitespace separates arguments (outside of quotes)
/// - Single quotes (`'...'`) preserve content literally (no escaping)
/// - Double quotes (`\"...\"`) preserve content; backslash escapes the next character
/// - Backslash (`\\`) escapes the next character (outside quotes and in double quotes)
///
/// Errors:
/// - Unclosed single/double quotes
/// - Trailing backslash escape
pub fn split_shell_args(input: &str) -> Result<Vec<String>, SplitShellArgsError> {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Normal,
        Single,
        Double,
    }

    let mut mode = Mode::Normal;
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match mode {
            Mode::Normal => {
                if ch.is_whitespace() {
                    if in_token {
                        args.push(std::mem::take(&mut current));
                        in_token = false;
                    }
                    continue;
                }

                in_token = true;
                match ch {
                    '\'' => mode = Mode::Single,
                    '"' => mode = Mode::Double,
                    '\\' => match chars.next() {
                        Some(next) => current.push(next),
                        None => return Err(SplitShellArgsError::TrailingEscape),
                    },
                    _ => current.push(ch),
                }
            }
            Mode::Single => match ch {
                '\'' => mode = Mode::Normal,
                _ => current.push(ch),
            },
            Mode::Double => match ch {
                '"' => mode = Mode::Normal,
                '\\' => match chars.next() {
                    Some(next) => current.push(next),
                    None => return Err(SplitShellArgsError::TrailingEscape),
                },
                _ => current.push(ch),
            },
        }
    }

    match mode {
        Mode::Normal => {}
        Mode::Single => return Err(SplitShellArgsError::UnclosedSingleQuote),
        Mode::Double => return Err(SplitShellArgsError::UnclosedDoubleQuote),
    }

    if in_token {
        args.push(current);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_shell_args_basic_whitespace() {
        let args = split_shell_args("a  b\tc").expect("parse");
        assert_eq!(args, vec!["a", "b", "c"]);
    }

    #[test]
    fn split_shell_args_double_quotes() {
        let args = split_shell_args("--message \"hello world\" --flag").expect("parse");
        assert_eq!(args, vec!["--message", "hello world", "--flag"]);
    }

    #[test]
    fn split_shell_args_single_quotes() {
        let args = split_shell_args("--message 'hello world'").expect("parse");
        assert_eq!(args, vec!["--message", "hello world"]);
    }

    #[test]
    fn split_shell_args_backslash_escapes() {
        let args = split_shell_args("hello\\ world").expect("parse");
        assert_eq!(args, vec!["hello world"]);

        let args = split_shell_args("\"a\\\"b\"").expect("parse");
        assert_eq!(args, vec!["a\"b"]);

        let args = split_shell_args("path\\\\name").expect("parse");
        assert_eq!(args, vec!["path\\name"]);
    }

    #[test]
    fn split_shell_args_empty_quote_creates_empty_arg() {
        let args = split_shell_args("--x \"\"").expect("parse");
        assert_eq!(args, vec!["--x", ""]);
    }

    #[test]
    fn split_shell_args_unclosed_quotes_error() {
        assert_eq!(
            split_shell_args("'unclosed").unwrap_err(),
            SplitShellArgsError::UnclosedSingleQuote
        );
        assert_eq!(
            split_shell_args("\"unclosed").unwrap_err(),
            SplitShellArgsError::UnclosedDoubleQuote
        );
    }

    #[test]
    fn split_shell_args_trailing_escape_error() {
        assert_eq!(
            split_shell_args("oops\\").unwrap_err(),
            SplitShellArgsError::TrailingEscape
        );
        assert_eq!(
            split_shell_args("\"oops\\").unwrap_err(),
            SplitShellArgsError::TrailingEscape
        );
    }
}
