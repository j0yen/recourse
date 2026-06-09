//! `recourse feedback propose` implementation.
//!
//! Gathers upheld contests (from upheld.ndjson) that have been amended into corpus
//! cases (i.e. a `field-<id>/` exists under the tribunal corpus), plus the pulse
//! summary if available, and renders:
//!   - `<proposals_dir>/changeset-<version>.toml`
//!   - `<proposals_dir>/CHANGELOG-<version>.md`
//!
//! Never invokes herald-market (publish_call_count must remain 0 after this runs).
//! AC5: if there are zero new upheld contests, prints "nothing to ship" and exits cleanly.

use super::version::{read_current, Version};
use crate::amend::contest::UpheldContest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Shape of the generated `changeset-<version>.toml`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangesetToml {
    /// The proposed new ontology semver.
    pub version: String,
    /// Field corpus cases included (each is "field-<contest-id>").
    pub field_cases: Vec<String>,
    /// Pulse deltas motivating the bump (key → delta description).
    pub pulse_deltas: Vec<String>,
    /// If --upstream was specified, the upstream target (intent only).
    pub upstream_target: Option<String>,
    /// Note about upstream PR mechanics being out of scope.
    pub upstream_note: Option<String>,
}

/// Load all upheld contests from `<data_dir>/upheld.ndjson`.
fn load_all_upheld(data_dir: &Path) -> Result<Vec<UpheldContest>, Box<dyn std::error::Error>> {
    let upheld_path = data_dir.join("upheld.ndjson");
    if !upheld_path.exists() {
        return Ok(Vec::new());
    }
    let f = fs::File::open(&upheld_path)
        .map_err(|e| format!("cannot open {}: {e}", upheld_path.display()))?;
    let reader = BufReader::new(f);
    let mut contests = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<UpheldContest>(trimmed) {
            Ok(c) if c.status == "upheld" => contests.push(c),
            Ok(_) => {}
            Err(e) => eprintln!("warn: skipping malformed upheld line: {e}"),
        }
    }
    Ok(contests)
}

/// Check whether the given contest has been amended into the corpus
/// (i.e. `<tribunal_corpus>/cases/field-<id>/` exists).
fn is_amended(tribunal_corpus: &Path, contest_id: &str) -> bool {
    let case_dir = tribunal_corpus.join("cases").join(format!("field-{contest_id}"));
    case_dir.exists()
}

/// Read pulse summary lines from `<data_dir>/pulse/latest.txt` if present.
fn read_pulse_summary(data_dir: &Path) -> Vec<String> {
    let path = data_dir.join("pulse").join("latest.txt");
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(s) => s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Core propose logic — injectable for testing.
pub fn run_with(
    data_dir: &Path,
    _since: Option<&str>,
    tribunal_corpus: Option<&Path>,
    upstream: Option<&str>,
    proposals_dir: Option<&Path>,
) -> Result<ProposalResult, Box<dyn std::error::Error>> {
    let current_version = read_current(data_dir)?;

    // Gather upheld contests that have been amended into the corpus.
    let all_upheld = load_all_upheld(data_dir)?;
    let amended: Vec<UpheldContest> = if let Some(corpus) = tribunal_corpus {
        all_upheld
            .into_iter()
            .filter(|c| is_amended(corpus, &c.contest_id))
            .collect()
    } else {
        all_upheld
    };

    // AC5: zero new upheld contests → nothing to ship.
    if amended.is_empty() {
        return Ok(ProposalResult::NothingToShip);
    }

    let proposed_version = current_version.next_minor();

    // Safety: proposed must be strictly greater than current.
    assert!(
        proposed_version.is_greater_than(&current_version),
        "proposed version {} must be > current {}",
        proposed_version,
        current_version
    );

    let field_cases: Vec<String> = amended
        .iter()
        .map(|c| format!("field-{}", c.contest_id))
        .collect();

    let pulse_deltas = read_pulse_summary(data_dir);

    let upstream_note = upstream.map(|_| {
        "Upstream PR mechanics are deferred to a successor PRD. \
         This field records intent only; no PR is opened and the local marketplace is not mutated."
            .to_string()
    });

    let changeset = ChangesetToml {
        version: proposed_version.to_string_ver(),
        field_cases: field_cases.clone(),
        pulse_deltas: pulse_deltas.clone(),
        upstream_target: upstream.map(|s| s.to_string()),
        upstream_note,
    };

    // Write proposal files.
    let proposals_path = proposals_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("proposals"));
    fs::create_dir_all(&proposals_path)?;

    let changeset_path = proposals_path.join(format!("changeset-{}.toml", proposed_version));
    let changelog_path = proposals_path.join(format!("CHANGELOG-{}.md", proposed_version));

    let changeset_content = toml::to_string_pretty(&changeset)?;
    fs::write(&changeset_path, &changeset_content)?;

    // Build CHANGELOG.
    let changelog_content = build_changelog(&proposed_version, &amended, &pulse_deltas, upstream);
    fs::write(&changelog_path, &changelog_content)?;

    Ok(ProposalResult::Proposed {
        version: proposed_version.to_string_ver(),
        field_cases,
        changeset_path,
        changelog_path,
    })
}

/// Build human-readable CHANGELOG-<version>.md.
fn build_changelog(
    version: &Version,
    contests: &[UpheldContest],
    pulse_deltas: &[String],
    upstream: Option<&str>,
) -> String {
    let mut out = format!(
        "# CHANGELOG — version {version}\n\n\
         This version ships because of the following field contests that the prior version got wrong:\n\n"
    );

    for c in contests {
        out.push_str(&format!(
            "- **{}**: {} (reviewer: {}, contestant: {})\n",
            c.contest_id, c.reason, c.reviewer, c.contestant
        ));
    }

    if !pulse_deltas.is_empty() {
        out.push_str("\n## Pulse deltas motivating this bump\n\n");
        for d in pulse_deltas {
            out.push_str(&format!("- {d}\n"));
        }
    }

    if let Some(target) = upstream {
        out.push_str(&format!(
            "\n## Upstream intent\n\n\
             Target: `{target}`\n\n\
             > NOTE: Submitting an upstream PR is deferred to a successor PRD. \
             This entry records intent only.\n"
        ));
    }

    out
}

/// Result returned by `run_with` — used in tests to assert behavior.
#[derive(Debug)]
pub enum ProposalResult {
    NothingToShip,
    Proposed {
        version: String,
        field_cases: Vec<String>,
        changeset_path: PathBuf,
        changelog_path: PathBuf,
    },
}

/// Public entry point called from `feedback::run`.
pub fn run(
    data_dir: &Path,
    since: Option<&str>,
    tribunal_corpus: Option<&Path>,
    upstream: Option<&str>,
    proposals_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    match run_with(data_dir, since, tribunal_corpus, upstream, proposals_dir)? {
        ProposalResult::NothingToShip => {
            println!("nothing to ship: no new upheld+amended contests since last version");
            Ok(())
        }
        ProposalResult::Proposed {
            version,
            field_cases,
            changeset_path,
            changelog_path,
        } => {
            println!("proposed version: {version}");
            println!("field cases: {}", field_cases.join(", "));
            println!("changeset: {}", changeset_path.display());
            println!("changelog: {}", changelog_path.display());
            println!("(review required before shipping — run `recourse feedback ship {version} --confirm`)");
            Ok(())
        }
    }
}
