use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Completed,
    Pending,
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub line_num: usize,
    pub text: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Default)]
pub struct TasksReport {
    pub items: Vec<TaskItem>,
    pub completed: usize,
    pub pending: usize,
}

impl TasksReport {
    pub fn total(&self) -> usize {
        self.items.len()
    }

    pub fn completion_ratio(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 1.0;
        }
        self.completed as f64 / total as f64
    }
}

fn is_checkbox_line(trimmed: &str) -> bool {
    trimmed.starts_with("- [") || trimmed.starts_with("* [")
}

fn is_checked(trimmed: &str) -> bool {
    let lower = trimmed.to_lowercase();
    lower.starts_with("- [x]") || lower.starts_with("* [x]")
}

fn classify_unchecked(_text: &str) -> TaskStatus {
    TaskStatus::Pending
}

pub fn parse_tasks(content: &str) -> TasksReport {
    let mut report = TasksReport::default();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if !is_checkbox_line(trimmed) {
            continue;
        }

        let status = if is_checked(trimmed) {
            TaskStatus::Completed
        } else {
            classify_unchecked(trimmed)
        };

        match &status {
            TaskStatus::Completed => report.completed += 1,
            TaskStatus::Pending => report.pending += 1,
        }

        let checkbox_text = extract_task_text(trimmed);
        report.items.push(TaskItem {
            line_num: idx + 1,
            text: checkbox_text,
            status,
        });
    }

    report
}

fn extract_task_text(trimmed: &str) -> String {
    let after = if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- [X] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("* [x] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("* [X] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("* [ ] ") {
        rest
    } else {
        trimmed
    };
    after.to_string()
}

pub fn parse_tasks_file(tasks_path: &Path) -> Result<Option<TasksReport>> {
    let content = match fs::read_to_string(tasks_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    Ok(Some(parse_tasks(&content)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_completed() {
        let content = "- [x] First\n- [X] Second\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 2);
        assert_eq!(report.completed, 2);
        assert_eq!(report.pending, 0);
    }

    #[test]
    fn parse_pending_tasks() {
        let content = "- [ ] Do thing\n- [x] Done thing\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 2);
        assert_eq!(report.completed, 1);
        assert_eq!(report.pending, 1);
    }

    #[test]
    fn legacy_annotations_are_pending() {
        // defer/cancelled annotations are no longer special; all unchecked are Pending.
        let content = "- [ ] Deferred (defer → c100)\n- [ ] Cancelled (cancelled — done)\n- [ ] Legacy (defer - reason)\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 3);
        assert_eq!(report.completed, 0);
        assert_eq!(report.pending, 3);
    }

    #[test]
    fn checked_overrides_annotations() {
        let content = "- [x] Done (defer → c99)\n";
        let report = parse_tasks(content);
        assert_eq!(report.completed, 1);
        assert_eq!(report.pending, 0);
    }

    #[test]
    fn mixed_status_report() {
        let content = "- [x] Completed task\n- [ ] Pending task\n- [ ] Another pending\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 3);
        assert_eq!(report.completed, 1);
        assert_eq!(report.pending, 2);
        assert!((report.completion_ratio() - 1.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_content_returns_empty_report() {
        let report = parse_tasks("");
        assert_eq!(report.total(), 0);
        assert!((report.completion_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn non_checkbox_lines_ignored() {
        let content = "## Header\nSome text\n- plain list item\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 0);
    }

    #[test]
    fn asterisk_checkboxes_work() {
        let content = "* [x] Done\n* [ ] Pending\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 2);
        assert_eq!(report.completed, 1);
        assert_eq!(report.pending, 1);
    }

    #[test]
    fn indented_checkboxes_work() {
        let content = "  - [x] Nested done\n  - [ ] Nested pending\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 2);
        assert_eq!(report.completed, 1);
    }

    #[test]
    fn line_numbers_are_one_based() {
        let content = "## Tasks\n- [ ] First\n- [x] Second\n";
        let report = parse_tasks(content);
        assert_eq!(report.items[0].line_num, 2);
        assert_eq!(report.items[1].line_num, 3);
    }

    #[test]
    fn extract_text_strips_checkbox_prefix() {
        let report = parse_tasks("- [ ] Do the thing\n");
        assert_eq!(report.items[0].text, "Do the thing");
    }

    #[test]
    fn parse_tasks_file_missing_returns_none() {
        let result = parse_tasks_file(Path::new("/nonexistent/tasks.md")).unwrap();
        assert!(result.is_none());
    }
}
