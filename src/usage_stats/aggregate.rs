use crate::usage_stats::model::{SessionId, SessionRecord, TokenUsage};
use crate::usage_stats::query::{GroupBy, StatsQuery, TimeRangeMode};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub total_sessions: usize,
    pub known_token_sessions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenTotals {
    pub tokens_total_known: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_input_known: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_output_known: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_cache_known: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_reasoning_known: Option<u64>,
}

#[derive(Default)]
struct TokenTotalsAccum {
    total: u64,
    input_sum: u64,
    input_any: bool,
    output_sum: u64,
    output_any: bool,
    cache_sum: u64,
    cache_any: bool,
    reasoning_sum: u64,
    reasoning_any: bool,
}

impl TokenTotalsAccum {
    fn add(&mut self, usage: &TokenUsage) {
        if let Some(v) = usage.total {
            self.total = self.total.saturating_add(v);
        }
        if let Some(v) = usage.input {
            self.input_any = true;
            self.input_sum = self.input_sum.saturating_add(v);
        }
        if let Some(v) = usage.output {
            self.output_any = true;
            self.output_sum = self.output_sum.saturating_add(v);
        }
        if let Some(v) = usage.cache {
            self.cache_any = true;
            self.cache_sum = self.cache_sum.saturating_add(v);
        }
        if let Some(v) = usage.reasoning {
            self.reasoning_any = true;
            self.reasoning_sum = self.reasoning_sum.saturating_add(v);
        }
    }

    fn build(self) -> TokenTotals {
        TokenTotals {
            tokens_total_known: self.total,
            tokens_input_known: self.input_any.then_some(self.input_sum),
            tokens_output_known: self.output_any.then_some(self.output_sum),
            tokens_cache_known: self.cache_any.then_some(self.cache_sum),
            tokens_reasoning_known: self.reasoning_any.then_some(self.reasoning_sum),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryView {
    pub totals: TokenTotals,
    pub coverage: Coverage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_end_ts: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsBucket {
    pub label: String,
    pub start_local: String,
    pub end_local_exclusive: String,
    pub totals: TokenTotals,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrendView {
    pub group_by: GroupBy,
    pub buckets: Vec<StatsBucket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionsView {
    pub total_sessions: usize,
    pub returned_sessions: usize,
    pub sessions: Vec<SessionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDetailView {
    pub session: SessionRecord,
}

pub fn filter_sessions_v1(sessions: &[SessionRecord], query: &StatsQuery) -> Vec<SessionRecord> {
    sessions
        .iter()
        .filter(|session| session.cwd == query.cwd)
        .filter(|session| match query.time_range.mode {
            TimeRangeMode::All => true,
            _ => {
                if let Some(since) = query.time_range.since
                    && session.end_ts < since
                {
                    return false;
                }
                if let Some(until) = query.time_range.until
                    && session.end_ts >= until
                {
                    return false;
                }
                true
            }
        })
        .cloned()
        .collect()
}

pub fn build_summary_view(sessions: &[SessionRecord]) -> SummaryView {
    let coverage = Coverage {
        total_sessions: sessions.len(),
        known_token_sessions: sessions
            .iter()
            .filter(|session| session.token_usage.total.is_some())
            .count(),
    };

    let mut totals = TokenTotalsAccum::default();
    let mut latest: Option<DateTime<Utc>> = None;

    for session in sessions {
        totals.add(&session.token_usage);
        latest = match latest {
            None => Some(session.end_ts),
            Some(current) => Some(current.max(session.end_ts)),
        };
    }

    SummaryView {
        totals: totals.build(),
        coverage,
        latest_end_ts: latest,
    }
}

pub fn build_trend_view(sessions: &[SessionRecord], group_by: GroupBy) -> TrendView {
    let mut buckets: BTreeMap<NaiveDate, Vec<&SessionRecord>> = BTreeMap::new();

    for session in sessions {
        let key = bucket_start_date_local(session.end_ts, group_by);
        buckets.entry(key).or_default().push(session);
    }

    let buckets = buckets
        .into_iter()
        .map(|(start, sessions)| build_bucket(start, group_by, sessions))
        .collect();

    TrendView { group_by, buckets }
}

fn build_bucket(start: NaiveDate, group_by: GroupBy, sessions: Vec<&SessionRecord>) -> StatsBucket {
    let (label, end_exclusive) = bucket_label_and_end_exclusive(start, group_by);

    let coverage = Coverage {
        total_sessions: sessions.len(),
        known_token_sessions: sessions
            .iter()
            .filter(|session| session.token_usage.total.is_some())
            .count(),
    };

    let mut totals = TokenTotalsAccum::default();
    for session in sessions {
        totals.add(&session.token_usage);
    }

    StatsBucket {
        label,
        start_local: start.format("%Y-%m-%d").to_string(),
        end_local_exclusive: end_exclusive.format("%Y-%m-%d").to_string(),
        totals: totals.build(),
        coverage,
    }
}

fn bucket_start_date_local(end_ts: DateTime<Utc>, group_by: GroupBy) -> NaiveDate {
    let local_date = end_ts.with_timezone(&Local).date_naive();
    match group_by {
        GroupBy::Day => local_date,
        GroupBy::Week => {
            let days_from_monday = i64::from(local_date.weekday().num_days_from_monday());
            local_date - Duration::days(days_from_monday)
        }
        GroupBy::Month => NaiveDate::from_ymd_opt(local_date.year(), local_date.month(), 1)
            .expect("valid first-of-month"),
    }
}

fn bucket_label_and_end_exclusive(start: NaiveDate, group_by: GroupBy) -> (String, NaiveDate) {
    match group_by {
        GroupBy::Day => (
            start.format("%Y-%m-%d").to_string(),
            start + Duration::days(1),
        ),
        GroupBy::Week => (
            start.format("%Y-%m-%d").to_string(),
            start + Duration::days(7),
        ),
        GroupBy::Month => {
            let label = format!("{:04}-{:02}", start.year(), start.month());
            let (year, month) = if start.month() == 12 {
                (start.year() + 1, 1)
            } else {
                (start.year(), start.month() + 1)
            };
            let end = NaiveDate::from_ymd_opt(year, month, 1).expect("valid first-of-month");
            (label, end)
        }
    }
}

pub fn build_sessions_view(sessions: &[SessionRecord], limit: usize) -> SessionsView {
    let mut sorted = sessions.to_vec();
    sorted.sort_by_key(|session| std::cmp::Reverse(session.end_ts));

    let total_sessions = sorted.len();
    let sessions = if limit == 0 {
        sorted
    } else {
        sorted.into_iter().take(limit).collect()
    };

    SessionsView {
        total_sessions,
        returned_sessions: sessions.len(),
        sessions,
    }
}

pub fn build_session_detail_view(
    sessions: &[SessionRecord],
    id: &SessionId,
) -> Option<SessionDetailView> {
    sessions
        .iter()
        .find(|session| &session.id == id)
        .cloned()
        .map(|session| SessionDetailView { session })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage_stats::model::{SessionRecord, TokenUsage, ToolKind};
    use crate::usage_stats::query::{StatsQuery, TimeRange};
    use chrono::{TimeZone, Weekday};
    use std::path::PathBuf;

    fn make_session(
        id: &str,
        cwd: &str,
        end_ts: DateTime<Utc>,
        tokens: Option<u64>,
    ) -> SessionRecord {
        SessionRecord {
            tool: ToolKind::Codex,
            id: SessionId(id.to_string()),
            cwd: cwd.into(),
            title: None,
            start_ts: None,
            end_ts,
            token_usage: TokenUsage {
                total: tokens,
                ..TokenUsage::default()
            },
            is_sidechain: None,
        }
    }

    #[test]
    fn sessions_view_sorts_desc_and_limits() {
        let sessions = vec![
            make_session(
                "a",
                "/p",
                DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                Some(10),
            ),
            make_session(
                "b",
                "/p",
                DateTime::parse_from_rfc3339("2026-02-02T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                Some(20),
            ),
        ];
        let view = build_sessions_view(&sessions, 1);
        assert_eq!(view.returned_sessions, 1);
        assert_eq!(view.sessions[0].id.0, "b");
        let view_all = build_sessions_view(&sessions, 0);
        assert_eq!(view_all.returned_sessions, 2);
    }

    #[test]
    fn filter_sessions_v1_filters_by_cwd_and_time() {
        let sessions = vec![
            make_session(
                "a",
                "/p/a",
                DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                Some(1),
            ),
            make_session(
                "b",
                "/p/b",
                DateTime::parse_from_rfc3339("2026-02-02T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                Some(1),
            ),
        ];

        let query = StatsQuery {
            view: crate::usage_stats::query::ViewKind::Summary,
            group_by: GroupBy::Day,
            time_range: TimeRange {
                mode: TimeRangeMode::SinceUntil,
                since: Some(
                    DateTime::parse_from_rfc3339("2026-02-02T00:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                ),
                until: None,
                last_days: None,
            },
            cwd: PathBuf::from("/p/b"),
            limit: 200,
            id: None,
        };

        let filtered = filter_sessions_v1(&sessions, &query);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id.0, "b");
    }

    #[test]
    fn bucketing_week_starts_monday() {
        // 2026-02-24 is Tuesday; bucket start should be 2026-02-23 (Monday) in local time.
        let end_ts = DateTime::parse_from_rfc3339("2026-02-24T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = bucket_start_date_local(end_ts, GroupBy::Week);
        // Convert the computed local bucket start back to local to assert weekday.
        let local_midnight = Local
            .from_local_datetime(
                &start
                    .and_hms_opt(0, 0, 0)
                    .expect("valid midnight NaiveDateTime"),
            )
            .single()
            .expect("local time");
        assert_eq!(local_midnight.weekday(), Weekday::Mon);
    }
}
