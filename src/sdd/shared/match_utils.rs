/// Outcome of prefix-based id resolution shared by `resolve_change_id`
/// (discovery.rs) and `resolve_target` (status.rs).
///
/// All comparisons are case-sensitive to keep change-id resolution
/// deterministic across commands (see cli spec r112).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefixOutcome<'a> {
    /// Exactly one candidate matched (exact or unique prefix).
    Single(&'a str),
    /// Multiple candidates matched by prefix; caller decides whether that is
    /// an error (discovery) or a legitimate multi-match (status).
    Multiple(Vec<&'a str>),
    /// No candidate matched.
    None,
}

/// Resolve `input` against a slice of candidate ids using the shared priority
/// chain: exact match > unique prefix match > multi prefix > no match.
///
/// This is the single source of truth for the "exact > prefix" rule mandated
/// by cli spec r112. Both `resolve_change_id` and `resolve_target` delegate
/// here so the rule cannot diverge between commands.
pub fn prefix_resolve<'a>(input: &str, candidates: &'a [String]) -> PrefixOutcome<'a> {
    if input.is_empty() || candidates.is_empty() {
        return PrefixOutcome::None;
    }

    // 1) Exact match
    if let Some(c) = candidates.iter().find(|c| c.as_str() == input) {
        return PrefixOutcome::Single(c);
    }

    // 2) Prefix match
    let prefix_matches: Vec<&str> = candidates
        .iter()
        .map(String::as_str)
        .filter(|c| c.starts_with(input))
        .collect();

    match prefix_matches.len() {
        0 => PrefixOutcome::None,
        1 => PrefixOutcome::Single(prefix_matches[0]),
        _ => PrefixOutcome::Multiple(prefix_matches),
    }
}

pub fn nearest_matches(needle: &str, candidates: &[String], limit: usize) -> Vec<String> {
    if needle.is_empty() || candidates.is_empty() || limit == 0 {
        return Vec::new();
    }

    let threshold = std::cmp::max(2, needle.len() / 2);
    let mut scored: Vec<(usize, &String)> = candidates
        .iter()
        .map(|candidate| (levenshtein(needle, candidate), candidate))
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));

    scored
        .into_iter()
        .filter(|(distance, _)| *distance <= threshold)
        .take(limit)
        .map(|(_, candidate)| candidate.clone())
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut prev: Vec<usize> = (0..=b_bytes.len()).collect();
    for (i, &a_ch) in a_bytes.iter().enumerate() {
        let mut current = Vec::with_capacity(b_bytes.len() + 1);
        current.push(i + 1);
        for (j, &b_ch) in b_bytes.iter().enumerate() {
            let cost = if a_ch == b_ch { 0 } else { 1 };
            let insert = current[j] + 1;
            let delete = prev[j + 1] + 1;
            let replace = prev[j] + cost;
            current.push(insert.min(delete).min(replace));
        }
        prev = current;
    }

    prev[b_bytes.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prefix_resolve_exact_match_wins_over_prefix() {
        let candidates = ids(&["c123-foo", "c123", "c456"]);
        // Exact "c123" must win even though "c123-foo" also starts with "c123".
        assert_eq!(
            prefix_resolve("c123", &candidates),
            PrefixOutcome::Single("c123")
        );
    }

    #[test]
    fn prefix_resolve_unique_prefix() {
        let candidates = ids(&["c123-foo", "c456-bar"]);
        assert_eq!(
            prefix_resolve("c123", &candidates),
            PrefixOutcome::Single("c123-foo")
        );
    }

    #[test]
    fn prefix_resolve_multiple_prefix_matches() {
        let candidates = ids(&["c123-foo", "c123-bar", "c456"]);
        let outcome = prefix_resolve("c123", &candidates);
        match outcome {
            PrefixOutcome::Multiple(matches) => {
                // Order follows the candidate slice (callers sort if needed).
                assert_eq!(matches, vec!["c123-foo", "c123-bar"]);
            }
            _ => panic!("expected Multiple, got {outcome:?}"),
        }
    }

    #[test]
    fn prefix_resolve_no_match() {
        let candidates = ids(&["c123-foo", "c456-bar"]);
        assert_eq!(prefix_resolve("zzz", &candidates), PrefixOutcome::None);
    }

    #[test]
    fn prefix_resolve_case_sensitive() {
        // Per cli spec r112, resolution is deterministic and case-sensitive.
        let candidates = ids(&["C123-foo"]);
        assert_eq!(prefix_resolve("c123", &candidates), PrefixOutcome::None);
        assert_eq!(
            prefix_resolve("C123", &candidates),
            PrefixOutcome::Single("C123-foo")
        );
    }

    #[test]
    fn prefix_resolve_empty_input_or_candidates() {
        assert_eq!(prefix_resolve("", &ids(&["c123"])), PrefixOutcome::None);
        assert_eq!(prefix_resolve("c123", &[]), PrefixOutcome::None);
    }
}
