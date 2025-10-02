use std::net::Ipv4Addr;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use color_eyre::eyre::{Context, Report, Result, bail, eyre};
use thirtyfour::common::capabilities::chromium::ChromiumLikeCapabilities;
use thirtyfour::common::capabilities::desiredcapabilities::CapabilitiesHelper;
use thirtyfour::common::capabilities::firefox::FirefoxPreferences;
use thirtyfour::{Capabilities, DesiredCapabilities, WebDriver};
use tokio::time::sleep;

use crate::screenshotter::args::{BrowserKind, ScreenshotterArgs, VIEWPORT_HEIGHT, VIEWPORT_WIDTH};

pub async fn start_webdriver(
    args: &ScreenshotterArgs,
    browser: BrowserKind,
) -> Result<(WebDriver, Option<Child>, String)> {
    if let Some(url) = &args.webdriver {
        let driver = connect_webdriver(url, browser, args.headless).await?;
        return Ok((driver, None, url.clone()));
    }

    if matches!(browser, BrowserKind::Safari) && !cfg!(target_os = "macos") {
        bail!("Safari automation is only supported on macOS hosts");
    }

    let port = match args.webdriver_port {
        Some(port) => port,
        None => pick_free_port()?,
    };
    let binary = match browser {
        BrowserKind::Chrome => args.driver.as_str(),
        BrowserKind::Firefox => args.geckodriver.as_str(),
        BrowserKind::Safari => args.safaridriver.as_str(),
    };
    let mut child = spawn_webdriver_process(binary, browser, port)?;
    let url = format!("http://127.0.0.1:{port}");

    let driver = match connect_webdriver(&url, browser, args.headless).await {
        Ok(driver) => driver,
        Err(err) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }
    };

    Ok((driver, Some(child), url))
}

pub fn pick_free_port() -> Result<u16> {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map(|listener| listener.local_addr().map(|addr| addr.port()))
        .and_then(|res| res)
        .context("failed to acquire a free TCP port")
}

fn spawn_webdriver_process(binary: &str, browser: BrowserKind, port: u16) -> Result<Child> {
    let mut cmd = Command::new(binary);
    cmd.arg(format!("--port={port}"));
    if matches!(browser, BrowserKind::Chrome) {
        cmd.arg("--disable-dev-shm-usage");
    }
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.spawn()
        .with_context(|| format!("failed to launch {binary}"))
}

async fn connect_webdriver(url: &str, browser: BrowserKind, headless: bool) -> Result<WebDriver> {
    let caps: Capabilities = match browser {
        BrowserKind::Chrome => {
            let mut caps = DesiredCapabilities::chrome();
            caps.set_no_sandbox().map_err(Report::from)?;
            caps.set_disable_dev_shm_usage().map_err(Report::from)?;
            caps.set_disable_gpu().map_err(Report::from)?;
            if headless {
                caps.add_arg("--headless=new").map_err(Report::from)?;
            }
            caps.add_arg(&format!(
                "--window-size={},{}",
                VIEWPORT_WIDTH, VIEWPORT_HEIGHT
            ))
            .map_err(Report::from)?;
            caps.add_arg("--disable-infobars").map_err(Report::from)?;
            caps.add_arg("--no-first-run").map_err(Report::from)?;
            caps.add_arg("--no-default-browser-check")
                .map_err(Report::from)?;
            caps.add_arg("--force-device-scale-factor=1")
                .map_err(Report::from)?;
            caps.add_arg("--hide-scrollbars").map_err(Report::from)?;
            caps.accept_insecure_certs(true).map_err(Report::from)?;
            caps.into()
        }
        BrowserKind::Firefox => {
            let mut caps = DesiredCapabilities::firefox();
            if headless {
                caps.set_headless().map_err(Report::from)?;
            }
            caps.accept_insecure_certs(true).map_err(Report::from)?;

            let mut prefs = FirefoxPreferences::new();
            prefs
                .set("layout.css.devPixelsPerPx", "1.0")
                .map_err(Report::from)?;
            caps.set_preferences(prefs).map_err(Report::from)?;

            caps.into()
        }
        BrowserKind::Safari => {
            if headless {
                eprintln!(
                    "Warning: Safari WebDriver does not support headless mode; launching normally."
                );
            }
            let mut caps = DesiredCapabilities::safari();
            caps.accept_insecure_certs(true).map_err(Report::from)?;
            caps.into()
        }
    };

    let mut last_err = None;
    for _ in 0..40 {
        match WebDriver::new(url, caps.clone()).await {
            Ok(driver) => return Ok(driver),
            Err(err) => {
                last_err = Some(err);
                sleep(Duration::from_millis(250)).await;
            }
        }
    }

    if let Some(err) = last_err {
        Err(eyre!("failed to connect to WebDriver at {url}: {err}"))
    } else {
        Err(eyre!("failed to connect to WebDriver at {url}"))
    }
}

pub fn ensure_output_dirs(root: &camino::Utf8Path) -> Result<()> {
    use crate::screenshotter::args::{ARTIFACT_ROOT, DIFF_DIR, HTML_DIR, NEW_DIR};

    let artifact_root = root.join(ARTIFACT_ROOT);
    std::fs::create_dir_all(artifact_root.as_std_path())?;
    std::fs::create_dir_all(root.join(NEW_DIR).as_std_path())?;
    std::fs::create_dir_all(root.join(DIFF_DIR).as_std_path())?;
    std::fs::create_dir_all(root.join(HTML_DIR).as_std_path())?;
    Ok(())
}
