use crate::receipt::store;
use std::path::PathBuf;

pub fn cmd_show(id: &str, format: &str, data_dir_override: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let base = store::data_dir(data_dir_override.as_deref());
    let receipt = store::find_receipt(&base, id)?
        .ok_or_else(|| format!("receipt not found: {id}"))?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        _ => {
            // pretty-print
            println!("Receipt ID : {}", receipt.receipt_id);
            println!("Schema     : {}", receipt.schema);
            println!("Timestamp  : {}", receipt.ts.to_rfc3339());
            println!("Verdict    : {}", receipt.verdict);
            println!("Fired Rule : {}", receipt.fired_rule);
            println!("Tenet      : {}", receipt.tenet);
            println!("Digest     : {}", receipt.action_digest);
            println!("Ontology   : {}", receipt.ontology_version);
            println!("Guard      : {}", receipt.guard_version);
            println!("Install ID : {}", receipt.installation_id);
            if !receipt.axiom_chain.is_empty() {
                println!("Axiom chain:");
                for ax in &receipt.axiom_chain {
                    println!("  - {ax}");
                }
            }
        }
    }
    Ok(())
}
