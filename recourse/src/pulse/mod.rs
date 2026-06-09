//! `recourse pulse` — aggregate-only health report over the local receipt+contest sinks.
//!
//! Privacy invariant: this module never reads `actions/` and never emits
//! `receipt_id`, `action_digest`, or any per-action row in any output mode.

pub mod report;
pub mod since;

pub use report::{build_report, build_report_with_known_tenets, ContestEntry, PulseReport};
pub use since::parse_since;

use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct PulseArgs {
    /// Only count receipts/contests from the last window, e.g. 30d, 7d, 24h
    #[arg(long)]
    pub since: Option<String>,

    /// Output format: table (default) or json
    #[arg(long, default_value = "table")]
    pub format: String,

    /// Write the aggregate report to a file (aggregates only — no per-receipt rows)
    #[arg(long)]
    pub export: Option<PathBuf>,

    /// Override data directory (default: $XDG_DATA_HOME/recourse)
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

pub fn run(args: PulseArgs) -> Result<(), Box<dyn std::error::Error>> {
    let base = crate::receipt::store::data_dir(args.data_dir.as_deref());
    let cutoff = args.since.as_deref().map(parse_since).transpose()?;

    // PRIVACY: we intentionally never open base/actions/ here.
    // Only receipts/ and upheld.ndjson are read.
    let receipts = crate::receipt::store::load_all_receipts(&base)?;

    // Load contests from upheld.ndjson (all statuses — we need both upheld and rejected counts)
    let contests = load_all_contests(&base);

    let report = build_report(&receipts, &contests, cutoff)?;

    // Output
    let output = match args.format.as_str() {
        "json" => serde_json::to_string_pretty(&report)?,
        _ => format_table(&report),
    };
    println!("{output}");

    // Export
    if let Some(export_path) = args.export {
        // Serialize to JSON and assert private fields absent
        let json_str = serde_json::to_string_pretty(&report)?;
        assert_no_private_fields(&json_str);
        std::fs::write(&export_path, &json_str)?;
        eprintln!("pulse: export written to {}", export_path.display());
    }

    Ok(())
}

/// Load all contests from upheld.ndjson — both upheld and rejected entries.
fn load_all_contests(base: &std::path::Path) -> Vec<ContestEntry> {
    let path = base.join("upheld.ndjson");
    if !path.exists() {
        return Vec::new();
    }
    use std::io::{BufRead, BufReader};
    let f = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<ContestEntry>(trimmed) {
            Ok(c) => out.push(c),
            Err(e) => eprintln!("warn: skipping malformed contest line: {e}"),
        }
    }
    out
}

/// Assert that the export JSON contains none of the private fields.
fn assert_no_private_fields(json: &str) {
    for field in &["receipt_id", "action_digest"] {
        if json.contains(field) {
            panic!("pulse export must not contain private field '{field}'");
        }
    }
}

fn format_table(r: &PulseReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "=== recourse pulse (window: {}) ===\n",
        r.window
    ));
    out.push_str(&format!("Total receipts: {}\n\n", r.total_receipts));

    out.push_str("Verdict distribution:\n");
    for vd in &r.verdict_distribution {
        out.push_str(&format!(
            "  {:6}  {:5}  {:5.1}%\n",
            vd.verdict, vd.count, vd.pct
        ));
    }

    out.push('\n');
    out.push_str(&format!(
        "Contest rate:  {:.1}%  ({} / {})\n",
        r.contest_rate_pct, r.contest_count, r.total_receipts
    ));
    out.push_str(&format!(
        "Uphold rate:   {:.1}%  ({} upheld / {} reviewed)\n",
        r.uphold_rate_pct, r.upheld_count, r.reviewed_count
    ));

    out.push('\n');
    out.push_str("Axiom fire counts (sorted desc):\n");
    for af in &r.axiom_fire_counts {
        let flag = if af.zero_fired { "  *** UNDER-FIRED ***" } else { "" };
        out.push_str(&format!("  {:40}  {:5}{}\n", af.tenet, af.count, flag));
    }

    out.push('\n');
    out.push_str("Version drift (verdict dist by ontology_version):\n");
    for vv in &r.version_drift {
        out.push_str(&format!("  version {}:\n", vv.ontology_version));
        for vd in &vv.verdict_distribution {
            out.push_str(&format!(
                "    {:6}  {:5}  {:5.1}%\n",
                vd.verdict, vd.count, vd.pct
            ));
        }
    }

    out
}
