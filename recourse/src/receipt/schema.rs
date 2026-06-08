use crate::receipt::types::GuardVerdict;

/// Validate that a parsed GuardVerdict is structurally sound.
/// Returns an error string if validation fails.
pub fn validate(v: &GuardVerdict) -> Result<(), String> {
    // verdict must be one of the three known values
    match v.verdict.as_str() {
        "allow" | "flag" | "deny" => {}
        other => {
            return Err(format!(
                "invalid verdict {:?}: must be allow, flag, or deny",
                other
            ))
        }
    }

    // action must not be null (we need to hash it)
    if v.action.is_null() {
        return Err("verdict.action must not be null".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_verdict(verdict: &str) -> GuardVerdict {
        serde_json::from_value(json!({
            "verdict": verdict,
            "action": {"intent": "test"},
            "fired_rule": null,
            "tenet": null,
            "axiom_chain": null,
            "ontology_version": null,
            "guard_version": null,
            "installation_id": null
        }))
        .unwrap()
    }

    #[test]
    fn valid_verdicts_pass() {
        for v in &["allow", "flag", "deny"] {
            assert!(validate(&make_verdict(v)).is_ok());
        }
    }

    #[test]
    fn invalid_verdict_rejected() {
        let v = make_verdict("APPROVED");
        assert!(validate(&v).is_err());
    }
}
