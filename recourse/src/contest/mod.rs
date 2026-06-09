//! `recourse contest` subcommands — dispute a verdict, list disputes, review them.

pub mod ls;
pub mod review;
pub mod store;
pub mod submit;
pub mod types;

#[allow(unused_imports)]
pub use store::ContestStore;

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// `recourse contest <subcommand>`
#[derive(Subcommand)]
pub enum ContestCommand {
    /// Dispute a verdict: contest submit <receipt-id> --expected <verdict> --reason "<text>"
    Submit(SubmitArgs),
    /// List contests (reviewer view)
    Ls(LsArgs),
    /// Review a pending contest: move it to upheld or rejected (human action only)
    Review(ReviewArgs),
}

#[derive(Args)]
pub struct SubmitArgs {
    /// Receipt ID (ULID) to dispute
    pub receipt_id: String,

    /// The verdict you believe is correct: allow, flag, or deny
    #[arg(long)]
    pub expected: String,

    /// Why you believe the verdict is wrong (required, non-empty)
    #[arg(long)]
    pub reason: String,

    /// Override data directory
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct LsArgs {
    /// Show only pending contests (default when neither flag given)
    #[arg(long, conflicts_with = "all")]
    pub pending: bool,

    /// Show all contests regardless of status
    #[arg(long)]
    pub all: bool,

    /// Output format: pretty (default) or json
    #[arg(long, default_value = "pretty")]
    pub format: String,

    /// Override data directory
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct ReviewArgs {
    /// Contest ID (ULID)
    pub contest_id: String,

    /// Uphold the contest (move to upheld.ndjson)
    #[arg(long, conflicts_with = "reject")]
    pub uphold: bool,

    /// Reject the contest (move to rejected.ndjson)
    #[arg(long)]
    pub reject: bool,

    /// Optional reviewer note
    #[arg(long)]
    pub note: Option<String>,

    /// Override data directory
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

pub fn run(cmd: ContestCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ContestCommand::Submit(args) => {
            submit::cmd_contest_submit(&args.receipt_id, &args.expected, &args.reason, args.data_dir)
        }
        ContestCommand::Ls(args) => {
            let filter = if args.all { "all" } else { "pending" };
            ls::cmd_contest_ls(filter, &args.format, args.data_dir)
        }
        ContestCommand::Review(args) => {
            review::cmd_contest_review(
                &args.contest_id,
                args.uphold,
                args.reject,
                args.note.as_deref(),
                args.data_dir,
            )
        }
    }
}
