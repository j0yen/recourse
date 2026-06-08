use crate::receipt::{canon, schema, store, types::GuardVerdict};
use crate::receipt::types::Receipt;
use std::io::{self, Read};
use std::path::PathBuf;

pub fn cmd_emit(store_raw: bool, data_dir_override: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Read stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    // Parse as GuardVerdict — malformed JSON or missing fields → error, no partial write
    let verdict: GuardVerdict = serde_json::from_str(&input)
        .map_err(|e| format!("malformed verdict JSON: {e}"))?;

    // Validate schema
    schema::validate(&verdict)
        .map_err(|e| format!("schema violation: {e}"))?;

    // Extract action before consuming verdict
    let action_value = verdict.action.clone();

    // Canonicalize action and compute digest
    let canonical_action = canon::canonicalize(&action_value);
    let action_digest = canon::digest(&canonical_action);

    // Build receipt
    let receipt = Receipt::new(
        action_digest.clone(),
        verdict.verdict.clone(),
        verdict.fired_rule.clone().unwrap_or_else(|| "none".to_string()),
        verdict.tenet.clone().unwrap_or_else(|| "none".to_string()),
        verdict.axiom_chain.clone().unwrap_or_default(),
        verdict.ontology_version.clone().unwrap_or_else(|| "unknown".to_string()),
        verdict.guard_version.clone().unwrap_or_else(|| "unknown".to_string()),
        verdict.installation_id.clone().unwrap_or_else(|| "unknown".to_string()),
    );

    let base = store::data_dir(data_dir_override.as_deref());

    // Append receipt (all-or-nothing write; if this fails, no partial line)
    store::append_receipt(&base, &receipt)?;

    // Optionally store raw action
    if store_raw {
        store::store_raw_action(&base, &action_digest, &canonical_action)?;
    }

    println!("receipt {} written", receipt.receipt_id);
    Ok(())
}
