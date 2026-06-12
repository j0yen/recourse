use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// The `ousia-guard` verdict shape we accept on stdin.
/// This is the checked-in wire contract — only fields we need.
#[derive(Debug, Deserialize)]
pub struct GuardVerdict {
    pub verdict: String,
    pub fired_rule: Option<String>,
    pub tenet: Option<String>,
    pub axiom_chain: Option<Vec<String>>,
    pub ontology_version: Option<String>,
    pub guard_version: Option<String>,
    pub installation_id: Option<String>,
    /// The raw action object (will be digested, never stored in receipt)
    pub action: serde_json::Value,
}

/// One line in the receipt NDJSON sink.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub schema: String,
    pub receipt_id: String,
    pub ts: DateTime<Utc>,
    pub action_digest: String,
    pub verdict: String,
    pub fired_rule: String,
    pub tenet: String,
    pub axiom_chain: Vec<String>,
    pub ontology_version: String,
    pub guard_version: String,
    pub installation_id: String,
}

impl Receipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action_digest: String,
        verdict: String,
        fired_rule: String,
        tenet: String,
        axiom_chain: Vec<String>,
        ontology_version: String,
        guard_version: String,
        installation_id: String,
    ) -> Self {
        Receipt {
            schema: "recourse.receipt.v1".to_string(),
            receipt_id: Ulid::new().to_string(),
            ts: Utc::now(),
            action_digest,
            verdict,
            fired_rule,
            tenet,
            axiom_chain,
            ontology_version,
            guard_version,
            installation_id,
        }
    }
}
