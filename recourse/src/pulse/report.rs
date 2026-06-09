//! Build the aggregate PulseReport from receipts + contests.
//!
//! Privacy invariant: PulseReport contains ONLY aggregates —
//! no receipt_id, no action_digest, no per-action rows.

use crate::receipt::types::Receipt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimal contest entry for pulse aggregation (no private fields from receipt).
#[derive(Debug, serde::Deserialize)]
pub struct ContestEntry {
    pub status: String,
    #[serde(default)]
    pub ts: Option<chrono::DateTime<chrono::Utc>>,
}

/// Aggregate verdict distribution entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerdictDist {
    pub verdict: String,
    pub count: u64,
    pub pct: f64,
}

/// Per-axiom/tenet fire count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AxiomFireCount {
    pub tenet: String,
    pub count: u64,
    /// True when count == 0: tenet was never fired in this window.
    pub zero_fired: bool,
}

/// Per-version verdict distribution slice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionSlice {
    pub ontology_version: String,
    pub verdict_distribution: Vec<VerdictDist>,
}

/// The complete pulse report — aggregates only, no per-receipt rows.
///
/// INVARIANT: this struct must never contain `receipt_id` or `action_digest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseReport {
    /// Human-readable description of the time window
    pub window: String,
    /// Total receipts in window
    pub total_receipts: u64,
    /// Verdict distribution (allow / flag / deny / …)
    pub verdict_distribution: Vec<VerdictDist>,
    /// Contest rate as percentage (contests / receipts * 100)
    pub contest_rate_pct: f64,
    /// Number of contests in window
    pub contest_count: u64,
    /// Number of reviewed contests (upheld + rejected)
    pub reviewed_count: u64,
    /// Number of upheld contests
    pub upheld_count: u64,
    /// Uphold rate as percentage (upheld / reviewed * 100)
    pub uphold_rate_pct: f64,
    /// Per-tenet fire counts, sorted desc by count; zero-fired tenets flagged
    pub axiom_fire_counts: Vec<AxiomFireCount>,
    /// Verdict distribution sliced by ontology_version
    pub version_drift: Vec<VersionSlice>,
}

/// Build a PulseReport from the loaded receipts and contests.
///
/// `cutoff`: if Some, only receipts at or after the cutoff are included.
/// Contest filtering uses the contest `ts` field if present; if absent, all
/// contests are included (conservative — we can't exclude what we can't date).
pub fn build_report(
    all_receipts: &[Receipt],
    all_contests: &[ContestEntry],
    cutoff: Option<DateTime<Utc>>,
) -> Result<PulseReport, Box<dyn std::error::Error>> {
    // Filter receipts by window
    let receipts: Vec<&Receipt> = match cutoff {
        Some(c) => all_receipts.iter().filter(|r| r.ts >= c).collect(),
        None => all_receipts.iter().collect(),
    };

    let window = match cutoff {
        Some(c) => {
            let dur = Utc::now() - c;
            let days = dur.num_days();
            if days > 0 {
                format!("last {days}d")
            } else {
                let hrs = dur.num_hours();
                format!("last {hrs}h")
            }
        }
        None => "all time".to_string(),
    };

    let total_receipts = receipts.len() as u64;

    // Verdict distribution
    let verdict_distribution = compute_verdict_dist(&receipts);

    // Contest stats
    let (contest_count, reviewed_count, upheld_count) =
        compute_contest_stats(all_contests, cutoff);
    let contest_rate_pct = if total_receipts == 0 {
        0.0
    } else {
        (contest_count as f64 / total_receipts as f64) * 100.0
    };
    let uphold_rate_pct = if reviewed_count == 0 {
        0.0
    } else {
        (upheld_count as f64 / reviewed_count as f64) * 100.0
    };

    // Per-axiom fire counts — collect all known tenets from receipts
    let axiom_fire_counts = compute_axiom_fire_counts(&receipts);

    // Version drift
    let version_drift = compute_version_drift(&receipts);

    Ok(PulseReport {
        window,
        total_receipts,
        verdict_distribution,
        contest_rate_pct,
        contest_count,
        reviewed_count,
        upheld_count,
        uphold_rate_pct,
        axiom_fire_counts,
        version_drift,
    })
}

fn compute_verdict_dist(receipts: &[&Receipt]) -> Vec<VerdictDist> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    for r in receipts {
        *counts.entry(r.verdict.clone()).or_insert(0) += 1;
    }
    let total = receipts.len() as f64;
    let mut dist: Vec<VerdictDist> = counts
        .into_iter()
        .map(|(verdict, count)| VerdictDist {
            pct: if total > 0.0 {
                (count as f64 / total) * 100.0
            } else {
                0.0
            },
            verdict,
            count,
        })
        .collect();
    // Sort by count desc, then by verdict name for determinism
    dist.sort_by(|a, b| b.count.cmp(&a.count).then(a.verdict.cmp(&b.verdict)));
    dist
}

fn compute_contest_stats(
    contests: &[ContestEntry],
    cutoff: Option<DateTime<Utc>>,
) -> (u64, u64, u64) {
    let filtered: Vec<&ContestEntry> = match cutoff {
        Some(c) => contests
            .iter()
            .filter(|ct| ct.ts.map(|t| t >= c).unwrap_or(true))
            .collect(),
        None => contests.iter().collect(),
    };

    let contest_count = filtered.len() as u64;
    let upheld_count = filtered.iter().filter(|c| c.status == "upheld").count() as u64;
    let rejected_count = filtered.iter().filter(|c| c.status == "rejected").count() as u64;
    let reviewed_count = upheld_count + rejected_count;

    (contest_count, reviewed_count, upheld_count)
}

fn compute_axiom_fire_counts(receipts: &[&Receipt]) -> Vec<AxiomFireCount> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    for r in receipts {
        // A tenet of "" or "none" means no tenet fired; still count it
        let tenet = if r.tenet.is_empty() {
            "(none)".to_string()
        } else {
            r.tenet.clone()
        };
        *counts.entry(tenet).or_insert(0) += 1;
    }
    // Note: AC3 requires that tenets with zero fires in the window are listed.
    // Since we only have the receipts (not a registry of all possible tenets),
    // we can only flag tenets that appear in ANY receipt across all time but 0
    // in the current window. The caller passes `all_receipts` filtered to the
    // window; the test fixture must include the zero-fired tenet in the fixture
    // so it appears at count=0 — we handle that via the `all_known_tenets`
    // parameter that callers can pass via `build_report_with_known_tenets`.
    // See `build_report_with_known_tenets` for the full API.
    let mut fire_counts: Vec<AxiomFireCount> = counts
        .into_iter()
        .map(|(tenet, count)| AxiomFireCount {
            zero_fired: count == 0,
            tenet,
            count,
        })
        .collect();
    fire_counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.tenet.cmp(&b.tenet)));
    fire_counts
}

fn compute_version_drift(receipts: &[&Receipt]) -> Vec<VersionSlice> {
    let mut by_version: HashMap<String, Vec<&Receipt>> = HashMap::new();
    for r in receipts {
        by_version
            .entry(r.ontology_version.clone())
            .or_default()
            .push(r);
    }
    let mut slices: Vec<VersionSlice> = by_version
        .into_iter()
        .map(|(version, recs)| VersionSlice {
            verdict_distribution: compute_verdict_dist(&recs),
            ontology_version: version,
        })
        .collect();
    slices.sort_by(|a, b| a.ontology_version.cmp(&b.ontology_version));
    slices
}

/// Build a report with a known set of tenets, so that tenets that appear
/// in `known_tenets` but never fired in the window are listed with count=0 and
/// `zero_fired=true`. This is the primary API for AC3.
#[allow(dead_code)]
pub fn build_report_with_known_tenets(
    all_receipts: &[Receipt],
    all_contests: &[ContestEntry],
    cutoff: Option<DateTime<Utc>>,
    known_tenets: &[&str],
) -> Result<PulseReport, Box<dyn std::error::Error>> {
    let mut report = build_report(all_receipts, all_contests, cutoff)?;

    // Merge known tenets that didn't fire in the window
    let fired: std::collections::HashSet<String> = report
        .axiom_fire_counts
        .iter()
        .map(|a| a.tenet.clone())
        .collect();
    for tenet in known_tenets {
        if !fired.contains(*tenet) {
            report.axiom_fire_counts.push(AxiomFireCount {
                tenet: tenet.to_string(),
                count: 0,
                zero_fired: true,
            });
        }
    }
    // Re-sort: by count desc, then tenet asc
    report
        .axiom_fire_counts
        .sort_by(|a, b| b.count.cmp(&a.count).then(a.tenet.cmp(&b.tenet)));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::types::Receipt;
    use chrono::Utc;

    fn make_receipt(verdict: &str, tenet: &str, version: &str) -> Receipt {
        Receipt {
            schema: "recourse.receipt.v1".to_string(),
            receipt_id: ulid::Ulid::new().to_string(),
            ts: Utc::now(),
            action_digest: "blake3:aabbcc".to_string(),
            verdict: verdict.to_string(),
            fired_rule: "test-rule".to_string(),
            tenet: tenet.to_string(),
            axiom_chain: vec![],
            ontology_version: version.to_string(),
            guard_version: "0.1.0".to_string(),
            installation_id: "inst-test".to_string(),
        }
    }

    // AC1: verdict counts and percentages sum to ~100%
    #[test]
    fn ac1_verdict_dist_sums_to_100() {
        let receipts = vec![
            make_receipt("allow", "tenet-a", "1.0.0"),
            make_receipt("allow", "tenet-a", "1.0.0"),
            make_receipt("deny", "tenet-b", "1.0.0"),
            make_receipt("flag", "tenet-c", "1.0.0"),
        ];
        let report = build_report(&receipts, &[], None).unwrap();
        assert_eq!(report.total_receipts, 4);
        let total_pct: f64 = report.verdict_distribution.iter().map(|v| v.pct).sum();
        assert!(
            (total_pct - 100.0).abs() < 0.5,
            "pct sum should be ~100%, got {total_pct}"
        );
        // Hand-count: allow=2, deny=1, flag=1
        let allow = report
            .verdict_distribution
            .iter()
            .find(|v| v.verdict == "allow")
            .unwrap();
        assert_eq!(allow.count, 2);
        assert!((allow.pct - 50.0).abs() < 0.1);
    }

    // AC2: contest rate and uphold rate
    #[test]
    fn ac2_contest_and_uphold_rates() {
        let receipts: Vec<Receipt> = (0..10).map(|i| {
            make_receipt(if i % 2 == 0 { "allow" } else { "deny" }, "tenet-x", "1.0.0")
        }).collect();
        let contests = vec![
            ContestEntry { status: "upheld".to_string(), ts: None },
            ContestEntry { status: "upheld".to_string(), ts: None },
            ContestEntry { status: "rejected".to_string(), ts: None },
        ];
        let report = build_report(&receipts, &contests, None).unwrap();
        assert_eq!(report.contest_count, 3);
        assert_eq!(report.upheld_count, 2);
        assert_eq!(report.reviewed_count, 3);
        assert!((report.contest_rate_pct - 30.0).abs() < 0.1); // 3/10 = 30%
        // uphold rate: 2/3 ≈ 66.67%
        assert!((report.uphold_rate_pct - 66.67).abs() < 0.1);
    }

    // AC3: under-fired tenet with zero fires appears flagged
    #[test]
    fn ac3_zero_fired_tenet_flagged() {
        let receipts = vec![
            make_receipt("allow", "tenet-fired", "1.0.0"),
            make_receipt("deny", "tenet-fired", "1.0.0"),
        ];
        // tenet-never-fires exists in the ontology but never in the window
        let known_tenets = ["tenet-fired", "tenet-never-fires"];
        let report = build_report_with_known_tenets(&receipts, &[], None, &known_tenets).unwrap();

        let never = report
            .axiom_fire_counts
            .iter()
            .find(|a| a.tenet == "tenet-never-fires")
            .expect("under-fired tenet must appear in report");
        assert_eq!(never.count, 0);
        assert!(never.zero_fired, "zero_fired flag must be set");

        let fired = report
            .axiom_fire_counts
            .iter()
            .find(|a| a.tenet == "tenet-fired")
            .unwrap();
        assert_eq!(fired.count, 2);
        assert!(!fired.zero_fired);
    }

    // AC4: version drift — two ontology versions, distribution per-version
    #[test]
    fn ac4_version_drift_visible() {
        let receipts = vec![
            make_receipt("allow", "tenet-a", "1.0.0"),
            make_receipt("allow", "tenet-a", "1.0.0"),
            make_receipt("deny", "tenet-a", "1.0.0"),
            // Post-bump: all deny
            make_receipt("deny", "tenet-a", "2.0.0"),
            make_receipt("deny", "tenet-a", "2.0.0"),
        ];
        let report = build_report(&receipts, &[], None).unwrap();
        assert_eq!(report.version_drift.len(), 2);

        let v1 = report
            .version_drift
            .iter()
            .find(|v| v.ontology_version == "1.0.0")
            .unwrap();
        let v2 = report
            .version_drift
            .iter()
            .find(|v| v.ontology_version == "2.0.0")
            .unwrap();

        // v1: 2 allow + 1 deny
        let v1_allow = v1.verdict_distribution.iter().find(|x| x.verdict == "allow").unwrap();
        assert_eq!(v1_allow.count, 2);
        let v1_deny = v1.verdict_distribution.iter().find(|x| x.verdict == "deny").unwrap();
        assert_eq!(v1_deny.count, 1);

        // v2: all deny (shift visible)
        assert_eq!(v2.verdict_distribution.len(), 1);
        assert_eq!(v2.verdict_distribution[0].verdict, "deny");
        assert_eq!(v2.verdict_distribution[0].count, 2);
        assert!((v2.verdict_distribution[0].pct - 100.0).abs() < 0.1);
    }

    // AC5: no private fields in serialized PulseReport
    #[test]
    fn ac5_no_private_fields_in_export() {
        let receipts = vec![
            make_receipt("allow", "tenet-a", "1.0.0"),
        ];
        let report = build_report(&receipts, &[], None).unwrap();
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(
            !json.contains("receipt_id"),
            "export must not contain receipt_id"
        );
        assert!(
            !json.contains("action_digest"),
            "export must not contain action_digest"
        );
    }

    // AC6: pulse never opens actions/ dir — verified structurally:
    // build_report receives pre-loaded receipts, never a path to actions/.
    // The mod.rs run() function explicitly documents it never opens base/actions/.
    #[test]
    fn ac6_no_actions_dir_access_structural() {
        // Structural: build_report takes &[Receipt], not a path.
        // It has no way to open the filesystem — only the caller (run()) could,
        // and run() explicitly avoids actions/. This test confirms compilation.
        let receipts: Vec<Receipt> = Vec::new();
        let report = build_report(&receipts, &[], None).unwrap();
        assert_eq!(report.total_receipts, 0);
    }
}
