mod args;
mod build;
mod compare;
mod dataset;
mod fs_utils;
mod logger;
mod models;
mod runner;
mod server;
mod viewport;
mod webdriver;

pub use self::args::ScreenshotterArgs;
pub use runner::run;

pub fn build_katex() -> color_eyre::eyre::Result<()> {
    build::ensure_katex_dist_assets(&dataset::workspace_root()?, args::BuildMode::Always)
}
