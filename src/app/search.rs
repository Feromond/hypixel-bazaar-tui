pub const MIN_SCORE: i32 = i32::MIN / 2;

/// Fuzzy score assuming `query_norm` and `candidate_norm` are already normalized via `normalize`.
pub fn score_normalized(query_norm: &str, candidate_norm: &str) -> i32 {
    if query_norm.is_empty() || candidate_norm.is_empty() {
        return MIN_SCORE;
    }
    if query_norm == candidate_norm {
        return 500;
    }

    let mut score: i32 = 0;

    // Global prefix bonus
    if candidate_norm.starts_with(query_norm) {
        score += 120;
    }

    // Subsequence and adjacency bonuses
    if let Some(pos) = subsequence_positions(query_norm, candidate_norm) {
        score += 60;
        let streak = best_consecutive_streak(&pos);
        score += (streak as i32) * 6;
        let boundary_hits = boundary_hits(&pos, candidate_norm);
        score += (boundary_hits as i32) * 8;
    }

    // Token-level features
    let q_tokens = tokenize(query_norm);
    let c_tokens = tokenize(candidate_norm);
    if !q_tokens.is_empty() && !c_tokens.is_empty() {
        let exact = token_exact_matches(&q_tokens, &c_tokens);
        score += (exact as i32) * 40;
        let pref = token_prefix_matches(&q_tokens, &c_tokens);
        score += (pref as i32) * 24;
        let overlap = token_overlap_count(&q_tokens, &c_tokens);
        score += (overlap as i32) * 18;
    }

    // Acronym match (e.g., "eb" -> "enchanted book")
    if is_acronym_subsequence(query_norm, &c_tokens) {
        score += 45;
    }

    // Edit distance penalty (bounded)
    let d = bounded_lev(query_norm, candidate_norm, 3);
    score -= (d as i32) * 12;

    // Length proximity
    let len_diff = (candidate_norm.len() as i32 - query_norm.len() as i32).abs().min(12);
    score -= len_diff;

    score
}

fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    for ch in needle.chars() {
        if !it.by_ref().any(|c| c == ch) {
            return false;
        }
    }
    true
}

fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace().map(|t| t.to_string()).collect()
}

fn token_exact_matches(a: &[String], b: &[String]) -> usize {
    use std::collections::HashSet;
    let sa: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let sb: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    sa.intersection(&sb).count()
}

fn token_prefix_matches(a: &[String], b: &[String]) -> usize {
    let mut count = 0usize;
    for qa in a {
        if b.iter().any(|t| t.starts_with(qa)) {
            count += 1;
        }
    }
    count
}

fn token_overlap_count(a: &[String], b: &[String]) -> usize {
    token_exact_matches(a, b)
}

fn is_acronym_subsequence(query_norm: &str, c_tokens: &[String]) -> bool {
    if c_tokens.is_empty() || query_norm.len() > c_tokens.len() {
        return false;
    }
    let acronym: String = c_tokens
        .iter()
        .filter_map(|t| t.chars().next())
        .collect();
    is_subsequence(query_norm, &acronym)
}

fn subsequence_positions(needle: &str, hay: &str) -> Option<Vec<usize>> {
    let mut positions = Vec::with_capacity(needle.len());
    let mut hay_iter = hay.char_indices();
    let mut last_idx = 0usize;
    for ch in needle.chars() {
        let mut found = None;
        for (i, c) in hay_iter.by_ref() {
            if c == ch {
                found = Some(i);
                last_idx = i;
                break;
            }
        }
        if let Some(i) = found {
            positions.push(i);
        } else {
            return None;
        }
    }
    if positions.is_empty() {
        None
    } else {
        // Ensure strictly increasing
        if positions.windows(2).all(|w| w[0] < w[1]) {
            Some(positions)
        } else {
            // Fallback: monotonicity broken (shouldn't happen), ignore
            Some(vec![last_idx])
        }
    }
}

fn best_consecutive_streak(positions: &[usize]) -> usize {
    if positions.is_empty() {
        return 0;
    }
    let mut best = 1usize;
    let mut cur = 1usize;
    for w in positions.windows(2) {
        if w[1] == w[0] + 1 {
            cur += 1;
            if cur > best {
                best = cur;
            }
        } else {
            cur = 1;
        }
    }
    best
}

fn boundary_hits(positions: &[usize], hay: &str) -> usize {
    if positions.is_empty() {
        return 0;
    }
    let mut boundaries = std::collections::HashSet::new();
    boundaries.insert(0usize);
    for (i, ch) in hay.char_indices() {
        if ch == ' ' {
            // next character after space is a boundary
            if let Some((next_i, _)) = hay[i + ch.len_utf8()..].char_indices().next() {
                boundaries.insert(i + ch.len_utf8() + next_i);
            }
        }
    }
    positions.iter().filter(|p| boundaries.contains(p)).count()
}

/// Bounded Levenshtein: early exit if distance > bound
fn bounded_lev(a: &str, b: &str, bound: usize) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m.min(bound + 1);
    }
    if m == 0 {
        return n.min(bound + 1);
    }

    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr = vec![0; m + 1];

    for i in 1..=n {
        curr[0] = i;
        let mut row_min = curr[0];

        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            if curr[j] < row_min {
                row_min = curr[j];
            }
        }

        if row_min > bound {
            return bound + 1;
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m].min(bound + 1)
}
