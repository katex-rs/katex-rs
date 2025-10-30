mod extract_data;
mod screenshotter;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

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
    /// Run the browser-based screenshotter tests using WebDriver.
    Screenshotter(Box<screenshotter::ScreenshotterArgs>),
    /// Regenerate JSON data extracted from the upstream KaTeX repository.
    ExtractData(extract_data::ExtractDataArgs),
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Command::Screenshotter(args) => screenshotter::run(*args),
        Command::ExtractData(args) => extract_data::run(args),
    }
}
