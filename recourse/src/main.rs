use clap::{Parser, Subcommand};

mod amend;
mod contest;
mod feedback;
mod pulse;
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
    /// Contest a verdict (dispute / list / review)
    Contest {
        #[command(subcommand)]
        subcommand: contest::ContestCommand,
    },
    /// Propose and ship versioned ontology changesets from upheld contests
    ///
    /// Use `recourse feedback propose` to gather upheld contests into a reviewer-gated
    /// changeset proposal, then `recourse feedback ship <version> --confirm` to publish.
    Feedback(feedback::FeedbackArgs),
    /// Aggregate-only field health report: verdict distribution, contest rate,
    /// per-axiom fire counts, version drift. Privacy: never emits per-receipt rows.
    Pulse(pulse::PulseArgs),
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
        Commands::Contest { subcommand } => contest::run(subcommand),
        Commands::Feedback(args) => feedback::run(args),
        Commands::Pulse(args) => pulse::run(args),
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
