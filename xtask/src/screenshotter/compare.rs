use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use image::{ColorType, ImageBuffer, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};
use image_compare::{CompareError as HybridCompareError, rgba_hybrid_compare};

use crate::screenshotter::args::{CompareTolerance, DIFF_DIR};
use crate::screenshotter::models::{BaselineEntry, MismatchSeverity, Screenshot};

#[derive(Copy, Clone, Debug)]
pub struct CompareSettings {
    tolerance: CompareTolerance,
    pass_ratio: f64,
    warn_ratio: f64,
    diff_ratio: f64,
}

#[derive(Copy, Clone, Debug)]
struct CompareThresholds {
    pass_limit: u64,
    warn_limit: u64,
    diff_limit: u64,
}

#[derive(Clone, Debug)]
struct MismatchSummary {
    severity: MismatchSeverity,
    message: String,
}

#[derive(Clone, Debug)]
pub struct CompareOutcome {
    pub equal: bool,
    pub diff_pixels: Option<u64>,
    pub note: Option<String>,
    pub severity: Option<MismatchSeverity>,
    pub diff_image: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct CompareJob {
    pub screenshot: Screenshot,
    pub baseline: Option<BaselineEntry>,
    pub baseline_path: camino::Utf8PathBuf,
    pub settings: CompareSettings,
}

#[derive(Clone, Debug)]
pub struct CompareWorkResult {
    pub screenshot: Screenshot,
    pub outcome: CompareOutcome,
}

impl CompareTolerance {
    pub fn label(self) -> &'static str {
        match self {
            CompareTolerance::Strict => "strict",
            CompareTolerance::Normal => "normal",
            CompareTolerance::Tolerant => "tolerant",
        }
    }

    pub fn settings(self) -> CompareSettings {
        match self {
            CompareTolerance::Strict => CompareSettings::new(self, 0.0010, 0.0030, 0.0070),
            CompareTolerance::Normal => CompareSettings::new(self, 0.0035, 0.0085, 0.0180),
            CompareTolerance::Tolerant => CompareSettings::new(self, 0.0090, 0.0180, 0.0350),
        }
    }
}

impl CompareSettings {
    fn new(tolerance: CompareTolerance, pass_ratio: f64, warn_ratio: f64, diff_ratio: f64) -> Self {
        Self {
            tolerance,
            pass_ratio,
            warn_ratio,
            diff_ratio,
        }
    }

    pub fn summary(self) -> String {
        format!(
            "Diff tolerance: {} (pass ≤ {:.3}%, minor ≤ {:.3}%, diff artifacts ≥ {:.3}%)",
            self.tolerance.label(),
            self.pass_ratio * 100.0,
            self.warn_ratio * 100.0,
            self.diff_ratio * 100.0,
        )
    }

    fn thresholds(self, total_pixels: u64) -> CompareThresholds {
        let total = total_pixels as f64;
        let pass_limit = (self.pass_ratio * total).round() as u64;
        let warn_limit = (self.warn_ratio * total).round() as u64;
        let diff_limit = (self.diff_ratio * total).round() as u64;

        let warn_limit = warn_limit.max(pass_limit);
        let diff_limit = diff_limit.max(warn_limit);

        CompareThresholds {
            pass_limit,
            warn_limit,
            diff_limit,
        }
    }

    fn describe_mismatch(
        self,
        diff_pixels: u64,
        total_pixels: u64,
        thresholds: &CompareThresholds,
    ) -> MismatchSummary {
        let percent = if total_pixels == 0 {
            0.0
        } else {
            (diff_pixels as f64 / total_pixels as f64) * 100.0
        };

        let severity = if diff_pixels >= thresholds.diff_limit {
            MismatchSeverity::Major
        } else if diff_pixels <= thresholds.warn_limit {
            MismatchSeverity::Minor
        } else {
            MismatchSeverity::Noticeable
        };

        let tone = match severity {
            MismatchSeverity::Major => "Significant mismatch",
            MismatchSeverity::Noticeable => "Visual mismatch",
            MismatchSeverity::Minor => "Minor pixel drift",
        };

        let message = format!(
            "{tone} ({diff_pixels} diff pixels, {percent:.4}%, tolerance: {})",
            self.tolerance.label()
        );

        MismatchSummary { severity, message }
    }
}

pub fn run_compare_job(job: CompareJob) -> Result<CompareWorkResult> {
    let CompareJob {
        screenshot,
        baseline,
        baseline_path,
        settings,
    } = job;

    let outcome = compare_screenshot(
        &screenshot,
        baseline.as_ref(),
        baseline_path.as_std_path(),
        settings,
    )?;

    Ok(CompareWorkResult {
        screenshot,
        outcome,
    })
}

fn compare_screenshot(
    screenshot: &Screenshot,
    baseline: Option<&BaselineEntry>,
    baseline_path: &std::path::Path,
    settings: CompareSettings,
) -> Result<CompareOutcome> {
    let Some(baseline) = baseline else {
        return Ok(CompareOutcome {
            equal: false,
            diff_pixels: None,
            note: Some(format!(
                "Baseline missing at {} – copying artifact to {}",
                baseline_path.display(),
                DIFF_DIR
            )),
            severity: Some(MismatchSeverity::Major),
            diff_image: None,
        });
    };

    let actual_image = &screenshot.image;
    let baseline_image = &baseline.image;

    let (aw, ah) = actual_image.dimensions();
    let (bw, bh) = baseline_image.dimensions();

    if aw != bw || ah != bh {
        let diff_png = build_composite_diff(actual_image, baseline_image)?;
        return Ok(CompareOutcome {
            equal: false,
            diff_pixels: None,
            note: Some(format!(
                "Screenshot dimensions differ (actual: {}x{}, baseline: {}x{})",
                aw, ah, bw, bh
            )),
            severity: Some(MismatchSeverity::Major),
            diff_image: Some(diff_png),
        });
    }

    let similarity =
        rgba_hybrid_compare(actual_image, baseline_image).map_err(|err| match err {
            HybridCompareError::DimensionsDiffer => {
                anyhow!(
                    "image dimensions diverged during hybrid comparison despite preflight checks"
                )
            }
            HybridCompareError::CalculationFailed(msg) => {
                anyhow!("hybrid comparison failed: {msg}")
            }
        })?;

    let total_pixels = (aw as u64) * (ah as u64);
    let estimated_diff = estimate_diff_pixels(similarity.score, total_pixels);
    let thresholds = settings.thresholds(total_pixels);

    if estimated_diff <= thresholds.pass_limit {
        return Ok(CompareOutcome {
            equal: true,
            diff_pixels: Some(estimated_diff),
            note: None,
            severity: None,
            diff_image: None,
        });
    }

    let mismatch = settings.describe_mismatch(estimated_diff, total_pixels, &thresholds);
    let diff_image = if mismatch.severity == MismatchSeverity::Major {
        Some(build_composite_diff(actual_image, baseline_image)?)
    } else {
        None
    };

    Ok(CompareOutcome {
        equal: false,
        diff_pixels: Some(estimated_diff),
        note: Some(mismatch.message),
        severity: Some(mismatch.severity),
        diff_image,
    })
}

fn estimate_diff_pixels(score: f64, total_pixels: u64) -> u64 {
    let clamped = score.clamp(0.0, 1.0);
    ((1.0 - clamped) * total_pixels as f64).round() as u64
}

fn build_composite_diff(actual: &RgbaImage, baseline: &RgbaImage) -> Result<Vec<u8>> {
    let (width, height) = actual.dimensions();
    let separator = 4;
    let total_width = width * 3 + separator * 2;
    let mut canvas = ImageBuffer::from_pixel(total_width, height, Rgba([24, 24, 24, 255]));

    blit_image(&mut canvas, baseline, 0);
    fill_separator(&mut canvas, width, separator, height);
    blit_image(&mut canvas, actual, width + separator);
    fill_separator(&mut canvas, width * 2 + separator, separator, height);
    let highlight = build_highlight_view(actual, baseline);
    blit_image(&mut canvas, &highlight, width * 2 + separator * 2);

    encode_rgba_png(&canvas)
}

pub(crate) fn encode_rgba_png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    PngEncoder::new(&mut buffer)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .context("failed to encode PNG")?;
    Ok(buffer)
}

fn build_highlight_view(actual: &RgbaImage, baseline: &RgbaImage) -> RgbaImage {
    let (width, height) = actual.dimensions();
    let mut tinted = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let a = actual.get_pixel(x, y).0;
            let b = baseline.get_pixel(x, y).0;
            let diff_r = (a[0] as f32 - b[0] as f32).abs() / 255.0;
            let diff_g = (a[1] as f32 - b[1] as f32).abs() / 255.0;
            let diff_b = (a[2] as f32 - b[2] as f32).abs() / 255.0;
            let diff = diff_r.max(diff_g.max(diff_b));

            if diff > 0.0 {
                let weight = diff.min(1.0);
                let red = (a[0] as f32 * (1.0 - weight) + 255.0 * weight) as u8;
                let green = (a[1] as f32 * (1.0 - weight * 0.9)) as u8;
                let blue = (a[2] as f32 * (1.0 - weight * 0.9)) as u8;
                tinted.put_pixel(x, y, Rgba([red, green, blue, 255]));
            } else {
                tinted.put_pixel(x, y, Rgba([a[0], a[1], a[2], 255]));
            }
        }
    }

    tinted
}

fn blit_image(target: &mut RgbaImage, source: &RgbaImage, offset_x: u32) {
    for (x, y, pixel) in source.enumerate_pixels() {
        target.put_pixel(offset_x + x, y, *pixel);
    }
}

fn fill_separator(image: &mut RgbaImage, start_x: u32, width: u32, height: u32) {
    for dx in 0..width {
        for y in 0..height {
            image.put_pixel(start_x + dx, y, Rgba([54, 54, 54, 255]));
        }
    }
}

pub async fn preload_baselines(
    baseline_dir: &camino::Utf8Path,
    cases: &[crate::screenshotter::models::TestCase],
    browser: crate::screenshotter::args::BrowserKind,
) -> Result<std::collections::HashMap<String, BaselineEntry>> {
    use anyhow::Context;
    use futures::{StreamExt, stream::FuturesUnordered};
    use tokio::task::spawn_blocking;

    type BaselineTaskResult = Result<(String, Option<BaselineEntry>)>;

    let mut tasks = FuturesUnordered::new();
    for case in cases {
        let key = case.key.clone();
        let path = baseline_dir.join(format!("{}{}", case.key, browser.screenshot_suffix()));
        tasks.push(spawn_blocking(move || -> BaselineTaskResult {
            let baseline_bytes = match std::fs::read(path.as_std_path()) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    return Ok((key, None));
                }
                Err(err) => return Err(err.into()),
            };

            let image = image::load_from_memory(&baseline_bytes)
                .context("failed to decode baseline PNG")?
                .to_rgba8();

            Ok((
                key,
                Some(BaselineEntry {
                    image: Arc::new(image),
                }),
            ))
        }));
    }

    let mut baselines = std::collections::HashMap::new();
    while let Some(result) = tasks.next().await {
        let (key, maybe_entry) = result.map_err(anyhow::Error::from)??;
        if let Some(entry) = maybe_entry {
            baselines.insert(key, entry);
        }
    }

    Ok(baselines)
}
