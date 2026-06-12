//! Tribunal runner — invokes `tribunal corpus validate` then `tribunal gate`.
//!
//! The real runner shells out to the `tribunal` binary.  Tests inject a
//! `RecordedTribunalRunner` (stub) that records the calls and returns
//! configurable outcomes — the real `tribunal` binary may be absent.

use std::path::Path;

/// Trait so tests can inject a stub.
pub trait TribunalRunnerTrait {
    fn corpus_validate(&self, corpus: &Path) -> Result<(), Box<dyn std::error::Error>>;
    fn gate(&self, corpus: &Path) -> Result<(), Box<dyn std::error::Error>>;
}

/// Production runner: shells out to the `tribunal` binary on PATH.
pub struct TribunalRunner;

impl TribunalRunner {
    pub fn new_system() -> Self {
        TribunalRunner
    }
}

impl TribunalRunnerTrait for TribunalRunner {
    fn corpus_validate(&self, corpus: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let status = std::process::Command::new("tribunal")
            .args(["corpus", "validate", "--corpus"])
            .arg(corpus)
            .status()
            .map_err(|e| format!("failed to invoke `tribunal corpus validate`: {e}"))?;
        if !status.success() {
            return Err(format!(
                "`tribunal corpus validate` exited with status {}",
                status
            )
            .into());
        }
        Ok(())
    }

    fn gate(&self, corpus: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let status = std::process::Command::new("tribunal")
            .args(["gate", "--corpus"])
            .arg(corpus)
            .status()
            .map_err(|e| format!("failed to invoke `tribunal gate`: {e}"))?;
        if !status.success() {
            return Err(format!("`tribunal gate` exited with status {}", status).into());
        }
        Ok(())
    }
}

// ── Test helpers (pub so tests/amend_tests.rs can use them) ──────────────────

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A recorded stub: captures calls and returns configurable outcomes.
pub struct RecordedTribunalRunner {
    pub validate_calls: Arc<Mutex<Vec<PathBuf>>>,
    pub gate_calls: Arc<Mutex<Vec<PathBuf>>>,
    pub gate_should_fail: RefCell<bool>,
    pub validate_should_fail: RefCell<bool>,
}

impl Default for RecordedTribunalRunner {
    fn default() -> Self {
        RecordedTribunalRunner {
            validate_calls: Arc::new(Mutex::new(Vec::new())),
            gate_calls: Arc::new(Mutex::new(Vec::new())),
            gate_should_fail: RefCell::new(false),
            validate_should_fail: RefCell::new(false),
        }
    }
}

#[allow(dead_code)]
impl RecordedTribunalRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_gate_fails(&self) {
        *self.gate_should_fail.borrow_mut() = true;
    }
}

impl TribunalRunnerTrait for RecordedTribunalRunner {
    fn corpus_validate(&self, corpus: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_calls
            .lock()
            .unwrap()
            .push(corpus.to_path_buf());
        if *self.validate_should_fail.borrow() {
            return Err("`tribunal corpus validate` stub: configured to fail".into());
        }
        Ok(())
    }

    fn gate(&self, corpus: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.gate_calls.lock().unwrap().push(corpus.to_path_buf());
        if *self.gate_should_fail.borrow() {
            return Err("`tribunal gate` stub: configured to fail".into());
        }
        Ok(())
    }
}
