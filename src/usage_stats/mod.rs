pub mod aggregate;
pub mod model;
pub mod path_display;
pub mod query;
pub mod render;
pub mod tui;

pub use aggregate::{
    Coverage, SessionDetailView, SessionsView, StatsBucket, SummaryView, TrendView,
    build_session_detail_view, build_sessions_view, build_summary_view, build_trend_view,
    filter_sessions_v1,
};
pub use model::{SessionId, SessionRecord, TokenUsage, ToolKind};
pub use query::{
    ColorMode, GroupBy, OutputFormat, StatsCliArgs, StatsQuery, TimeRange, TimeRangeArgs,
    TimeRangeMode, ViewKind, parse_time_range, validate_stats_cli_args,
};
pub use render::{
    RenderOptions, StatsJsonOutput, StatsViewResult, render_stats_json, render_stats_table,
};
pub use tui::{ScanMessage, ScanProgress, StatsTuiScanRequest, run_stats_tui};
