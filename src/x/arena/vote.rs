use crate::x::arena::generate::{
    ApplyRecord, GenerationRecord, MatchRecord, MatchSide, VerificationRecord,
};
use crate::x::arena::jsonl;
use crate::x::arena::paths::ArenaPaths;
use anyhow::{Result, anyhow};
use clap::Args;
use inquire::Select;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Args, Debug, Clone)]
pub struct VoteArgs {
    #[arg(long)]
    pub run: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteWinner {
    A,
    B,
    Tie,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    pub match_id: String,
    pub winner: VoteWinner,
    pub ts_ms: u64,
}

pub fn run(args: &VoteArgs) -> Result<()> {
    let paths = ArenaPaths::resolve()?;
    let run_dir = paths.run_dir(&args.run);
    if !run_dir.exists() {
        return Err(anyhow!("Run not found: {}", run_dir.display()));
    }

    let matches_path = run_dir.join("matches.jsonl");
    let gens_path = run_dir.join("generations.jsonl");
    let applies_path = run_dir.join("applies.jsonl");
    let verifs_path = run_dir.join("verifications.jsonl");
    let votes_path = run_dir.join("votes.jsonl");

    let matches = jsonl::read_lines::<MatchRecord>(&matches_path)?;
    let generations = jsonl::read_lines::<GenerationRecord>(&gens_path)?;
    let applies = if applies_path.exists() {
        jsonl::read_lines::<ApplyRecord>(&applies_path)?
    } else {
        Vec::new()
    };
    let verifs = if verifs_path.exists() {
        jsonl::read_lines::<VerificationRecord>(&verifs_path)?
    } else {
        Vec::new()
    };

    let voted = load_voted_match_ids(&votes_path)?;

    let gen_map = index_generations(&generations);
    let apply_map = index_applies(&applies);
    let verif_map = index_verifs(&verifs);

    let mut writer = jsonl::open_append_writer(&votes_path)?;
    let mut added = 0usize;

    for m in matches {
        if voted.contains(&m.match_id) {
            continue;
        }
        println!("\n=== match {} ===", m.match_id);
        println!("task: {} ({:?})", m.task.id, m.task.kind);
        println!("\nPROMPT:\n{}\n", m.task.prompt);
        if let Some(rubric) = &m.task.rubric {
            println!("RUBRIC:\n{}\n", rubric);
        }

        if m.task.kind == crate::x::arena::dataset::TaskKind::Repo {
            print_repo_summary(&m.match_id, &apply_map, &verif_map);
        }

        let out_a = gen_map
            .get(&(m.match_id.clone(), MatchSide::A))
            .map(|g| g.output.as_str())
            .unwrap_or("<missing>");
        let out_b = gen_map
            .get(&(m.match_id.clone(), MatchSide::B))
            .map(|g| g.output.as_str())
            .unwrap_or("<missing>");

        println!("--- A ({}) ---\n{}\n", m.contestant_a.id, out_a);
        println!("--- B ({}) ---\n{}\n", m.contestant_b.id, out_b);

        let choice = Select::new(
            "Pick winner:",
            vec!["A wins", "B wins", "Tie", "Skip", "Quit"],
        )
        .prompt()?;

        if choice == "Quit" {
            break;
        }

        let winner = match choice {
            "A wins" => VoteWinner::A,
            "B wins" => VoteWinner::B,
            "Tie" => VoteWinner::Tie,
            _ => VoteWinner::Skip,
        };

        let vote = VoteRecord {
            match_id: m.match_id,
            winner,
            ts_ms: now_ms(),
        };
        jsonl::write_line(&mut writer, &vote)?;
        writer.flush()?;
        added += 1;
    }

    println!("✅ Votes recorded: {added}");
    Ok(())
}

fn now_ms() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_millis() as u64
}

fn load_voted_match_ids(path: &Path) -> Result<HashSet<String>> {
    if !path.exists() {
        return Ok(HashSet::new());
    }
    let votes = jsonl::read_lines::<VoteRecord>(path)?;
    Ok(votes.into_iter().map(|v| v.match_id).collect())
}

fn index_generations(gens: &[GenerationRecord]) -> HashMap<(String, MatchSide), GenerationRecord> {
    let mut map = HashMap::new();
    for g in gens {
        map.insert((g.match_id.clone(), g.side), g.clone());
    }
    map
}

fn index_applies(applies: &[ApplyRecord]) -> HashMap<(String, MatchSide), ApplyRecord> {
    let mut map = HashMap::new();
    for a in applies {
        map.insert((a.match_id.clone(), a.side), a.clone());
    }
    map
}

fn index_verifs(
    verifs: &[VerificationRecord],
) -> HashMap<(String, MatchSide), Vec<VerificationRecord>> {
    let mut map: HashMap<(String, MatchSide), Vec<VerificationRecord>> = HashMap::new();
    for v in verifs {
        map.entry((v.match_id.clone(), v.side))
            .or_default()
            .push(v.clone());
    }
    map
}

fn print_repo_summary(
    match_id: &str,
    applies: &HashMap<(String, MatchSide), ApplyRecord>,
    verifs: &HashMap<(String, MatchSide), Vec<VerificationRecord>>,
) {
    for side in [MatchSide::A, MatchSide::B] {
        let apply = applies
            .get(&(match_id.to_string(), side))
            .map(|a| a.ok)
            .unwrap_or(false);
        println!("repo summary {side:?}: apply_ok={apply}");
        if let Some(list) = verifs.get(&(match_id.to_string(), side)) {
            for v in list {
                println!(
                    "  verify: {} status={:?} exit={:?}",
                    v.command, v.status, v.exit_code
                );
            }
        }
    }
    println!();
}
