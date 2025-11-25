/// Returns a prettified display name from a bazaar product id.
pub fn pretty_name(id: &str) -> String {
    let mut parts = id.split(':');
    let base = parts
        .next()
        .unwrap_or_default()
        .split('_')
        .map(|w| {
            let lc = w.to_ascii_lowercase();
            let mut c = lc.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    match parts.next() {
        Some(tail) if !tail.is_empty() => format!("{base} ({tail})"),
        _ => base,
    }
}

/// Normalizes a string for fuzzy matching and indexing.
pub fn normalize(s: &str) -> String {
    s.to_ascii_lowercase()
        .replace(['_', ':'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}


