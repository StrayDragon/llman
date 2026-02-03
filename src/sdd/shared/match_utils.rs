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
