//! `recourse amend` — turn an upheld field contest into a tribunal-corpus case.
//!
//! Emits `<tribunal-corpus>/cases/field-<contest-id>/`:
//!   - `action.json`      — canonical action (ABox for ousia-guard)
//!   - `expected.toml`    — {verdict, rule, tenet, rationale}
//!   - `provenance.toml`  — {source, source_ref, author, spot_checked_by}
//!
//! Independence guarantee: `provenance.author` always starts with `downstream:`
//! and is never equal to `"ousia-axioms"`.

pub mod contest;
pub mod corpus;
pub mod tribunal;

// FakeUpheldStore and UpheldContest are used by tests/amend_tests.rs
#[allow(unused_imports)]
pub use contest::{ContestStore, FakeUpheldStore, UpheldContest};
pub use corpus::write_case;
// RecordedTribunalRunner and FakeUpheldStore are used by tests/amend_tests.rs
#[allow(unused_imports)]
pub use tribunal::{RecordedTribunalRunner, TribunalRunner, TribunalRunnerTrait};

use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct AmendArgs {
    /// Contest ID from upheld.ndjson
    pub contest_id: String,

    /// Path to the tribunal-corpus root
    #[arg(long)]
    pub tribunal_corpus: PathBuf,

    /// Override data directory (default: $XDG_DATA_HOME/recourse)
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    /// Path to a raw action JSON file (overrides --store-raw lookup).
    /// Required when the receipt was not emitted with --store-raw.
    #[arg(long)]
    pub action: Option<PathBuf>,

    /// Write case to a temp dir and print it; do not touch real corpus or tribunal
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Core logic with injectable runner — used by tests and the `run` entry point.
#[allow(dead_code)]
pub fn amend_with<R: TribunalRunnerTrait>(
    contest_id: &str,
    tribunal_corpus: &std::path::Path,
    data_dir: &std::path::Path,
    action_json: &str,
    dry_run: bool,
    runner: &R,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cases_dir = if dry_run {
        let tmp = tempfile::tempdir()?;
        // keep() prevents deletion on drop; the temp dir is cleaned by the OS on reboot
        tmp.keep()
    } else {
        tribunal_corpus.join("cases")
    };

    let contest = {
        let store = ContestStore::new(data_dir);
        store.load_upheld(contest_id)?
    };

    let case_dir = write_case(&cases_dir, &contest, action_json)?;

    if !dry_run {
        runner.corpus_validate(tribunal_corpus)?;
        runner.gate(tribunal_corpus)?;
    }

    Ok(case_dir)
}

pub fn run(args: AmendArgs) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = crate::receipt::store::data_dir(args.data_dir.as_deref());

    // Resolve action JSON.  If --action is not given, look up by digest from the raw store.
    let action_json: String = if let Some(action_path) = &args.action {
        std::fs::read_to_string(action_path)
            .map_err(|e| format!("cannot read --action file {}: {e}", action_path.display()))?
    } else {
        let store = ContestStore::new(&data_dir);
        let contest = store.load_upheld(&args.contest_id)?;
        let digest = contest.action_digest.as_deref().unwrap_or("");
        if digest.is_empty() {
            return Err(
                "contest has no action_digest; provide --action <file> to supply the ABox".into(),
            );
        }
        let hex = digest.strip_prefix("blake3:").unwrap_or(digest);
        let action_path = data_dir.join("actions").join(format!("{hex}.json"));
        if !action_path.exists() {
            return Err(format!(
                "raw action not found at {}; re-emit with --store-raw or pass --action <file>",
                action_path.display()
            )
            .into());
        }
        std::fs::read_to_string(&action_path)
            .map_err(|e| format!("cannot read action file {}: {e}", action_path.display()))?
    };

    let cases_dir = if args.dry_run {
        let tmp = tempfile::tempdir()?;
        tmp.keep()
    } else {
        args.tribunal_corpus.join("cases")
    };

    let store = ContestStore::new(&data_dir);
    let contest = store.load_upheld(&args.contest_id)?;
    let case_dir = write_case(&cases_dir, &contest, &action_json)?;

    if args.dry_run {
        println!("-- dry-run: case written to {}", case_dir.display());
        let mut entries: Vec<_> = std::fs::read_dir(&case_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let content = std::fs::read_to_string(entry.path())?;
            println!("\n=== {} ===\n{}", entry.file_name().to_string_lossy(), content);
        }
        return Ok(());
    }

    let runner = TribunalRunner::new_system();
    runner.corpus_validate(&args.tribunal_corpus)?;
    runner.gate(&args.tribunal_corpus)?;

    println!("amend: case written to {}", case_dir.display());
    Ok(())
}
