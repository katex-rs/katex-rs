use anyhow::{Context, Result, bail, ensure};
use image::GenericImageView;
use indicatif::ProgressBar;
use serde_json::json;
use thirtyfour::WebDriver;
use thirtyfour::extensions::cdp::ChromeDevTools;
use url::form_urlencoded::byte_serialize;

use crate::screenshotter::args::{
    BrowserKind, VIEWPORT_CALIBRATION_ATTEMPTS, VIEWPORT_HEIGHT, VIEWPORT_WIDTH,
};
use crate::screenshotter::compare::encode_rgba_png;
use crate::screenshotter::logger::{Logger, WarnLevel};
use crate::screenshotter::models::Screenshot;

pub async fn calibrate_browser_viewport(
    logger: &Logger,
    driver: &WebDriver,
    browser: BrowserKind,
) -> Result<()> {
    logger.detail(None, format!("Calibrating {browser} viewport"));

    let calibration_url = viewport_calibration_data_url();
    driver
        .goto(&calibration_url)
        .await
        .map_err(anyhow::Error::from)?;

    let mut target_width = VIEWPORT_WIDTH as i32;
    let mut target_height = VIEWPORT_HEIGHT as i32;

    for attempt in 0..VIEWPORT_CALIBRATION_ATTEMPTS {
        let width = target_width.max(1) as u32;
        let height = target_height.max(1) as u32;

        driver
            .set_window_rect(0, 0, width, height)
            .await
            .map_err(anyhow::Error::from)?;

        if matches!(browser, BrowserKind::Chrome)
            && let Err(err) = driver
                .execute(
                    &format!("window.resizeTo({}, {});", width, height),
                    Vec::<serde_json::Value>::new(),
                )
                .await
                .map_err(anyhow::Error::from)
        {
            logger.warn(format!("Failed to request Chrome resize: {err}"));
        }

        let png = driver
            .screenshot_as_png()
            .await
            .map_err(anyhow::Error::from)?;
        let (actual_width, actual_height) = png_dimensions(&png)?;

        if actual_width == VIEWPORT_WIDTH && actual_height == VIEWPORT_HEIGHT {
            if attempt > 0 {
                logger.detail(
                    None,
                    format!(
                        "Adjusted viewport to {}x{} (attempt {})",
                        width,
                        height,
                        attempt + 1
                    ),
                );
            }
            return Ok(());
        }

        let delta_width = VIEWPORT_WIDTH as i32 - actual_width as i32;
        let delta_height = VIEWPORT_HEIGHT as i32 - actual_height as i32;

        target_width += delta_width;
        target_height += delta_height;
    }

    bail!(
        "{} could not reach a {}x{} viewport after {} attempts",
        browser,
        VIEWPORT_WIDTH,
        VIEWPORT_HEIGHT,
        VIEWPORT_CALIBRATION_ATTEMPTS
    );
}

pub async fn configure_chrome_viewport(driver: &WebDriver) -> Result<()> {
    let devtools = ChromeDevTools::new(driver.handle.clone());

    devtools
        .execute_cdp_with_params(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "mobile": false,
                "deviceScaleFactor": 1,
                "width": VIEWPORT_WIDTH,
                "height": VIEWPORT_HEIGHT,
                "screenWidth": VIEWPORT_WIDTH,
                "screenHeight": VIEWPORT_HEIGHT,
            }),
        )
        .await
        .map_err(anyhow::Error::from)?;

    devtools
        .execute_cdp_with_params(
            "Emulation.setVisibleSize",
            json!({
                "width": VIEWPORT_WIDTH,
                "height": VIEWPORT_HEIGHT,
            }),
        )
        .await
        .map_err(anyhow::Error::from)?;

    Ok(())
}

pub fn normalize_viewport_screenshot(
    logger: &Logger,
    progress: Option<&ProgressBar>,
    data: &[u8],
    browser: BrowserKind,
) -> Result<Screenshot> {
    let image = image::load_from_memory(data).context("failed to decode screenshot PNG")?;
    let (width, height) = image.dimensions();

    if width == VIEWPORT_WIDTH && height == VIEWPORT_HEIGHT {
        let rgba = image.to_rgba8();
        return Ok(Screenshot {
            png: data.to_vec(),
            image: rgba,
        });
    }

    logger.warn_with_progress(
        progress,
        WarnLevel::Low,
        format!(
            "{} screenshot produced {}x{}; normalizing to {}x{}",
            browser, width, height, VIEWPORT_WIDTH, VIEWPORT_HEIGHT
        ),
    );

    let mut canvas = image::ImageBuffer::from_pixel(
        VIEWPORT_WIDTH,
        VIEWPORT_HEIGHT,
        image::Rgba([255u8, 255u8, 255u8, 255u8]),
    );

    let copy_width = width.min(VIEWPORT_WIDTH);
    let copy_height = height.min(VIEWPORT_HEIGHT);
    let region = image.crop_imm(0, 0, copy_width, copy_height).to_rgba8();
    for (x, y, pixel) in region.enumerate_pixels() {
        canvas.put_pixel(x, y, *pixel);
    }

    let png = encode_rgba_png(&canvas).context("failed to encode normalized screenshot PNG")?;

    Ok(Screenshot { png, image: canvas })
}

fn viewport_calibration_data_url() -> String {
    let html = "<!DOCTYPE html><html><head><style>html,body{width:100%;height:100%;margin:0;padding:0;overflow:hidden;}</style></head><body></body></html>";
    let encoded: String = byte_serialize(html.as_bytes()).collect();
    format!("data:text/html,{}", encoded)
}

fn png_dimensions(data: &[u8]) -> Result<(u32, u32)> {
    const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    ensure!(data.len() >= 24, "screenshot PNG is truncated");
    ensure!(data.starts_with(&PNG_MAGIC), "unexpected screenshot format");
    ensure!(&data[12..16] == b"IHDR", "missing PNG header chunk");

    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

    Ok((width, height))
}
