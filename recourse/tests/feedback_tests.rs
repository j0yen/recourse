//! Integration tests for `recourse feedback propose` and `recourse feedback ship`.
//!
//! Uses in-process helpers (RecordedHeraldClient, RecordedTribunalGate) so no
//! external binaries are required.

use recourse::feedback::herald::{HeraldMarketTrait, RecordedHeraldClient};
use recourse::feedback::propose::{run_with as propose_run, ProposalResult};
use recourse::feedback::ship::{run_with as ship_run, RecordedTribunalGate, TribunalGateTrait};
use recourse::feedback::version;

use std::fs;
use tempfile::TempDir;

// ---- helpers ----

fn make_data_dir() -> TempDir {
    tempfile::tempdir().expect("tmpdir")
}

/// Write a minimal upheld.ndjson with the given contest ids.
fn write_upheld(data_dir: &std::path::Path, ids: &[&str]) {
    let content: String = ids
        .iter()
        .map(|id| {
            format!(
                r#"{{"contest_id":"{id}","status":"upheld","claimed_verdict":"allow","receipt_id":"r-{id}","reason":"wrong verdict on field","contestant":"alice","reviewer":"bob","rule":"rule-1","tenet":"tenet-1"}}"#,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(data_dir.join("upheld.ndjson"), content).unwrap();
}

/// Create a corpus case dir to simulate `recourse amend` having run.
fn make_corpus_case(corpus_dir: &std::path::Path, id: &str) {
    let case_dir = corpus_dir.join("cases").join(format!("field-{id}"));
    fs::create_dir_all(&case_dir).unwrap();
    fs::write(case_dir.join("action.json"), r#"{"type":"test"}"#).unwrap();
}

// ---- AC1: propose renders changeset.toml with correct fields + proposed semver ----

#[test]
fn propose_renders_changeset_with_field_cases_and_semver() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    write_upheld(data_dir.path(), &["c1", "c2"]);
    make_corpus_case(corpus_dir.path(), "c1");
    make_corpus_case(corpus_dir.path(), "c2");

    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        None,
        Some(proposals_dir.path()),
    )
    .expect("propose should succeed");

    match result {
        ProposalResult::Proposed {
            version,
            field_cases,
            changeset_path,
            changelog_path,
        } => {
            // Semver must be strictly > 0.0.0 (the baseline)
            let v = version::Version::parse(&version).expect("valid semver");
            let baseline = version::Version::zero();
            assert!(v.is_greater_than(&baseline), "proposed {v} must be > 0.0.0");

            // Both field cases present
            assert!(field_cases.contains(&"field-c1".to_string()));
            assert!(field_cases.contains(&"field-c2".to_string()));

            // Files exist
            assert!(changeset_path.exists(), "changeset.toml missing");
            assert!(changelog_path.exists(), "CHANGELOG-<version>.md missing");

            // changeset.toml lists both cases
            let content = fs::read_to_string(&changeset_path).unwrap();
            assert!(content.contains("field-c1"));
            assert!(content.contains("field-c2"));
            assert!(content.contains(&version));

            // CHANGELOG names both contests
            let cl = fs::read_to_string(&changelog_path).unwrap();
            assert!(cl.contains("c1"));
            assert!(cl.contains("c2"));
        }
        ProposalResult::NothingToShip => panic!("expected a proposal, got NothingToShip"),
    }
}

// ---- AC2: propose never calls herald-market ----

#[test]
fn propose_never_calls_herald() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    write_upheld(data_dir.path(), &["c3"]);
    make_corpus_case(corpus_dir.path(), "c3");

    // We call the pure propose_run — it never touches any herald client.
    // Asserting this at the type level: propose_run has no herald parameter.
    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        None,
        Some(proposals_dir.path()),
    )
    .unwrap();

    assert!(matches!(result, ProposalResult::Proposed { .. }));

    // Confirm no marketplace file was written (the only place propose could write one
    // would be proposals_dir, and it only writes changeset.toml + CHANGELOG-*.md).
    let files: Vec<_> = fs::read_dir(proposals_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        !files.iter().any(|f| f.contains("marketplace")),
        "propose must not write any marketplace file; found: {files:?}"
    );
}

// ---- AC3: gate failure blocks publish ----

#[test]
fn ship_gate_failure_blocks_publish() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    // First propose so ship has a changeset to read.
    write_upheld(data_dir.path(), &["c4"]);
    make_corpus_case(corpus_dir.path(), "c4");
    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        None,
        Some(proposals_dir.path()),
    )
    .unwrap();
    let version = match result {
        ProposalResult::Proposed { version, .. } => version,
        _ => panic!("expected proposal"),
    };

    let gate = RecordedTribunalGate::new();
    gate.set_fail_next("simulated tribunal gate failure");
    let herald = RecordedHeraldClient::new();

    let err = ship_run(
        data_dir.path(),
        &version,
        true, // --confirm
        corpus_dir.path(),
        Some(proposals_dir.path()),
        &gate,
        &herald,
    )
    .expect_err("ship should fail when gate fails");

    assert!(
        err.to_string().contains("simulated tribunal gate failure"),
        "error should mention gate failure: {err}"
    );
    // herald must NOT have been called
    assert_eq!(herald.publish_call_count(), 0, "herald must not be called after gate failure");
}

// ---- AC4a: passing gate + --confirm invokes herald exactly once ----

#[test]
fn ship_confirm_invokes_herald_exactly_once() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    write_upheld(data_dir.path(), &["c5"]);
    make_corpus_case(corpus_dir.path(), "c5");
    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        None,
        Some(proposals_dir.path()),
    )
    .unwrap();
    let version = match result {
        ProposalResult::Proposed { version, .. } => version,
        _ => panic!("expected proposal"),
    };

    let gate = RecordedTribunalGate::new();
    let herald = RecordedHeraldClient::new();

    ship_run(
        data_dir.path(),
        &version,
        true, // --confirm
        corpus_dir.path(),
        Some(proposals_dir.path()),
        &gate,
        &herald,
    )
    .expect("ship should succeed");

    assert_eq!(gate.gate_call_count(), 1, "tribunal gate must be called exactly once");
    assert_eq!(herald.publish_call_count(), 1, "herald must be called exactly once");
    assert_eq!(herald.calls.borrow()[0], version);
}

// ---- AC4b: without --confirm, no publish ----

#[test]
fn ship_without_confirm_does_not_publish() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    write_upheld(data_dir.path(), &["c6"]);
    make_corpus_case(corpus_dir.path(), "c6");
    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        None,
        Some(proposals_dir.path()),
    )
    .unwrap();
    let version = match result {
        ProposalResult::Proposed { version, .. } => version,
        _ => panic!("expected proposal"),
    };

    let gate = RecordedTribunalGate::new();
    let herald = RecordedHeraldClient::new();

    ship_run(
        data_dir.path(),
        &version,
        false, // no --confirm
        corpus_dir.path(),
        Some(proposals_dir.path()),
        &gate,
        &herald,
    )
    .expect("dry preview should succeed");

    assert_eq!(herald.publish_call_count(), 0, "no --confirm → no publish");
    assert_eq!(gate.gate_call_count(), 0, "dry mode must not run tribunal gate");
}

// ---- AC5: zero new upheld contests → nothing to ship ----

#[test]
fn propose_nothing_to_ship_when_no_amended_contests() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    // upheld.ndjson has a contest but it has NOT been amended into the corpus
    write_upheld(data_dir.path(), &["c_not_amended"]);
    // Note: we do NOT call make_corpus_case

    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        None,
        Some(proposals_dir.path()),
    )
    .unwrap();

    assert!(
        matches!(result, ProposalResult::NothingToShip),
        "expected NothingToShip"
    );

    // No files written
    let count = fs::read_dir(proposals_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .count();
    assert_eq!(count, 0, "propose must write nothing when there's nothing to ship");
}

// ---- AC5b: completely empty upheld.ndjson ----

#[test]
fn propose_nothing_to_ship_when_empty_upheld() {
    let data_dir = make_data_dir();
    // No upheld.ndjson at all
    let result = propose_run(data_dir.path(), None, None, None, None).unwrap();
    assert!(matches!(result, ProposalResult::NothingToShip));
}

// ---- AC6: --upstream records intent, does not mutate marketplace ----

#[test]
fn propose_upstream_records_intent_no_marketplace_mutation() {
    let data_dir = make_data_dir();
    let corpus_dir = make_data_dir();
    let proposals_dir = make_data_dir();

    write_upheld(data_dir.path(), &["c7"]);
    make_corpus_case(corpus_dir.path(), "c7");

    let result = propose_run(
        data_dir.path(),
        None,
        Some(corpus_dir.path()),
        Some("j0yen/ousia-axioms"),
        Some(proposals_dir.path()),
    )
    .unwrap();

    match result {
        ProposalResult::Proposed { changeset_path, .. } => {
            let content = fs::read_to_string(&changeset_path).unwrap();
            // upstream_target recorded
            assert!(
                content.contains("j0yen/ousia-axioms"),
                "changeset must record upstream target"
            );
            // upstream_note about deferred mechanics present
            assert!(
                content.contains("deferred") || content.contains("out of scope") || content.contains("successor"),
                "changeset must note that PR mechanics are deferred"
            );
            // No marketplace file
            let files: Vec<_> = fs::read_dir(proposals_dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            assert!(
                !files.iter().any(|f| f.contains("marketplace")),
                "must not write marketplace file; got {files:?}"
            );
        }
        ProposalResult::NothingToShip => panic!("expected proposal"),
    }
}
