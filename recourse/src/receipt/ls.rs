use crate::receipt::store;
use crate::receipt::types::Receipt;
use chrono::{Duration, Utc};
use std::path::PathBuf;

pub fn cmd_ls(
    since: Option<&str>,
    verdict_filter: Option<&str>,
    format: &str,
    data_dir_override: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = store::data_dir(data_dir_override.as_deref());
    let mut receipts = store::load_all_receipts(&base)?;

    // Filter by --since (e.g. "30d")
    if let Some(since_str) = since {
        let days = parse_duration_days(since_str)?;
        let cutoff = Utc::now() - Duration::days(days);
        receipts.retain(|r| r.ts >= cutoff);
    }

    // Filter by --verdict
    if let Some(v) = verdict_filter {
        receipts.retain(|r| r.verdict == v);
    }

    // Already sorted newest-first by store::load_all_receipts

    match format {
        "json" => {
            println!("{}", serde_json::to_string(&receipts)?);
        }
        _ => {
            for r in &receipts {
                println!(
                    "{}\t{}\t{}\t{}",
                    r.receipt_id, r.ts.to_rfc3339(), r.verdict, r.fired_rule
                );
            }
        }
    }
    Ok(())
}

fn parse_duration_days(s: &str) -> Result<i64, Box<dyn std::error::Error>> {
    if let Some(stripped) = s.strip_suffix('d') {
        let days: i64 = stripped.parse().map_err(|_| format!("invalid duration: {s}"))?;
        Ok(days)
    } else if let Some(stripped) = s.strip_suffix('h') {
        let hours: i64 = stripped.parse().map_err(|_| format!("invalid duration: {s}"))?;
        Ok(hours / 24)
    } else {
        Err(format!("unsupported duration format: {s} (use Nd or Nh)").into())
    }
}

/// Helper used in tests
#[cfg(test)]
pub fn filter_receipts(receipts: Vec<Receipt>, since_days: Option<i64>, verdict: Option<&str>) -> Vec<Receipt> {
    let mut out = receipts;
    if let Some(days) = since_days {
        let cutoff = Utc::now() - Duration::days(days);
        out.retain(|r| r.ts >= cutoff);
    }
    if let Some(v) = verdict {
        out.retain(|r| r.verdict == v);
    }
    out
}
