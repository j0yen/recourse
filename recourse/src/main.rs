use clap::{Parser, Subcommand};

mod amend;
mod feedback;
mod receipt;

#[derive(Parser)]
#[command(name = "recourse", about = "Durability and field-improvement loop for ousia-guard verdicts")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Receipt subcommands
    Receipt {
        #[command(subcommand)]
        subcommand: receipt::ReceiptCommand,
    },
    /// Amend an upheld field contest into a tribunal-corpus case
    Amend(amend::AmendArgs),
    /// Propose and ship versioned ontology changesets from upheld contests
    ///
    /// Use `recourse feedback propose` to gather upheld contests into a reviewer-gated
    /// changeset proposal, then `recourse feedback ship <version> --confirm` to publish.
    Feedback(feedback::FeedbackArgs),
}

fn main() {
    // SIGPIPE safety: reset to default so `recourse ls | head` exits cleanly
    #[cfg(unix)]
    {
        // SAFETY: called before any threads are spawned
        unsafe {
            libc_sigpipe_reset();
        }
    }

    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Receipt { subcommand } => receipt::run(subcommand),
        Commands::Amend(args) => amend::run(args),
        Commands::Feedback(args) => feedback::run(args),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
unsafe fn libc_sigpipe_reset() {
    // Reset SIGPIPE to SIG_DFL so broken-pipe exits instead of panicking
    // libc is not in deps; use raw syscall via signal(2)
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    signal(SIGPIPE, SIG_DFL);
}
