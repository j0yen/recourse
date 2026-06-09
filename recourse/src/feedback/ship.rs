//! `recourse feedback ship <version> --confirm` implementation.
//!
//! Pre-flight: runs `tribunal gate` against the amended corpus.
//! A gate failure → non-zero exit, no herald-market call.
//! Without `--confirm` → dry preview only, no publish.
//! On pass + --confirm → `herald-market publish <version>` called exactly once.

use super::herald::HeraldMarketTrait;
use super::propose::ChangesetToml;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

// ---- tribunal gate abstraction ----

#[allow(dead_code)]
pub trait TribunalGateTrait {
    fn gate(&self, tribunal_corpus: &Path) -> Result<(), Box<dyn std::error::Error>>;
    fn gate_call_count(&self) -> usize;
}

/// System (real) runner — shells out to `tribunal gate <corpus>`.
pub struct TribunalGateRunner {
    call_count: RefCell<usize>,
}

impl TribunalGateRunner {
    pub fn new_system() -> Self {
        TribunalGateRunner {
            call_count: RefCell::new(0),
        }
    }
}

impl TribunalGateTrait for TribunalGateRunner {
    fn gate(&self, tribunal_corpus: &Path) -> Result<(), Box<dyn std::error::Error>> {
        *self.call_count.borrow_mut() += 1;
        let status = std::process::Command::new("tribunal")
            .args(["gate", &tribunal_corpus.to_string_lossy()])
            .status()
            .map_err(|e| format!("tribunal not found or failed to start: {e}"))?;
        if !status.success() {
            return Err(format!(
                "tribunal gate failed (exit {}); refusing to publish",
                status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            )
            .into());
        }
        Ok(())
    }

    fn gate_call_count(&self) -> usize {
        *self.call_count.borrow()
    }
}

/// Recorded stub for tests.
pub struct RecordedTribunalGate {
    pub calls: RefCell<Vec<PathBuf>>,
    /// If Some, return this error on next call.
    pub fail_next: RefCell<Option<String>>,
}

impl RecordedTribunalGate {
    #[allow(dead_code)]
    pub fn new() -> Self {
        RecordedTribunalGate {
            calls: RefCell::new(Vec::new()),
            fail_next: RefCell::new(None),
        }
    }

    #[allow(dead_code)]
    pub fn set_fail_next(&self, msg: &str) {
        *self.fail_next.borrow_mut() = Some(msg.to_string());
    }
}

impl TribunalGateTrait for RecordedTribunalGate {
    fn gate(&self, tribunal_corpus: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(msg) = self.fail_next.borrow_mut().take() {
            return Err(msg.into());
        }
        self.calls.borrow_mut().push(tribunal_corpus.to_path_buf());
        Ok(())
    }

    fn gate_call_count(&self) -> usize {
        self.calls.borrow().len()
    }
}

// ---- ship logic ----

/// Core ship logic with injectable runner + herald — used by tests.
pub fn run_with<G: TribunalGateTrait, H: HeraldMarketTrait>(
    data_dir: &Path,
    version: &str,
    confirm: bool,
    tribunal_corpus: &Path,
    proposals_dir: Option<&Path>,
    runner: &G,
    herald: &H,
) -> Result<(), Box<dyn std::error::Error>> {
    let proposals_path = proposals_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| data_dir.join("proposals"));

    // Load changeset proposal.
    let changeset_path = proposals_path.join(format!("changeset-{version}.toml"));
    if !changeset_path.exists() {
        return Err(format!(
            "no changeset proposal found for version {version} at {}; \
             run `recourse feedback propose` first",
            changeset_path.display()
        )
        .into());
    }
    let changeset_content = std::fs::read_to_string(&changeset_path)
        .map_err(|e| format!("cannot read {}: {e}", changeset_path.display()))?;
    let changeset: ChangesetToml = toml::from_str(&changeset_content)
        .map_err(|e| format!("cannot parse changeset: {e}"))?;

    if !confirm {
        // Dry preview: print what would happen, publish nothing.
        println!("(dry preview — pass --confirm to actually publish)");
        println!("version: {}", changeset.version);
        println!("field cases: {}", changeset.field_cases.join(", "));
        if !changeset.pulse_deltas.is_empty() {
            println!("pulse deltas: {}", changeset.pulse_deltas.join("; "));
        }
        println!("pre-flight: tribunal gate would run against {}", tribunal_corpus.display());
        println!("publish: herald-market publish {} (NOT called in dry mode)", version);
        return Ok(());
    }

    // Pre-flight: tribunal gate BEFORE any publish call.
    println!("running tribunal gate against {}...", tribunal_corpus.display());
    runner.gate(tribunal_corpus)?;
    println!("tribunal gate: passed");

    // Publish.
    println!("publishing version {} via herald-market...", version);
    herald.publish(version)?;
    println!("published: {}", version);

    // Record the new shipped version.
    let ver = super::version::Version::parse(version)
        .map_err(|e| format!("invalid version '{version}': {e}"))?;
    super::version::write_current(data_dir, &ver)?;
    println!("version.toml updated to {}", version);

    Ok(())
}
