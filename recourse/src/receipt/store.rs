use crate::receipt::types::Receipt;
use chrono::{DateTime, Datelike, Utc};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Resolve the data directory: $XDG_DATA_HOME/recourse or ~/.local/share/recourse
pub fn data_dir(override_dir: Option<&Path>) -> PathBuf {
    if let Some(d) = override_dir {
        return d.to_path_buf();
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg).join("recourse")
    } else {
        dirs_or_home().join(".local").join("share").join("recourse")
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Path to the monthly NDJSON sink for a given timestamp
pub fn receipt_sink(base: &Path, ts: &DateTime<Utc>) -> PathBuf {
    let name = format!("receipts/{:04}-{:02}.ndjson", ts.year(), ts.month());
    base.join(name)
}

/// Append one receipt to the monthly NDJSON sink (atomic enough: single append write)
pub fn append_receipt(base: &Path, receipt: &Receipt) -> Result<(), Box<dyn std::error::Error>> {
    let sink = receipt_sink(base, &receipt.ts);
    if let Some(parent) = sink.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(receipt)?;
    let mut f = OpenOptions::new().create(true).append(true).open(&sink)?;
    writeln!(f, "{}", line)?;
    Ok(())
}

/// Write the canonical action JSON to actions/<digest>.json
pub fn store_raw_action(base: &Path, digest: &str, canonical_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    let actions_dir = base.join("actions");
    fs::create_dir_all(&actions_dir)?;
    // digest is "blake3:<hex>"; use hex part as filename
    let hex = digest.strip_prefix("blake3:").unwrap_or(digest);
    let path = actions_dir.join(format!("{}.json", hex));
    fs::write(path, canonical_json)?;
    Ok(())
}

/// Load all receipts from all monthly sinks, sorted newest-first
pub fn load_all_receipts(base: &Path) -> Result<Vec<Receipt>, Box<dyn std::error::Error>> {
    let receipts_dir = base.join("receipts");
    if !receipts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut all: Vec<Receipt> = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&receipts_dir)?
        .filter_map(|e| e.ok())
        .collect();
    // Sort filenames so we process in month order (newest last, then we reverse)
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ndjson") {
            continue;
        }
        let f = fs::File::open(&path)?;
        let reader = BufReader::new(f);
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Receipt>(trimmed) {
                Ok(r) => all.push(r),
                Err(e) => eprintln!("warn: skipping malformed receipt line: {e}"),
            }
        }
    }

    // Newest first by timestamp
    all.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(all)
}

/// Find a single receipt by ID
pub fn find_receipt(base: &Path, id: &str) -> Result<Option<Receipt>, Box<dyn std::error::Error>> {
    let all = load_all_receipts(base)?;
    Ok(all.into_iter().find(|r| r.receipt_id == id))
}
