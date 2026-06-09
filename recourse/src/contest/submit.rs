//! `recourse contest <receipt-id> --expected <verdict> --reason "<text>"`

use crate::contest::store::ContestStore;
use crate::contest::types::Contest;
use crate::receipt::store as receipt_store;
use std::path::PathBuf;

pub fn cmd_contest_submit(
    receipt_id: &str,
    expected: &str,
    reason: &str,
    data_dir: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate reason is non-empty
    if reason.trim().is_empty() {
        return Err("--reason must be non-empty".into());
    }

    let data = receipt_store::data_dir(data_dir.as_deref());
    let receipt = receipt_store::find_receipt(&data, receipt_id)?
        .ok_or_else(|| format!("receipt {receipt_id} not found"))?;

    // Refuse if expected equals the receipt's actual verdict (nothing to contest)
    if expected == receipt.verdict {
        return Err(format!(
            "nothing to contest: receipt {} already has verdict '{}'",
            receipt_id, expected
        )
        .into());
    }

    // Validate expected is a known verdict
    match expected {
        "allow" | "flag" | "deny" => {}
        other => {
            return Err(format!(
                "unknown verdict '{}'; expected one of: allow, flag, deny",
                other
            )
            .into())
        }
    }

    let installation_id = receipt.installation_id.clone();
    let contest = Contest::new_pending(
        receipt_id.to_string(),
        expected.to_string(),
        receipt.verdict.clone(),
        reason.to_string(),
        installation_id,
    );

    let store = ContestStore::new(&data);
    store.append_pending(&contest)?;

    println!("{}", contest.contest_id);
    Ok(())
}
