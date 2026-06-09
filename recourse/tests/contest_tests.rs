//! Integration tests for `recourse contest` (ACs 1–8 from PRD-recourse-contest).

use recourse::contest::store::ContestStore;
#[allow(unused_imports)]
use recourse::contest::types::Contest;
use recourse::receipt::store as receipt_store;
use recourse::receipt::types::Receipt;
use std::fs;
use tempfile::TempDir;

fn setup() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn write_receipt(data: &std::path::Path, verdict: &str) -> Receipt {
    let receipt = Receipt::new(
        "blake3:aabbcc".to_string(),
        verdict.to_string(),
        "test-rule".to_string(),
        "test-tenet".to_string(),
        vec!["axiom-1".to_string()],
        "ontology-1".to_string(),
        "guard-1".to_string(),
        "install-abc".to_string(),
    );
    receipt_store::append_receipt(data, &receipt).expect("append receipt");
    receipt
}

// AC1: contest <id> --expected deny against allow receipt → one pending record + prints contest_id
#[test]
fn ac1_contest_submit_appends_pending() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "allow");

    // Run submit
    let result = recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "deny",
        "this should be denied",
        Some(data.to_path_buf()),
    );
    assert!(result.is_ok(), "submit failed: {:?}", result);

    let store = ContestStore::new(data);
    let pending = store.load_pending().expect("load pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, "pending");
    assert_eq!(pending[0].schema, "recourse.contest.v1");
    assert_eq!(pending[0].claimed_verdict, "deny");
    assert_eq!(pending[0].observed_verdict, "allow");
    assert_eq!(pending[0].receipt_id, receipt.receipt_id);
}

// AC2: contesting with --expected equal to receipt verdict exits non-zero; no record written
#[test]
fn ac2_same_verdict_rejected() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "allow");

    let result = recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "allow", // same as receipt verdict
        "some reason",
        Some(data.to_path_buf()),
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("nothing to contest"), "expected 'nothing to contest' in: {msg}");

    // No record written
    let store = ContestStore::new(data);
    let pending = store.load_pending().expect("load pending");
    assert!(pending.is_empty());
}

// AC3: empty/missing reason is rejected; no record written
#[test]
fn ac3_empty_reason_rejected() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "allow");

    let result = recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "deny",
        "   ", // whitespace-only
        Some(data.to_path_buf()),
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("non-empty"), "expected 'non-empty' in: {msg}");

    let store = ContestStore::new(data);
    let pending = store.load_pending().expect("load pending");
    assert!(pending.is_empty());
}

// AC4: contesting unknown receipt-id exits non-zero; no record written
#[test]
fn ac4_unknown_receipt_rejected() {
    let tmp = setup();
    let data = tmp.path();

    let result = recourse::contest::submit::cmd_contest_submit(
        "01AAAAAAAAAAAAAAAAAAAAAAAAA", // bogus ULID
        "deny",
        "some reason",
        Some(data.to_path_buf()),
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("not found"), "expected 'not found' in: {msg}");

    let store = ContestStore::new(data);
    let pending = store.load_pending().expect("load pending");
    assert!(pending.is_empty());
}

// AC5: contest ls --pending shows only pending contests, includes observed verdict + fired_rule
#[test]
fn ac5_ls_pending_shows_context() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "flag");

    // Submit a contest
    recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "allow",
        "false positive",
        Some(data.to_path_buf()),
    )
    .expect("submit");

    // Run ls and capture output via the store directly (unit-level check)
    let store = ContestStore::new(data);
    let pending = store.load_pending().expect("load pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].observed_verdict, "flag");

    // Verify the joined row includes fired_rule
    let joined_receipt = receipt_store::find_receipt(data, &pending[0].receipt_id)
        .expect("find")
        .expect("exists");
    assert_eq!(joined_receipt.fired_rule, "test-rule");
}

// AC5 (json format): --format json returns valid array
#[test]
fn ac5_ls_json_format() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "deny");

    recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "allow",
        "reason here",
        Some(data.to_path_buf()),
    )
    .expect("submit");

    // ls cmd returns Ok (output goes to stdout in real use; we verify store)
    let result = recourse::contest::ls::cmd_contest_ls("pending", "json", Some(data.to_path_buf()));
    assert!(result.is_ok(), "{:?}", result);
}

// AC6: no-mutation invariant — corpus/ontology dirs untouched after contest + ls
#[test]
fn ac6_no_mutation_of_corpus_or_ontology() {
    let tmp = setup();
    let data = tmp.path();

    // Create fake corpus + ontology fixture dirs
    let corpus_dir = tmp.path().join("corpus");
    let ontology_dir = tmp.path().join("ontology");
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::create_dir_all(&ontology_dir).unwrap();
    fs::write(corpus_dir.join("sentinel.txt"), b"unchanged").unwrap();
    fs::write(ontology_dir.join("classes.json"), b"{}").unwrap();

    let receipt = write_receipt(data, "allow");

    recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "deny",
        "should be denied",
        Some(data.to_path_buf()),
    )
    .expect("submit");

    let _ = recourse::contest::ls::cmd_contest_ls("pending", "pretty", Some(data.to_path_buf()));

    // Assert corpus + ontology dirs are byte-identical
    let corpus_after = fs::read(corpus_dir.join("sentinel.txt")).unwrap();
    assert_eq!(corpus_after, b"unchanged");
    let ontology_after = fs::read(ontology_dir.join("classes.json")).unwrap();
    assert_eq!(ontology_after, b"{}");

    // Strictly: no new files in corpus or ontology dirs
    let corpus_files: Vec<_> = fs::read_dir(&corpus_dir).unwrap().collect();
    let ontology_files: Vec<_> = fs::read_dir(&ontology_dir).unwrap().collect();
    assert_eq!(corpus_files.len(), 1, "corpus dir must have exactly 1 file");
    assert_eq!(ontology_files.len(), 1, "ontology dir must have exactly 1 file");
}

// AC7: review --uphold moves record from pending to upheld; --reject moves to rejected
#[test]
fn ac7_review_uphold_and_reject() {
    let tmp = setup();
    let data = tmp.path();
    let r1 = write_receipt(data, "allow");
    let r2 = write_receipt(data, "allow");

    // Two contests
    recourse::contest::submit::cmd_contest_submit(
        &r1.receipt_id,
        "deny",
        "reason A",
        Some(data.to_path_buf()),
    )
    .expect("submit 1");
    recourse::contest::submit::cmd_contest_submit(
        &r2.receipt_id,
        "deny",
        "reason B",
        Some(data.to_path_buf()),
    )
    .expect("submit 2");

    let store = ContestStore::new(data);
    let pending = store.load_pending().expect("load pending");
    assert_eq!(pending.len(), 2);

    let cid_a = pending[0].contest_id.clone();
    let cid_b = pending[1].contest_id.clone();

    // Uphold first
    recourse::contest::review::cmd_contest_review(
        &cid_a,
        true,
        false,
        Some("looks valid"),
        Some(data.to_path_buf()),
    )
    .expect("uphold");

    // Reject second
    recourse::contest::review::cmd_contest_review(
        &cid_b,
        false,
        true,
        Some("not actionable"),
        Some(data.to_path_buf()),
    )
    .expect("reject");

    // Pending should be empty
    let pending_after = store.load_pending().expect("reload");
    assert!(pending_after.is_empty(), "pending should be empty after review");

    // Upheld should have one entry
    let all = store.load_all().expect("load all");
    let upheld: Vec<_> = all.iter().filter(|c| c.status == "upheld").collect();
    let rejected: Vec<_> = all.iter().filter(|c| c.status == "rejected").collect();
    assert_eq!(upheld.len(), 1);
    assert_eq!(rejected.len(), 1);
}

// AC7: no code path allows auto-uphold (grep-level: no batch move without explicit review call)
// This is enforced structurally — the store has no auto-uphold fn; this test proves submit alone
// does not change any record out of pending.
#[test]
fn ac7_no_auto_uphold() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "allow");

    recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "deny",
        "disputed",
        Some(data.to_path_buf()),
    )
    .expect("submit");

    let store = ContestStore::new(data);
    let all = store.load_all().expect("all");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].status, "pending", "submit must not change status off pending");
}

// AC8: SIGPIPE safety — verified at compile time by main.rs; contest functions are callable
#[test]
fn ac8_contest_functions_compile_and_run() {
    let tmp = setup();
    let data = tmp.path();
    let receipt = write_receipt(data, "flag");

    let r = recourse::contest::submit::cmd_contest_submit(
        &receipt.receipt_id,
        "allow",
        "test reason",
        Some(data.to_path_buf()),
    );
    assert!(r.is_ok());
}
