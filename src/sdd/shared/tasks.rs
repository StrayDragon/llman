use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static RE_DEFER_LINKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(defer\s*→\s*([\w-]+)\)").unwrap());

static RE_CANCELLED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\(cancell?ed\s*[-–—]\s*(.+)\)").unwrap());

static RE_DEFER_LEGACY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(defer\s*[-–—]\s*(.+)\)").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Completed,
    Pending,
    Deferred { target: String },
    LegacyDefer { reason: String },
    Cancelled { reason: String },
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
    pub deferred: usize,
    pub legacy_defer: usize,
    pub cancelled: usize,
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

fn classify_unchecked(text: &str) -> TaskStatus {
    if let Some(caps) = RE_DEFER_LINKED.captures(text) {
        return TaskStatus::Deferred {
            target: caps[1].to_string(),
        };
    }
    if let Some(caps) = RE_CANCELLED.captures(text) {
        return TaskStatus::Cancelled {
            reason: caps[1].trim().to_string(),
        };
    }
    if let Some(caps) = RE_DEFER_LEGACY.captures(text) {
        return TaskStatus::LegacyDefer {
            reason: caps[1].trim().to_string(),
        };
    }
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
            TaskStatus::Deferred { .. } => report.deferred += 1,
            TaskStatus::LegacyDefer { .. } => report.legacy_defer += 1,
            TaskStatus::Cancelled { .. } => report.cancelled += 1,
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
    fn parse_linked_defer() {
        let content = "- [ ] Refactor hub (defer → c98-refactor-hub)\n";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 1);
        assert_eq!(report.deferred, 1);
        assert_eq!(
            report.items[0].status,
            TaskStatus::Deferred {
                target: "c98-refactor-hub".to_string()
            }
        );
    }

    #[test]
    fn parse_linked_defer_no_spaces() {
        let content = "- [ ] Refactor (defer→c98)\n";
        let report = parse_tasks(content);
        assert_eq!(report.deferred, 1);
        assert_eq!(
            report.items[0].status,
            TaskStatus::Deferred {
                target: "c98".to_string()
            }
        );
    }

    #[test]
    fn parse_legacy_defer() {
        let content = "- [ ] Refactor hub (defer - needs bigger rewrite)\n";
        let report = parse_tasks(content);
        assert_eq!(report.legacy_defer, 1);
        assert_eq!(
            report.items[0].status,
            TaskStatus::LegacyDefer {
                reason: "needs bigger rewrite".to_string()
            }
        );
    }

    #[test]
    fn parse_legacy_defer_em_dash() {
        let content = "- [ ] Clean up (defer — too complex)\n";
        let report = parse_tasks(content);
        assert_eq!(report.legacy_defer, 1);
    }

    #[test]
    fn parse_cancelled() {
        let content = "- [ ] Remove deps (cancelled — no longer needed)\n";
        let report = parse_tasks(content);
        assert_eq!(report.cancelled, 1);
        assert_eq!(
            report.items[0].status,
            TaskStatus::Cancelled {
                reason: "no longer needed".to_string()
            }
        );
    }

    #[test]
    fn parse_canceled_american_spelling() {
        let content = "- [ ] Remove deps (canceled - done)\n";
        let report = parse_tasks(content);
        assert_eq!(report.cancelled, 1);
    }

    #[test]
    fn checked_overrides_annotations() {
        let content = "- [x] Done (defer → c99)\n";
        let report = parse_tasks(content);
        assert_eq!(report.completed, 1);
        assert_eq!(report.deferred, 0);
    }

    #[test]
    fn mixed_status_report() {
        let content = "\
## Tasks
- [x] Completed task
- [ ] Pending task
- [ ] Deferred (defer → c100-followup)
- [ ] Legacy deferred (defer - reason)
- [ ] Cancelled work (cancelled — not needed)
";
        let report = parse_tasks(content);
        assert_eq!(report.total(), 5);
        assert_eq!(report.completed, 1);
        assert_eq!(report.pending, 1);
        assert_eq!(report.deferred, 1);
        assert_eq!(report.legacy_defer, 1);
        assert_eq!(report.cancelled, 1);
        assert!((report.completion_ratio() - 0.2).abs() < f64::EPSILON);
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
