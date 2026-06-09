//! Contest store — append / load / move contests between pending/upheld/rejected.

use crate::contest::types::Contest;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub struct ContestStore {
    contests_dir: PathBuf,
}

impl ContestStore {
    pub fn new(data_dir: &Path) -> Self {
        ContestStore {
            contests_dir: data_dir.join("contests"),
        }
    }

    fn pending_path(&self) -> PathBuf {
        self.contests_dir.join("pending.ndjson")
    }

    fn upheld_path(&self) -> PathBuf {
        self.contests_dir.join("upheld.ndjson")
    }

    fn rejected_path(&self) -> PathBuf {
        self.contests_dir.join("rejected.ndjson")
    }

    /// Append a new contest record to pending.ndjson.
    pub fn append_pending(&self, contest: &Contest) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.contests_dir)?;
        let line = serde_json::to_string(contest)?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.pending_path())?;
        writeln!(f, "{}", line)?;
        Ok(())
    }

    /// Load all contests from a given NDJSON file.
    fn load_from(path: &Path) -> Result<Vec<Contest>, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let f = fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Contest>(trimmed) {
                Ok(c) => out.push(c),
                Err(e) => eprintln!("warn: skipping malformed contest line: {e}"),
            }
        }
        Ok(out)
    }

    pub fn load_pending(&self) -> Result<Vec<Contest>, Box<dyn std::error::Error>> {
        Self::load_from(&self.pending_path())
    }

    pub fn load_all(&self) -> Result<Vec<Contest>, Box<dyn std::error::Error>> {
        let mut all = Vec::new();
        all.extend(Self::load_from(&self.pending_path())?);
        all.extend(Self::load_from(&self.upheld_path())?);
        all.extend(Self::load_from(&self.rejected_path())?);
        all.sort_by(|a, b| b.ts.cmp(&a.ts));
        Ok(all)
    }

    /// Move a pending contest to upheld.ndjson with a reviewer note.
    /// Returns error if not found in pending.
    pub fn uphold(
        &self,
        contest_id: &str,
        note: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.move_contest(contest_id, "upheld", note)
    }

    /// Move a pending contest to rejected.ndjson with a reviewer note.
    pub fn reject(
        &self,
        contest_id: &str,
        note: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.move_contest(contest_id, "rejected", note)
    }

    fn move_contest(
        &self,
        contest_id: &str,
        new_status: &str,
        note: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pending = self.load_pending()?;
        let pos = pending
            .iter()
            .position(|c| c.contest_id == contest_id)
            .ok_or_else(|| {
                format!("contest {contest_id} not found in pending.ndjson")
            })?;

        // Build the updated record
        let mut updated = pending[pos].clone();
        updated.status = new_status.to_string();
        if let Some(n) = note {
            // Embed note into reason field as "reason + [review note: …]"
            updated.reason = format!("{} [review note: {}]", updated.reason, n);
        }

        // Rewrite pending without this contest
        let remaining: Vec<&Contest> = pending
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != pos)
            .map(|(_, c)| c)
            .collect();
        self.rewrite_file(&self.pending_path(), &remaining)?;

        // Append to destination file
        fs::create_dir_all(&self.contests_dir)?;
        let dest = if new_status == "upheld" {
            self.upheld_path()
        } else {
            self.rejected_path()
        };
        let line = serde_json::to_string(&updated)?;
        let mut f = OpenOptions::new().create(true).append(true).open(dest)?;
        writeln!(f, "{}", line)?;

        Ok(())
    }

    fn rewrite_file(
        &self,
        path: &Path,
        contests: &[&Contest],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if contests.is_empty() {
            // Truncate to empty
            fs::write(path, b"")?;
            return Ok(());
        }
        let mut content = String::new();
        for c in contests {
            content.push_str(&serde_json::to_string(c)?);
            content.push('\n');
        }
        fs::write(path, content)?;
        Ok(())
    }
}
