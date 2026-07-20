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

fn group_digits(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Formats a coin amount in full, grouped: `4.81`, `1,355.2`, `12,345,678`.
///
/// Decimals shrink as magnitude grows, since a fraction of a coin stops
/// mattering once the price is in the millions.
pub fn fmt_price(v: f64) -> String {
    if !v.is_finite() {
        return "-".to_string();
    }
    let sign = if v < 0.0 { "-" } else { "" };
    let a = v.abs();
    let decimals = if a >= 1e6 {
        0
    } else if a >= 1000.0 {
        1
    } else {
        2
    };

    let s = format!("{a:.decimals$}");
    match s.split_once('.') {
        Some((int_part, frac)) => format!("{sign}{}.{frac}", group_digits(int_part)),
        None => format!("{sign}{}", group_digits(&s)),
    }
}

/// Formats a whole-number count with separators.
pub fn fmt_count(n: i64) -> String {
    let sign = if n < 0 { "-" } else { "" };
    format!("{sign}{}", group_digits(&n.unsigned_abs().to_string()))
}

/// Abbreviates a count for narrow columns: `588k`, `4.7M`.
pub fn fmt_compact(n: i64) -> String {
    let sign = if n < 0 { "-" } else { "" };
    let a = n.unsigned_abs() as f64;
    if a >= 1e12 {
        format!("{sign}{:.1}T", a / 1e12)
    } else if a >= 1e9 {
        format!("{sign}{:.1}B", a / 1e9)
    } else if a >= 1e6 {
        format!("{sign}{:.1}M", a / 1e6)
    } else if a >= 1e3 {
        format!("{sign}{:.0}k", a / 1e3)
    } else {
        format!("{sign}{a:.0}")
    }
}

/// Formats a percentage with an explicit sign.
pub fn fmt_pct(v: f64) -> String {
    if v.is_finite() {
        format!("{v:+.2}%")
    } else {
        "-".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_keep_precision_where_coins_matter() {
        assert_eq!(fmt_price(4.81), "4.81");
        assert_eq!(fmt_price(0.0), "0.00");
        assert_eq!(fmt_price(999.994), "999.99");
    }

    #[test]
    fn prices_stay_exact_and_grouped() {
        assert_eq!(fmt_price(1355.2), "1,355.2");
        assert_eq!(fmt_price(999_999.0), "999,999.0");
        assert_eq!(fmt_price(12_345_678.0), "12,345,678");
        assert_eq!(fmt_price(4_000_000_000.0), "4,000,000,000");
        assert_eq!(fmt_price(1_234_567_890.0), "1,234,567,890");
    }

    #[test]
    fn negatives_keep_their_sign() {
        assert_eq!(fmt_price(-1355.2), "-1,355.2");
        assert_eq!(fmt_price(-97.3), "-97.30");
        assert_eq!(fmt_price(-12_345_678.0), "-12,345,678");
    }

    #[test]
    fn counts_are_grouped_exactly() {
        assert_eq!(fmt_count(0), "0");
        assert_eq!(fmt_count(999), "999");
        assert_eq!(fmt_count(1000), "1,000");
        assert_eq!(fmt_count(16_008_062), "16,008,062");
        assert_eq!(fmt_count(-1_234), "-1,234");
    }

    #[test]
    fn compact_counts_stay_narrow() {
        assert_eq!(fmt_compact(0), "0");
        assert_eq!(fmt_compact(999), "999");
        assert_eq!(fmt_compact(587_654), "588k");
        assert_eq!(fmt_compact(4_673_752), "4.7M");
        assert_eq!(fmt_compact(2_400_000_000), "2.4B");
        assert_eq!(fmt_compact(2_400_000_000_000), "2.4T");
    }

    #[test]
    fn compact_counts_fit_their_column() {
        for n in [0, 1, 999, 1_000, 999_999, 1e9 as i64, 999e12 as i64] {
            assert!(
                fmt_compact(n).len() <= 6,
                "{n} rendered as {:?}",
                fmt_compact(n)
            );
        }
        assert!(!fmt_compact(i64::MAX).is_empty());
        assert!(!fmt_compact(i64::MIN).is_empty());
    }

    #[test]
    fn non_finite_prices_degrade() {
        assert_eq!(fmt_price(f64::NAN), "-");
        assert_eq!(fmt_price(f64::INFINITY), "-");
        assert_eq!(fmt_pct(f64::NAN), "-");
    }
}

