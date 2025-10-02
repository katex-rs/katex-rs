use clap::{Parser, ValueEnum};
use strum_macros::Display;

pub const PAGE_PATH: &str = "/screenshot.html";
pub const BASELINE_DIR: &str = "KaTeX/test/screenshotter/images";
pub const ARTIFACT_ROOT: &str = "artifacts/screenshots";
pub const NEW_DIR: &str = "artifacts/screenshots/new";
pub const DIFF_DIR: &str = "artifacts/screenshots/diff";
pub const HTML_DIR: &str = "artifacts/screenshots/html";

pub const VIEWPORT_WIDTH: u32 = 1024;
pub const VIEWPORT_HEIGHT: u32 = 768;
pub const VIEWPORT_CALIBRATION_ATTEMPTS: u32 = 6;

pub const DEFAULT_BROWSERS: [BrowserKind; 3] = [
    BrowserKind::Safari,
    BrowserKind::Firefox,
    BrowserKind::Chrome,
];

#[derive(Copy, Clone, Debug, ValueEnum, Eq, PartialEq)]
pub enum BuildMode {
    Auto,
    Always,
    Never,
}

#[derive(Copy, Clone, Debug, ValueEnum, Eq, PartialEq, Hash, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum BrowserKind {
    #[strum(to_string = "Chrome", serialize = "chrome", serialize = "chromium")]
    #[value(alias("chromium"))]
    Chrome,
    #[strum(to_string = "Firefox")]
    Firefox,
    #[strum(to_string = "Safari")]
    Safari,
}

impl BrowserKind {
    pub fn slug(self) -> &'static str {
        match self {
            BrowserKind::Chrome => "chrome",
            BrowserKind::Firefox => "firefox",
            BrowserKind::Safari => "safari",
        }
    }

    pub fn screenshot_suffix(self) -> String {
        format!("-{}.png", self.slug())
    }

    pub fn diff_suffix(self) -> String {
        format!("-{}-diff.png", self.slug())
    }
}

#[derive(Copy, Clone, Debug, ValueEnum, Eq, PartialEq)]
pub enum CompareTolerance {
    Strict,
    Normal,
    Tolerant,
}

#[derive(Parser, Debug, Clone)]
pub struct ScreenshotterArgs {
    /// Browser engines to exercise (comma-separated).
    #[arg(
        long = "browser",
        value_enum,
        value_delimiter = ',',
        default_values_t = DEFAULT_BROWSERS
    )]
    pub browsers: Vec<BrowserKind>,
    /// Filter cases to include (comma-separated substrings).
    #[arg(long, value_delimiter = ',')]
    pub include: Option<Vec<String>>,
    /// Filter cases to exclude (comma-separated substrings).
    #[arg(long, value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,
    /// Retry attempts per case.
    #[arg(long, default_value_t = 1)]
    pub attempts: u32,
    /// Extra wait after window.__ready becomes true (seconds).
    #[arg(long, default_value_t = 0.0)]
    pub wait: f64,
    /// Timeout waiting for window.__ready (milliseconds).
    #[arg(long, default_value_t = 15_000)]
    pub timeout: u64,
    /// Restrict execution to a single named case.
    #[arg(long)]
    pub case: Option<String>,
    /// Render an ad-hoc TeX expression without loading the dataset.
    #[arg(long)]
    pub tex: Option<String>,
    /// Preferred HTTP port for the static server (0 chooses a free port).
    #[arg(long, default_value_t = 0)]
    pub port: u16,
    /// Connect to an existing WebDriver endpoint instead of launching
    /// chromedriver.
    #[arg(long)]
    pub webdriver: Option<String>,
    /// Path to the chromedriver binary when spawning automatically.
    #[arg(long, default_value = "chromedriver")]
    pub driver: String,
    /// Path to the geckodriver (Firefox) binary when spawning automatically.
    #[arg(
        long = "geckodriver",
        alias = "firefox-driver",
        default_value = "geckodriver"
    )]
    pub geckodriver: String,
    /// Path to the safaridriver binary when spawning automatically (macOS
    /// only).
    #[arg(
        long = "safaridriver",
        alias = "safari-driver",
        default_value = "safaridriver"
    )]
    pub safaridriver: String,
    /// Override the chromedriver port (random free port by default).
    #[arg(long)]
    pub webdriver_port: Option<u16>,
    /// Run Chrome in headless mode (set to false to show the browser).
    #[arg(long, default_value_t = true)]
    pub headless: bool,
    /// Build mode for wasm-pack and KaTeX assets (auto builds when missing by
    /// default).
    #[arg(long, value_enum, default_value_t = BuildMode::Auto)]
    pub build: BuildMode,
    /// Pixel-diff tolerance profile to apply during comparisons.
    #[arg(long, value_enum, default_value_t = CompareTolerance::Normal)]
    pub tolerance: CompareTolerance,
    /// When set, capture the rendered HTML for failing cases using the default
    /// implementation and the fallback JavaScript implementation.
    #[arg(long = "html-on-failure", default_value_t = false)]
    pub html_on_failure: bool,
    /// Allow falling back to JS-vs-WASM comparisons when baselines are missing
    /// or mismatched.
    #[arg(long = "allow-js-fallback", default_value_t = false)]
    pub allow_js_fallback: bool,
}
