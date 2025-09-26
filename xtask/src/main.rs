mod flamegraph;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Development tasks for katex-rs",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate CPU flamegraphs for the available performance harnesses.
    Flamegraph(flamegraph::FlamegraphArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Flamegraph(args) => flamegraph::run(args),
    }
}
