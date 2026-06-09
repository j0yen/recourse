//! Parse a `--since` duration string like "30d", "7d", "24h", "1h" into a cutoff timestamp.

use chrono::{DateTime, Duration, Utc};

/// Parse a duration string like "30d", "7d", "24h" into a UTC cutoff timestamp
/// (i.e. `now - duration`).
pub fn parse_since(s: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let (n, unit) = split_duration(s)?;
    let dur = match unit {
        "d" => Duration::days(n),
        "h" => Duration::hours(n),
        "m" => Duration::minutes(n),
        other => {
            return Err(format!(
                "unknown duration unit '{other}'; use d (days), h (hours), or m (minutes)"
            )
            .into())
        }
    };
    Ok(Utc::now() - dur)
}

fn split_duration(s: &str) -> Result<(i64, &str), Box<dyn std::error::Error>> {
    // Find the boundary between digits and the unit suffix
    let idx = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| format!("missing unit in duration '{s}'; expected e.g. 30d"))?;
    let num: i64 = s[..idx]
        .parse()
        .map_err(|_| format!("cannot parse number in duration '{s}'"))?;
    Ok((num, &s[idx..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_days() {
        let cutoff = parse_since("30d").unwrap();
        let diff = Utc::now() - cutoff;
        // 30 days = 2_592_000s; allow ±5s slop for test execution time
        let expected = 30i64 * 24 * 3600;
        assert!(
            (diff.num_seconds() - expected).abs() < 5,
            "expected ~30d ({expected}s), got {}s",
            diff.num_seconds()
        );
    }

    #[test]
    fn parse_hours() {
        let cutoff = parse_since("24h").unwrap();
        let diff = Utc::now() - cutoff;
        let expected = 24i64 * 3600;
        assert!(
            (diff.num_seconds() - expected).abs() < 5,
            "expected ~24h ({expected}s), got {}s",
            diff.num_seconds()
        );
    }

    #[test]
    fn parse_bad_unit() {
        assert!(parse_since("30x").is_err());
    }

    #[test]
    fn parse_missing_unit() {
        assert!(parse_since("30").is_err());
    }
}
