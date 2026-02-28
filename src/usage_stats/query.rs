use crate::usage_stats::model::SessionId;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ViewKind {
    #[default]
    Summary,
    Trend,
    Sessions,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum GroupBy {
    #[default]
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeRangeMode {
    All,
    SinceUntil,
    LastDays,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub mode: TimeRangeMode,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub last_days: Option<u32>,
}

impl TimeRange {
    pub fn all() -> Self {
        Self {
            mode: TimeRangeMode::All,
            since: None,
            until: None,
            last_days: None,
        }
    }
}

#[derive(Args, Debug, Clone, Default)]
pub struct TimeRangeArgs {
    /// Include sessions ending at or after this time (RFC3339 or YYYY-MM-DD local).
    #[arg(long)]
    pub since: Option<String>,
    /// Include sessions ending before this time (RFC3339 or YYYY-MM-DD local; date-only is exclusive next-day start).
    #[arg(long)]
    pub until: Option<String>,
    /// Convenience range: last <Nd> days (mutually exclusive with --since/--until).
    #[arg(long)]
    pub last: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct StatsCliArgs {
    /// Which report view to render.
    #[arg(long, value_enum, default_value_t = ViewKind::Summary)]
    pub view: ViewKind,

    /// Grouping granularity for the trend view.
    #[arg(long, value_enum, default_value_t = GroupBy::Day)]
    pub group_by: GroupBy,

    #[command(flatten)]
    pub range: TimeRangeArgs,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Open interactive TUI (ratatui).
    #[arg(long)]
    pub tui: bool,

    /// Show full absolute paths in table output.
    #[arg(long)]
    pub verbose: bool,

    /// Limit number of sessions in the sessions view (default 200; 0 = unlimited).
    #[arg(long, default_value_t = 200)]
    pub limit: usize,

    /// Session/thread id for the session view (required when not using --tui).
    #[arg(long)]
    pub id: Option<String>,
}

pub fn validate_stats_cli_args(args: &StatsCliArgs) -> Result<()> {
    if args.view == ViewKind::Session && !args.tui && args.id.is_none() {
        bail!("--id is required when --view session without --tui");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsQuery {
    pub view: ViewKind,
    pub group_by: GroupBy,
    pub time_range: TimeRange,
    pub cwd: PathBuf,
    pub limit: usize,
    pub id: Option<SessionId>,
}

enum BoundKind {
    Since,
    Until,
}

pub fn parse_time_range(args: &TimeRangeArgs, now_utc: DateTime<Utc>) -> Result<TimeRange> {
    if args.last.is_some() && (args.since.is_some() || args.until.is_some()) {
        bail!("--last is incompatible with --since/--until");
    }

    if let Some(raw) = args.last.as_deref() {
        let days = parse_last_days(raw)?;
        let now_local = now_utc.with_timezone(&Local);
        let since_local = now_local - Duration::days(i64::from(days));
        return Ok(TimeRange {
            mode: TimeRangeMode::LastDays,
            since: Some(since_local.with_timezone(&Utc)),
            until: Some(now_local.with_timezone(&Utc)),
            last_days: Some(days),
        });
    }

    let since = args
        .since
        .as_deref()
        .map(|raw| parse_time_bound(raw, BoundKind::Since))
        .transpose()?;

    let until = args
        .until
        .as_deref()
        .map(|raw| parse_time_bound(raw, BoundKind::Until))
        .transpose()?;

    if since.is_none() && until.is_none() {
        return Ok(TimeRange::all());
    }

    Ok(TimeRange {
        mode: TimeRangeMode::SinceUntil,
        since,
        until,
        last_days: None,
    })
}

fn parse_last_days(raw: &str) -> Result<u32> {
    let trimmed = raw.trim();
    let Some(days_str) = trimmed
        .strip_suffix('d')
        .or_else(|| trimmed.strip_suffix('D'))
    else {
        bail!("--last must use the 'd' unit, e.g. 7d");
    };

    let days: u32 = days_str
        .parse()
        .with_context(|| format!("invalid --last value: {raw}"))?;
    if days == 0 {
        bail!("--last must be a positive integer (days)");
    }
    Ok(days)
}

fn parse_time_bound(raw: &str, kind: BoundKind) -> Result<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.with_timezone(&Utc));
    }

    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .with_context(|| format!("invalid time value: {raw}"))?;

    let date = match kind {
        BoundKind::Since => date,
        BoundKind::Until => date
            .succ_opt()
            .with_context(|| format!("invalid --until date: {raw}"))?,
    };

    let naive = date
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight NaiveDateTime");

    let local = match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(earliest, _) => earliest,
        chrono::LocalResult::None => {
            bail!("local time does not exist for date input: {raw}");
        }
    };

    Ok(local.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use chrono::Timelike;

    #[test]
    fn parse_last_days_requires_unit_d() {
        assert!(parse_last_days("7").is_err());
        assert!(parse_last_days("7h").is_err());
        assert!(parse_last_days("0d").is_err());
        assert_eq!(parse_last_days("7d").unwrap(), 7);
        assert_eq!(parse_last_days("30D").unwrap(), 30);
    }

    #[test]
    fn parse_time_range_last_conflicts_with_since() {
        let args = TimeRangeArgs {
            last: Some("7d".to_string()),
            since: Some("2026-02-01".to_string()),
            until: None,
        };
        assert!(parse_time_range(&args, Utc::now()).is_err());
    }

    #[test]
    fn parse_time_range_rfc3339_since() {
        let args = TimeRangeArgs {
            since: Some("2026-02-01T00:00:00Z".to_string()),
            until: None,
            last: None,
        };
        let range = parse_time_range(&args, Utc::now()).unwrap();
        assert_eq!(range.mode, TimeRangeMode::SinceUntil);
        assert_eq!(
            range.since.unwrap(),
            DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );
    }

    #[test]
    fn parse_time_range_date_only_rules() {
        let args = TimeRangeArgs {
            since: Some("2026-02-01".to_string()),
            until: Some("2026-02-01".to_string()),
            last: None,
        };
        let range = parse_time_range(&args, Utc::now()).unwrap();

        let since_local = range.since.unwrap().with_timezone(&Local);
        assert_eq!(
            (since_local.year(), since_local.month(), since_local.day()),
            (2026, 2, 1)
        );
        assert_eq!(
            (
                since_local.hour(),
                since_local.minute(),
                since_local.second()
            ),
            (0, 0, 0)
        );

        let until_local = range.until.unwrap().with_timezone(&Local);
        assert_eq!(
            (until_local.year(), until_local.month(), until_local.day()),
            (2026, 2, 2)
        );
        assert_eq!(
            (
                until_local.hour(),
                until_local.minute(),
                until_local.second()
            ),
            (0, 0, 0)
        );
    }
}
