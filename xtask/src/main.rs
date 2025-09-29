#![feature(portable_simd)]

mod flamegraph;
mod screenshotter;

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
    /// Run the browser-based screenshotter tests using WebDriver.
    Screenshotter(screenshotter::ScreenshotterArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Flamegraph(args) => flamegraph::run(args),
        Command::Screenshotter(args) => screenshotter::run(args),
    }
}
