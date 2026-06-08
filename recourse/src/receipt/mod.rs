pub mod canon;
mod emit;
mod ls;
pub mod schema;
mod show;
pub mod store;
pub mod types;

pub use emit::cmd_emit;
pub use ls::cmd_ls;
pub use show::cmd_show;

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ReceiptCommand {
    /// Read verdict JSON on stdin and append a receipt
    Emit {
        /// Also write raw canonical action to actions/<digest>.json
        #[arg(long, default_value_t = false)]
        store_raw: bool,

        /// Override data directory (default: $XDG_DATA_HOME/recourse)
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// Pretty-print a receipt by ID
    Show {
        /// Receipt ID (ULID)
        id: String,

        /// Output format: pretty (default) or json
        #[arg(long, default_value = "pretty")]
        format: String,

        /// Override data directory
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// List receipts
    Ls {
        /// Filter to receipts from the last N days, e.g. 30d
        #[arg(long)]
        since: Option<String>,

        /// Filter by verdict: allow, flag, deny
        #[arg(long)]
        verdict: Option<String>,

        /// Output format: pretty (default) or json
        #[arg(long, default_value = "pretty")]
        format: String,

        /// Override data directory
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

pub fn run(cmd: ReceiptCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ReceiptCommand::Emit { store_raw, data_dir } => cmd_emit(store_raw, data_dir),
        ReceiptCommand::Show { id, format, data_dir } => cmd_show(&id, &format, data_dir),
        ReceiptCommand::Ls { since, verdict, format, data_dir } => {
            cmd_ls(since.as_deref(), verdict.as_deref(), &format, data_dir)
        }
    }
}
