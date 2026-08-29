//! Minimal plain-text table renderer (hand-rolled, copy-friendly).
//!
//! Output format is deliberately free of box-drawing characters so users can
//! select and paste rows cleanly:
//!
//! ```text
//! path      exists
//! ----      ------
//! a.md      yes
//! llman.md  no
//! ```
//!
//! Column widths adapt to the content and total width budget (terminal width
//! via crossterm, `COLUMNS` env, then a fallback); over-long cells wrap.

use unicode_width::UnicodeWidthChar as _;

/// Two-space gutter between columns keeps rows token-separable on paste.
const GUTTER: &str = "  ";
/// Columns never shrink below this many columns so content stays readable.
const MIN_COLUMN_WIDTH: usize = 3;
/// Fallback total width when neither the terminal nor `COLUMNS` is available.
const FALLBACK_WIDTH: usize = 100;

/// A plain-text table under construction.
#[derive(Debug, Default)]
pub(crate) struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub(crate) fn new(header: Vec<String>) -> Self {
        Self {
            header,
            rows: Vec::new(),
        }
    }

    pub(crate) fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Render using the detected terminal width.
    pub(crate) fn render(&self) -> String {
        self.render_with_width(usable_width())
    }

    /// Render constrained to `total_width` columns.
    fn render_with_width(&self, total_width: usize) -> String {
        let column_count = self.header.len();
        let overhead = GUTTER.len() * column_count.saturating_sub(1);
        let budget = total_width.saturating_sub(overhead);
        let widths = self.fit_column_widths(budget);

        let mut out = String::new();
        push_row(&mut out, &self.header, &widths);
        push_separator(&mut out, &widths);
        for row in &self.rows {
            push_row(&mut out, row, &widths);
        }
        out
    }

    /// Per-column widths: natural maxima (each cell measured against its own
    /// column), shrunk widest-first until the total fits `budget`.
    fn fit_column_widths(&self, budget: usize) -> Vec<usize> {
        let column_count = self.header.len();
        let mut widths = vec![MIN_COLUMN_WIDTH; column_count];
        let mut measure = |cells: &[String]| {
            for (index, cell) in cells.iter().enumerate().take(column_count) {
                for line in cell.split('\n') {
                    widths[index] = widths[index].max(display_width(line));
                }
            }
        };
        measure(&self.header);
        for row in &self.rows {
            measure(row);
        }

        while widths.iter().sum::<usize>() > budget {
            let Some(widest) = widths
                .iter()
                .enumerate()
                .max_by_key(|(_, width)| **width)
                .map(|(index, _)| index)
            else {
                break;
            };
            if widths[widest] <= MIN_COLUMN_WIDTH {
                break;
            }
            widths[widest] -= 1;
        }
        widths
    }
}

/// One dashed underline per column, mirroring each column's width.
fn push_separator(out: &mut String, widths: &[usize]) {
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            out.push_str(GUTTER);
        }
        for _ in 0..*width {
            out.push('-');
        }
    }
    out.push('\n');
}

/// Render one logical row; cells may wrap to multiple physical lines. Lines
/// carry no trailing whitespace (kept copy-friendly).
fn push_row(out: &mut String, cells: &[String], widths: &[usize]) {
    let lines: Vec<Vec<String>> = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| wrap_cell(cell, widths[index]))
        .collect();
    let height = lines.iter().map(Vec::len).max().unwrap_or(1);

    for line_index in 0..height {
        let mut line = String::new();
        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                line.push_str(GUTTER);
            }
            let text = lines[index]
                .get(line_index)
                .map(String::as_str)
                .unwrap_or("");
            if index + 1 == widths.len() {
                line.push_str(text);
            } else {
                push_padded(&mut line, text, *width);
            }
        }
        let line = line.trim_end();
        out.push_str(line);
        out.push('\n');
    }
}

fn push_padded(out: &mut String, text: &str, width: usize) {
    out.push_str(text);
    for _ in display_width(text)..width {
        out.push(' ');
    }
}

/// Greedy whitespace wrapping with hard breaks for over-long tokens.
fn wrap_cell(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        let mut current_width = 0;
        for word in split_words(paragraph) {
            let word_width = display_width(&word);
            if !current.is_empty() && current_width + 1 + word_width > width {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            if word_width > width && current.is_empty() {
                // Hard-break a token that can never fit on its own line.
                let mut chunk = String::new();
                let mut chunk_width = 0;
                for ch in word.chars() {
                    let ch_width = char_width(ch);
                    if chunk_width + ch_width > width {
                        lines.push(std::mem::take(&mut chunk));
                        chunk_width = 0;
                    }
                    chunk.push(ch);
                    chunk_width += ch_width;
                }
                current = chunk;
                current_width = chunk_width;
            } else {
                if !current.is_empty() {
                    current.push(' ');
                    current_width += 1;
                }
                current.push_str(&word);
                current_width += word_width;
            }
        }
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Split on whitespace, keeping whitespace-free tokens intact.
fn split_words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

fn char_width(ch: char) -> usize {
    if ch.is_control() {
        0
    } else {
        ch.width().unwrap_or(0)
    }
}

/// Terminal width, preferring crossterm's probe and falling back to the
/// `COLUMNS` environment variable, then a fixed default.
fn usable_width() -> usize {
    if let Ok((columns, _)) = crossterm::terminal::size() {
        return columns.max(20) as usize;
    }
    if let Ok(parsed) = std::env::var("COLUMNS").map(|columns| columns.parse::<usize>())
        && let Ok(parsed) = parsed
        && parsed >= 20
    {
        return parsed;
    }
    FALLBACK_WIDTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_header_dashes_and_rows() {
        let mut table = Table::new(vec!["path".into(), "exists".into()]);
        table.add_row(vec!["a.md".into(), "yes".into()]);
        let rendered = table.render_with_width(40);

        assert_eq!(rendered, "path  exists\n----  ------\na.md  yes\n");
    }

    #[test]
    fn pads_inner_columns_but_keeps_last_column_trimmed() {
        let mut table = Table::new(vec!["a".into(), "b".into()]);
        table.add_row(vec!["long-cell".into(), "x".into()]);
        let rendered = table.render_with_width(60);

        assert_eq!(rendered, "a          b\n---------  ---\nlong-cell  x\n");
    }

    #[test]
    fn wraps_cells_that_exceed_the_budget() {
        let mut table = Table::new(vec!["h".into()]);
        table.add_row(vec!["some rather long path/that/keeps going on".into()]);
        let rendered = table.render_with_width(20);

        // 20 - one column → 16 usable columns for the single cell.
        assert!(rendered.lines().count() > 3, "wrapped: {rendered}");
        assert!(
            rendered.contains("some rather"),
            "first words kept: {rendered}"
        );
    }

    #[test]
    fn hard_breaks_tokens_longer_than_the_column() {
        let mut table = Table::new(vec!["h".into()]);
        table.add_row(vec!["averyveryverylongtoken".into()]);
        let rendered = table.render_with_width(12);

        assert!(rendered.lines().count() > 3, "hard-broken: {rendered}");
    }

    #[test]
    fn counts_cjk_cells_at_double_width() {
        let mut table = Table::new(vec!["中文".into()]);
        table.add_row(vec!["路径".into()]);
        let rendered = table.render_with_width(40);

        assert_eq!(rendered, "中文\n----\n路径\n");
    }

    #[test]
    fn shrinks_widest_column_first_when_budget_is_tight() {
        let mut table = Table::new(vec!["h1".into(), "h2".into()]);
        table.add_row(vec![
            "short".into(),
            "a very long wrapping cell here".into(),
        ]);
        let rendered = table.render_with_width(30);

        assert!(
            rendered.lines().all(|line| line.chars().count() <= 30),
            "within budget: {rendered}"
        );
    }

    #[test]
    fn multiline_wrapped_rows_keep_no_trailing_whitespace() {
        let mut table = Table::new(vec!["h1".into(), "h2".into()]);
        table.add_row(vec!["one two three four".into(), "x".into()]);
        let rendered = table.render_with_width(20);

        let body_lines: Vec<&str> = rendered.lines().skip(2).collect();
        assert!(body_lines.len() >= 2, "wrapped body: {rendered}");
        assert_eq!(body_lines[0], "one two three    x");
        // Continuation line: empty second column must not leave trailing
        // whitespace behind.
        assert_eq!(body_lines[1], "four");
    }
}
