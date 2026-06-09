//! `recourse contest ls [--pending|--all] [--format json|pretty]`

use crate::contest::store::ContestStore;
use crate::contest::types::ContestLsRow;
use crate::receipt::store as receipt_store;
use std::io::Write;
use std::path::PathBuf;

pub fn cmd_contest_ls(
    filter: &str,  // "pending" or "all"
    format: &str,  // "pretty" or "json"
    data_dir: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = receipt_store::data_dir(data_dir.as_deref());
    let store = ContestStore::new(&data);

    let contests = if filter == "all" {
        store.load_all()?
    } else {
        store.load_pending()?
    };

    // Join each contest with its receipt for context
    let mut rows: Vec<ContestLsRow> = Vec::new();
    for c in &contests {
        let fired_rule = match receipt_store::find_receipt(&data, &c.receipt_id)? {
            Some(r) => r.fired_rule,
            None => "(receipt not found)".to_string(),
        };
        rows.push(ContestLsRow {
            contest_id: c.contest_id.clone(),
            receipt_id: c.receipt_id.clone(),
            ts: c.ts,
            status: c.status.clone(),
            claimed_verdict: c.claimed_verdict.clone(),
            observed_verdict: c.observed_verdict.clone(),
            fired_rule,
            reason: c.reason.clone(),
            contestant: c.contestant.clone(),
        });
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if format == "json" {
        writeln!(out, "{}", serde_json::to_string(&rows)?)?;
    } else {
        if rows.is_empty() {
            writeln!(out, "(no contests)")?;
            return Ok(());
        }
        for row in &rows {
            writeln!(
                out,
                "{} [{}] receipt={} observed={} claimed={} rule={} | {}",
                row.contest_id,
                row.status,
                row.receipt_id,
                row.observed_verdict,
                row.claimed_verdict,
                row.fired_rule,
                row.reason
            )?;
        }
    }

    Ok(())
}
