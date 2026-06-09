//! `recourse feedback` — turn upheld+amended contests into versioned ontology changesets.
//!
//! Two sub-subcommands:
//!   - `propose [--since <dur>]` — gather upheld contests since the last version and
//!     render a `changeset.toml` + `CHANGELOG-<version>.md` into a reviewer-gated dir.
//!     Never publishes. Zero new contests → "nothing to ship", clean exit.
//!   - `ship <version> --confirm` — run `tribunal gate` against the amended corpus,
//!     then invoke `herald-market` to publish. Without `--confirm` → dry preview only.

pub mod herald;
pub mod propose;
pub mod ship;
pub mod version;

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct FeedbackArgs {
    #[command(subcommand)]
    pub subcommand: FeedbackCommand,

    /// Override data directory (default: $XDG_DATA_HOME/recourse)
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum FeedbackCommand {
    /// Gather upheld contests and propose a new versioned ontology changeset.
    ///
    /// Writes changeset.toml and CHANGELOG-<version>.md into the reviewer-gated
    /// proposals dir. Never publishes to herald-market. Exits cleanly with
    /// "nothing to ship" when there are no new upheld contests since the last version.
    Propose {
        /// Only include contests upheld since this duration ago (e.g. "7d", "30d").
        /// Omit to include all contests since the last shipped version.
        #[arg(long)]
        since: Option<String>,

        /// Path to the tribunal-corpus root (for reading existing field cases).
        #[arg(long)]
        tribunal_corpus: Option<PathBuf>,

        /// If specified, also record the upstream target for --upstream intent.
        /// The actual PR mechanics are out of scope (see --help for details).
        ///
        /// NOTE: --upstream records intent only. Submitting an upstream PR is deferred
        /// to a successor PRD. This flag does not open a PR or mutate the marketplace.
        #[arg(long)]
        upstream: Option<String>,

        /// Directory to write the proposal into (default: <data_dir>/proposals).
        #[arg(long)]
        proposals_dir: Option<PathBuf>,
    },

    /// Ship a proposed version to herald-market.
    ///
    /// Runs `tribunal gate` against the amended corpus before any publish.
    /// A gate failure blocks the publish and exits non-zero.
    /// Without `--confirm`, performs a dry preview only.
    Ship {
        /// Version to ship (must match a file in the proposals dir).
        version: String,

        /// Actually publish via herald-market. Without this flag, only a dry preview
        /// is printed and nothing is published.
        #[arg(long)]
        confirm: bool,

        /// Path to the tribunal-corpus root.
        #[arg(long)]
        tribunal_corpus: PathBuf,

        /// Directory containing the proposal files (default: <data_dir>/proposals).
        #[arg(long)]
        proposals_dir: Option<PathBuf>,
    },
}

pub fn run(args: FeedbackArgs) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = crate::receipt::store::data_dir(args.data_dir.as_deref());

    match args.subcommand {
        FeedbackCommand::Propose {
            since,
            tribunal_corpus,
            upstream,
            proposals_dir,
        } => propose::run(
            &data_dir,
            since.as_deref(),
            tribunal_corpus.as_deref(),
            upstream.as_deref(),
            proposals_dir.as_deref(),
        ),
        FeedbackCommand::Ship {
            version,
            confirm,
            tribunal_corpus,
            proposals_dir,
        } => {
            let runner = ship::TribunalGateRunner::new_system();
            let herald = herald::HeraldMarketClient::new_system();
            ship::run_with(
                &data_dir,
                &version,
                confirm,
                &tribunal_corpus,
                proposals_dir.as_deref(),
                &runner,
                &herald,
            )
        }
    }
}
