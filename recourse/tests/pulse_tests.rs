//! Integration tests for `recourse pulse` (AC1-AC7).

use recourse::pulse::report::{build_report, build_report_with_known_tenets, ContestEntry};
use recourse::pulse::since::parse_since;
use recourse::receipt::types::Receipt;
use chrono::Utc;
use tempfile::TempDir;

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

fn contest(status: &str) -> ContestEntry {
    ContestEntry { status: status.to_string(), ts: None }
}

// AC1: verdict counts and percentages from a fixture sink sum to 100% (±0.5%)
// and match hand-count.
#[test]
fn ac1_verdict_dist_fixture_hand_count() {
    let receipts = vec![
        make_receipt("allow", "tenet-a", "1.0.0"),
        make_receipt("allow", "tenet-a", "1.0.0"),
        make_receipt("allow", "tenet-b", "1.0.0"),
        make_receipt("deny",  "tenet-a", "1.0.0"),
        make_receipt("flag",  "tenet-c", "1.0.0"),
    ];
    let report = build_report(&receipts, &[], None).unwrap();
    assert_eq!(report.total_receipts, 5);

    // Hand-count: allow=3 (60%), deny=1 (20%), flag=1 (20%)
    let allow = report.verdict_distribution.iter().find(|v| v.verdict == "allow").unwrap();
    assert_eq!(allow.count, 3);
    assert!((allow.pct - 60.0).abs() < 0.5, "allow pct should be 60%, got {}", allow.pct);

    let total_pct: f64 = report.verdict_distribution.iter().map(|v| v.pct).sum();
    assert!((total_pct - 100.0).abs() < 0.5, "pct sum={total_pct}");
}

// AC2: contest rate and uphold rate against a fixture.
#[test]
fn ac2_contest_rate_and_uphold_rate() {
    let receipts: Vec<Receipt> = (0..10).map(|i| {
        make_receipt(if i < 7 { "allow" } else { "deny" }, "t", "1.0.0")
    }).collect();

    let contests = vec![
        contest("upheld"),
        contest("upheld"),
        contest("rejected"),
        contest("pending"),
    ];

    let report = build_report(&receipts, &contests, None).unwrap();
    // 4 contests / 10 receipts = 40%
    assert_eq!(report.contest_count, 4);
    assert!((report.contest_rate_pct - 40.0).abs() < 0.1, "contest_rate={}", report.contest_rate_pct);
    // reviewed = upheld(2) + rejected(1) = 3; pending excluded
    assert_eq!(report.reviewed_count, 3);
    assert_eq!(report.upheld_count, 2);
    // uphold rate: 2/3 ≈ 66.67%
    assert!((report.uphold_rate_pct - 66.67).abs() < 0.1, "uphold_rate={}", report.uphold_rate_pct);
}

// AC3: per-axiom fire counts; zero-fired tenet is listed with flag, not omitted.
#[test]
fn ac3_zero_fired_tenet_not_omitted() {
    let receipts = vec![
        make_receipt("allow", "active-tenet", "1.0.0"),
        make_receipt("deny",  "active-tenet", "1.0.0"),
    ];
    let known = ["active-tenet", "never-fired-tenet"];
    let report = build_report_with_known_tenets(&receipts, &[], None, &known).unwrap();

    let never = report.axiom_fire_counts.iter()
        .find(|a| a.tenet == "never-fired-tenet")
        .expect("under-fired tenet must be listed");
    assert_eq!(never.count, 0);
    assert!(never.zero_fired, "zero_fired must be true");

    let active = report.axiom_fire_counts.iter()
        .find(|a| a.tenet == "active-tenet")
        .unwrap();
    assert_eq!(active.count, 2);
    assert!(!active.zero_fired);

    // active appears before never-fired (sorted desc by count)
    let active_idx = report.axiom_fire_counts.iter().position(|a| a.tenet == "active-tenet").unwrap();
    let never_idx  = report.axiom_fire_counts.iter().position(|a| a.tenet == "never-fired-tenet").unwrap();
    assert!(active_idx < never_idx, "active tenet should sort before zero-fired");
}

// AC4: with receipts under two ontology_versions, per-version breakdown shows shift.
#[test]
fn ac4_version_drift_shows_shift() {
    let receipts = vec![
        make_receipt("allow", "t", "1.0.0"),
        make_receipt("allow", "t", "1.0.0"),
        make_receipt("deny",  "t", "1.0.0"),
        // Post-bump: mostly deny — a distribution shift
        make_receipt("deny", "t", "2.0.0"),
        make_receipt("deny", "t", "2.0.0"),
        make_receipt("deny", "t", "2.0.0"),
    ];
    let report = build_report(&receipts, &[], None).unwrap();
    assert_eq!(report.version_drift.len(), 2);

    let v2 = report.version_drift.iter().find(|v| v.ontology_version == "2.0.0").unwrap();
    assert_eq!(v2.verdict_distribution.len(), 1, "v2 should have only deny");
    assert_eq!(v2.verdict_distribution[0].verdict, "deny");
    assert!((v2.verdict_distribution[0].pct - 100.0).abs() < 0.1);
}

// AC5: export contains no receipt_id or action_digest.
#[test]
fn ac5_export_no_private_fields() {
    let tmp = TempDir::new().unwrap();
    let export_path = tmp.path().join("pulse-export.json");

    let receipts = vec![
        make_receipt("allow", "tenet-a", "1.0.0"),
        make_receipt("deny",  "tenet-b", "1.0.0"),
    ];
    let report = build_report(&receipts, &[], None).unwrap();
    let json = serde_json::to_string_pretty(&report).unwrap();
    std::fs::write(&export_path, &json).unwrap();

    let content = std::fs::read_to_string(&export_path).unwrap();
    assert!(!content.contains("receipt_id"), "export must not contain receipt_id");
    assert!(!content.contains("action_digest"), "export must not contain action_digest");
}

// AC6: pulse never reads actions/ directory.
// Structural: build_report receives &[Receipt] not a path, so it cannot open actions/.
// This test verifies that even when actions/ contains a sentinel file, the report
// is independent of its contents.
#[test]
fn ac6_actions_dir_never_read() {
    let tmp = TempDir::new().unwrap();
    // Create a sentinel file in actions/
    let actions_dir = tmp.path().join("actions");
    std::fs::create_dir_all(&actions_dir).unwrap();
    let sentinel = actions_dir.join("sentinel_file.json");
    std::fs::write(&sentinel, r#"{"secret": "SENTINEL_SECRET_VALUE"}"#).unwrap();

    // Load receipts normally (from receipts/ dir, not actions/)
    let receipts = recourse::receipt::store::load_all_receipts(tmp.path()).unwrap();
    // No receipts written → empty list
    assert!(receipts.is_empty());

    let report = build_report(&receipts, &[], None).unwrap();
    // Report is valid and doesn't contain the sentinel value
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("SENTINEL_SECRET_VALUE"), "sentinel must not appear in report");
}

// AC7: parse_since parses common formats correctly.
#[test]
fn ac7_parse_since_30d() {
    let cutoff = parse_since("30d").unwrap();
    let diff = Utc::now() - cutoff;
    let expected = 30i64 * 24 * 3600;
    assert!(
        (diff.num_seconds() - expected).abs() < 5,
        "expected ~30d, got {}s",
        diff.num_seconds()
    );
}

#[test]
fn ac7_parse_since_24h() {
    let cutoff = parse_since("24h").unwrap();
    let diff = Utc::now() - cutoff;
    let expected = 24i64 * 3600;
    assert!(
        (diff.num_seconds() - expected).abs() < 5,
        "expected ~24h, got {}s",
        diff.num_seconds()
    );
}

#[test]
fn ac7_parse_since_bad() {
    assert!(parse_since("30x").is_err());
}
