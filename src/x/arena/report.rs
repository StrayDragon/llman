use crate::x::arena::elo::{DEFAULT_INITIAL_RATING, DEFAULT_K, ensure_rating, update};
use crate::x::arena::generate::MatchRecord;
use crate::x::arena::jsonl;
use crate::x::arena::paths::ArenaPaths;
use crate::x::arena::vote::{VoteRecord, VoteWinner};
use anyhow::{Result, anyhow};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Args, Debug, Clone)]
pub struct ReportArgs {
    #[arg(long)]
    pub run: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingRow {
    pub contestant_id: String,
    pub rating: f64,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    pub skips: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingsReport {
    pub k: f64,
    pub initial: f64,
    pub ratings: Vec<RatingRow>,
}

pub fn run(args: &ReportArgs) -> Result<()> {
    let paths = ArenaPaths::resolve()?;
    let run_dir = paths.run_dir(&args.run);
    if !run_dir.exists() {
        return Err(anyhow!("Run not found: {}", run_dir.display()));
    }

    let matches_path = run_dir.join("matches.jsonl");
    let votes_path = run_dir.join("votes.jsonl");
    if !votes_path.exists() {
        return Err(anyhow!(
            "At least one vote is required to compute ratings (run: {})",
            args.run
        ));
    }

    let matches = jsonl::read_lines::<MatchRecord>(&matches_path)?;
    let votes = jsonl::read_lines::<VoteRecord>(&votes_path)?;
    if votes.is_empty() {
        return Err(anyhow!(
            "At least one vote is required to compute ratings (run: {})",
            args.run
        ));
    }

    let match_map = matches
        .into_iter()
        .map(|m| (m.match_id.clone(), (m.contestant_a.id, m.contestant_b.id)))
        .collect::<HashMap<_, _>>();

    let mut ratings: HashMap<String, f64> = HashMap::new();
    let mut stats: HashMap<String, RatingRow> = HashMap::new();

    for vote in votes {
        let Some((a_id, b_id)) = match_map.get(&vote.match_id).cloned() else {
            continue;
        };

        ensure_rating(&mut ratings, &a_id);
        ensure_rating(&mut ratings, &b_id);

        let score_a = match vote.winner {
            VoteWinner::A => 1.0,
            VoteWinner::B => 0.0,
            VoteWinner::Tie => 0.5,
            VoteWinner::Skip => {
                update_stats(&mut stats, &a_id, &b_id, vote.winner);
                continue;
            }
        };

        let ra = ratings[&a_id];
        let rb = ratings[&b_id];
        let (ra2, rb2) = update(ra, rb, score_a, DEFAULT_K);
        ratings.insert(a_id.clone(), ra2);
        ratings.insert(b_id.clone(), rb2);

        update_stats(&mut stats, &a_id, &b_id, vote.winner);
    }

    let mut rows = Vec::new();
    for (id, rating) in ratings {
        let mut row = stats.remove(&id).unwrap_or_else(|| RatingRow {
            contestant_id: id.clone(),
            rating: DEFAULT_INITIAL_RATING,
            games: 0,
            wins: 0,
            losses: 0,
            ties: 0,
            skips: 0,
        });
        row.rating = rating;
        rows.push(row);
    }
    rows.sort_by(|a, b| b.rating.total_cmp(&a.rating));

    let out = RatingsReport {
        k: DEFAULT_K,
        initial: DEFAULT_INITIAL_RATING,
        ratings: rows.clone(),
    };

    let ratings_path = run_dir.join("ratings.json");
    fs::write(&ratings_path, serde_json::to_string_pretty(&out)?)?;

    let report_md = render_report_md(&rows);
    let report_path = run_dir.join("report.md");
    fs::write(&report_path, report_md)?;

    println!("✅ Wrote ratings: {}", ratings_path.display());
    println!("✅ Wrote report:  {}", report_path.display());
    println!("\nLeaderboard:");
    for (i, row) in rows.iter().take(20).enumerate() {
        println!(
            "{:>2}. {:<30} {:>7.1} (W{} L{} T{} G{})",
            i + 1,
            row.contestant_id,
            row.rating,
            row.wins,
            row.losses,
            row.ties,
            row.games
        );
    }

    Ok(())
}

fn update_stats(stats: &mut HashMap<String, RatingRow>, a: &str, b: &str, winner: VoteWinner) {
    let mut row_a = stats.remove(a).unwrap_or_else(|| new_row(a));
    let mut row_b = stats.remove(b).unwrap_or_else(|| new_row(b));
    row_a.games += 1;
    row_b.games += 1;
    match winner {
        VoteWinner::A => {
            row_a.wins += 1;
            row_b.losses += 1;
        }
        VoteWinner::B => {
            row_b.wins += 1;
            row_a.losses += 1;
        }
        VoteWinner::Tie => {
            row_a.ties += 1;
            row_b.ties += 1;
        }
        VoteWinner::Skip => {
            row_a.skips += 1;
            row_b.skips += 1;
        }
    }
    stats.insert(a.to_string(), row_a);
    stats.insert(b.to_string(), row_b);
}

fn new_row(id: &str) -> RatingRow {
    RatingRow {
        contestant_id: id.to_string(),
        rating: DEFAULT_INITIAL_RATING,
        games: 0,
        wins: 0,
        losses: 0,
        ties: 0,
        skips: 0,
    }
}

fn render_report_md(rows: &[RatingRow]) -> String {
    let mut out = String::new();
    out.push_str("# Arena Elo Report\n\n");
    out.push_str("| Rank | Contestant | Rating | Games | W | L | T |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|\n");
    for (i, r) in rows.iter().enumerate() {
        out.push_str(&format!(
            "| {} | `{}` | {:.1} | {} | {} | {} | {} |\n",
            i + 1,
            r.contestant_id,
            r.rating,
            r.games,
            r.wins,
            r.losses,
            r.ties
        ));
    }
    out
}
