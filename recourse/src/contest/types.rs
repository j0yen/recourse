//! Contest record types for recourse.contest.v1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// One record in the pending/upheld/rejected contest NDJSON files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contest {
    pub schema: String,
    pub contest_id: String,
    pub receipt_id: String,
    pub ts: DateTime<Utc>,
    /// What the user believes the correct verdict should have been
    pub claimed_verdict: String,
    /// The verdict the engine emitted (copied from the receipt)
    pub observed_verdict: String,
    pub reason: String,
    /// Opaque installation/user id — no PII
    pub contestant: String,
    /// "pending" | "upheld" | "rejected"
    pub status: String,
}

impl Contest {
    pub fn new_pending(
        receipt_id: String,
        claimed_verdict: String,
        observed_verdict: String,
        reason: String,
        contestant: String,
    ) -> Self {
        Contest {
            schema: "recourse.contest.v1".to_string(),
            contest_id: Ulid::new().to_string(),
            receipt_id,
            ts: Utc::now(),
            claimed_verdict,
            observed_verdict,
            reason,
            contestant,
            status: "pending".to_string(),
        }
    }
}

/// A pending contest joined with its receipt context (for `contest ls`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestLsRow {
    pub contest_id: String,
    pub receipt_id: String,
    pub ts: DateTime<Utc>,
    pub status: String,
    pub claimed_verdict: String,
    pub observed_verdict: String,
    pub fired_rule: String,
    pub reason: String,
    pub contestant: String,
}
