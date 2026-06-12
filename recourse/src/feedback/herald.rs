//! herald-market client abstraction.
//!
//! The real implementation shells out to `herald-market publish <version>`.
//! Tests inject a `RecordedHeraldClient` to assert publish call counts
//! without touching the real marketplace.

use std::cell::RefCell;

/// Trait for publishing a version via herald-market.
#[allow(dead_code)]
pub trait HeraldMarketTrait {
    fn publish(&self, version: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn publish_call_count(&self) -> usize;
}

/// System (real) implementation — shells out to `herald-market publish <version>`.
pub struct HeraldMarketClient {
    call_count: RefCell<usize>,
}

impl HeraldMarketClient {
    pub fn new_system() -> Self {
        HeraldMarketClient {
            call_count: RefCell::new(0),
        }
    }
}

impl HeraldMarketTrait for HeraldMarketClient {
    fn publish(&self, version: &str) -> Result<(), Box<dyn std::error::Error>> {
        *self.call_count.borrow_mut() += 1;
        let status = std::process::Command::new("herald-market")
            .args(["publish", version])
            .status()
            .map_err(|e| format!("herald-market not found or failed to start: {e}"))?;
        if !status.success() {
            return Err(format!(
                "herald-market publish {version} exited with status: {status}"
            )
            .into());
        }
        Ok(())
    }

    fn publish_call_count(&self) -> usize {
        *self.call_count.borrow()
    }
}

/// Recorded stub for tests — never calls real herald-market.
#[allow(dead_code)]
pub struct RecordedHeraldClient {
    pub calls: RefCell<Vec<String>>,
    /// If Some, return this error on the next publish call.
    pub fail_next: RefCell<Option<String>>,
}

impl Default for RecordedHeraldClient {
    fn default() -> Self {
        RecordedHeraldClient {
            calls: RefCell::new(Vec::new()),
            fail_next: RefCell::new(None),
        }
    }
}

impl RecordedHeraldClient {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn set_fail_next(&self, msg: &str) {
        *self.fail_next.borrow_mut() = Some(msg.to_string());
    }
}

impl HeraldMarketTrait for RecordedHeraldClient {
    fn publish(&self, version: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(msg) = self.fail_next.borrow_mut().take() {
            return Err(msg.into());
        }
        self.calls.borrow_mut().push(version.to_string());
        Ok(())
    }

    fn publish_call_count(&self) -> usize {
        self.calls.borrow().len()
    }
}
