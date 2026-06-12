//! Contest store — reads upheld.ndjson and validates contest status.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// One record from upheld.ndjson (the minimal fields we need).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpheldContest {
    pub contest_id: String,
    pub status: String,
    /// The verdict the field claims should have been emitted
    pub claimed_verdict: String,
    /// Receipt that triggered the contest
    pub receipt_id: String,
    /// The rule that the reviewer assigned (may be same as receipt's fired_rule)
    pub rule: Option<String>,
    /// The tenet
    pub tenet: Option<String>,
    /// Human-readable reason from the contestant
    pub reason: String,
    /// Identity of the contestant (used as downstream:<contestant>)
    pub contestant: String,
    /// Identity of the reviewer who upheld the contest
    pub reviewer: String,
    /// Optional: action digest from the associated receipt (for raw-store lookup)
    pub action_digest: Option<String>,
}

pub struct ContestStore {
    data_dir: PathBuf,
}

impl ContestStore {
    pub fn new(data_dir: &Path) -> Self {
        ContestStore {
            data_dir: data_dir.to_path_buf(),
        }
    }

    /// Load a contest that must be in `upheld.ndjson`.
    /// Returns an error if the contest is pending, rejected, or not found.
    pub fn load_upheld(
        &self,
        contest_id: &str,
    ) -> Result<UpheldContest, Box<dyn std::error::Error>> {
        let upheld_path = self.data_dir.join("upheld.ndjson");
        if !upheld_path.exists() {
            return Err(format!(
                "upheld.ndjson not found at {}; no upheld contests exist",
                upheld_path.display()
            )
            .into());
        }

        let f = fs::File::open(&upheld_path)
            .map_err(|e| format!("cannot open {}: {e}", upheld_path.display()))?;
        let reader = BufReader::new(f);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let contest: UpheldContest = match serde_json::from_str(trimmed) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("warn: skipping malformed upheld line: {e}");
                    continue;
                }
            };
            if contest.contest_id == contest_id {
                if contest.status != "upheld" {
                    return Err(format!(
                        "contest {contest_id} has status '{}'; only 'upheld' contests may be amended",
                        contest.status
                    )
                    .into());
                }
                return Ok(contest);
            }
        }

        // Not found in upheld.ndjson — check if it exists in pending or rejected
        Err(format!(
            "contest {contest_id} not found in upheld.ndjson; \
             if it is pending or rejected it cannot be amended"
        )
        .into())
    }
}

/// A fake in-memory store for testing (used by tests/amend_tests.rs).
#[allow(dead_code)]
#[derive(Default)]
pub struct FakeUpheldStore {
    pub contests: Vec<UpheldContest>,
}

#[allow(dead_code)]
impl FakeUpheldStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, contest: UpheldContest) {
        self.contests.push(contest);
    }

    pub fn find_upheld(
        &self,
        contest_id: &str,
    ) -> Result<UpheldContest, Box<dyn std::error::Error>> {
        match self.contests.iter().find(|c| c.contest_id == contest_id) {
            None => Err(format!(
                "contest {contest_id} not found in upheld.ndjson"
            )
            .into()),
            Some(c) if c.status != "upheld" => Err(format!(
                "contest {contest_id} has status '{}'; only 'upheld' contests may be amended",
                c.status
            )
            .into()),
            Some(c) => Ok(c.clone()),
        }
    }
}
