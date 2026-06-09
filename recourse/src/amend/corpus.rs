//! Corpus case writer — emits the three files for a tribunal-corpus case.

use super::contest::UpheldContest;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// `expected.toml` shape.
#[derive(Debug, Serialize)]
pub struct ExpectedToml {
    pub verdict: String,
    pub rule: String,
    pub tenet: String,
    pub rationale: String,
}

/// `provenance.toml` shape.
#[derive(Debug, Serialize)]
pub struct ProvenanceToml {
    pub source: String,
    pub source_ref: String,
    pub author: String,
    pub spot_checked_by: String,
}

/// Write the three corpus files into `<cases_dir>/field-<contest-id>/`.
/// Returns the path of the created case directory.
pub fn write_case(
    cases_dir: &Path,
    contest: &UpheldContest,
    action_json: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let case_dir = cases_dir.join(format!("field-{}", contest.contest_id));
    fs::create_dir_all(&case_dir)?;

    // action.json — the canonical action ABox
    // Validate that action_json parses as JSON before writing
    let action_value: serde_json::Value = serde_json::from_str(action_json)
        .map_err(|e| format!("action file is not valid JSON: {e}"))?;
    let action_canonical = serde_json::to_string_pretty(&action_value)?;
    fs::write(case_dir.join("action.json"), &action_canonical)?;

    // expected.toml
    let expected = ExpectedToml {
        // AC3: verdict must come from claimed_verdict (field's corrected answer)
        verdict: contest.claimed_verdict.clone(),
        rule: contest
            .rule
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        tenet: contest
            .tenet
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        rationale: contest.reason.clone(),
    };
    let expected_toml = toml::to_string_pretty(&expected)?;
    fs::write(case_dir.join("expected.toml"), &expected_toml)?;

    // provenance.toml
    // AC2 / independence guarantee:
    //   author = "downstream:<contestant>" — by construction != "ousia-axioms"
    let author = format!("downstream:{}", contest.contestant);
    // Paranoid assertion — this is the independence hinge
    assert_ne!(
        author, "ousia-axioms",
        "provenance.author must never equal ousia-axioms"
    );

    let provenance = ProvenanceToml {
        source: "field-contest".to_string(),
        source_ref: format!(
            "receipt:{} contest:{}",
            contest.receipt_id, contest.contest_id
        ),
        author,
        spot_checked_by: contest.reviewer.clone(),
    };
    let provenance_toml = toml::to_string_pretty(&provenance)?;
    fs::write(case_dir.join("provenance.toml"), &provenance_toml)?;

    Ok(case_dir)
}
