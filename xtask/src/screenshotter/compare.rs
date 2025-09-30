use std::sync::Arc;

use anyhow::{Context, Result};
use image::{ColorType, ImageBuffer, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};

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
            CompareTolerance::Strict => CompareSettings::new(self, 0.002, 0.0035, 0.0070),
            CompareTolerance::Normal => CompareSettings::new(self, 0.004, 0.0085, 0.0180),
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

    let similarity = web_element_ssim(actual_image, baseline_image);

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

#[derive(Copy, Clone, Debug)]
struct SimilarityResult {
    score: f64,
}

fn web_element_ssim(actual: &RgbaImage, baseline: &RgbaImage) -> SimilarityResult {
    let patch = 8u32;
    let width = actual.width();
    let height = actual.height();
    let mut total_weight = 0.0f64;
    let mut weighted_score = 0.0f64;

    let mut y = 0u32;
    while y < height {
        let patch_h = patch.min(height - y);
        let mut x = 0u32;
        while x < width {
            let patch_w = patch.min(width - x);
            let stats = compute_patch_stats(actual, baseline, x, y, patch_w, patch_h);

            if stats.count == 0 {
                x += patch;
                continue;
            }

            let mean_x = stats.sum_x / stats.count as f64;
            let mean_y = stats.sum_y / stats.count as f64;
            let var_x = (stats.sum_xx / stats.count as f64 - mean_x * mean_x).max(0.0);
            let var_y = (stats.sum_yy / stats.count as f64 - mean_y * mean_y).max(0.0);
            let cov_xy = (stats.sum_xy / stats.count as f64 - mean_x * mean_y).clamp(-1.0, 1.0);

            // Tuned for screen content where crisp edges dominate perception.
            let c1 = 0.01f64.powi(2);
            let c2 = 0.03f64.powi(2);

            let numerator_luma = (2.0 * mean_x * mean_y) + c1;
            let numerator_structure = (2.0 * cov_xy) + c2;
            let denominator_luma = (mean_x * mean_x + mean_y * mean_y) + c1;
            let denominator_structure = (var_x + var_y) + c2;

            let mut ssim = (numerator_luma * numerator_structure)
                / (denominator_luma * denominator_structure + f64::EPSILON);
            if !ssim.is_finite() {
                ssim = 0.0;
            }

            let gradient_boost =
                1.0 + (stats.gradient / (stats.count as f64 + f64::EPSILON)).min(1.5) * 0.5;
            let weight = stats.count as f64 * gradient_boost;
            total_weight += weight;
            weighted_score += weight * ssim.clamp(0.0, 1.0);

            x += patch;
        }
        y += patch;
    }

    if total_weight == 0.0 {
        SimilarityResult { score: 1.0 }
    } else {
        SimilarityResult {
            score: (weighted_score / total_weight).clamp(0.0, 1.0),
        }
    }
}

struct PatchStats {
    sum_x: f64,
    sum_y: f64,
    sum_xx: f64,
    sum_yy: f64,
    sum_xy: f64,
    gradient: f64,
    count: usize,
}

fn compute_patch_stats(
    actual: &RgbaImage,
    baseline: &RgbaImage,
    origin_x: u32,
    origin_y: u32,
    patch_w: u32,
    patch_h: u32,
) -> PatchStats {
    let mut actual_buf = Vec::with_capacity((patch_w * patch_h) as usize);
    let mut baseline_buf = Vec::with_capacity((patch_w * patch_h) as usize);

    let width = actual.width() as usize;
    let actual_raw = actual.as_raw();
    let baseline_raw = baseline.as_raw();

    for row in 0..patch_h {
        let y = origin_y + row;
        let row_start = (y as usize * width + origin_x as usize) * 4;
        let row_len = patch_w as usize * 4;
        let actual_row = &actual_raw[row_start..row_start + row_len];
        let baseline_row = &baseline_raw[row_start..row_start + row_len];

        for pixel in 0..patch_w as usize {
            let idx = pixel * 4;
            actual_buf.push(luma_from_rgba(&actual_row[idx..idx + 4]));
            baseline_buf.push(luma_from_rgba(&baseline_row[idx..idx + 4]));
        }
    }

    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    let mut sum_xx = 0.0f64;
    let mut sum_yy = 0.0f64;
    let mut sum_xy = 0.0f64;

    for (&ax, &bx) in actual_buf.iter().zip(baseline_buf.iter()) {
        let ax = ax as f64;
        let bx = bx as f64;
        sum_x += ax;
        sum_y += bx;
        sum_xx += ax * ax;
        sum_yy += bx * bx;
        sum_xy += ax * bx;
    }

    let patch_w = patch_w as usize;
    let patch_h = patch_h as usize;
    let mut gradient = 0.0f64;

    for y in 0..patch_h {
        for x in 0..patch_w {
            let idx = y * patch_w + x;
            let actual_val = actual_buf[idx];
            let baseline_val = baseline_buf[idx];

            if x + 1 < patch_w {
                let right_idx = idx + 1;
                gradient += (actual_val - actual_buf[right_idx]).abs() as f64;
                gradient += (baseline_val - baseline_buf[right_idx]).abs() as f64;
            }

            if y + 1 < patch_h {
                let down_idx = idx + patch_w;
                gradient += (actual_val - actual_buf[down_idx]).abs() as f64;
                gradient += (baseline_val - baseline_buf[down_idx]).abs() as f64;
            }
        }
    }

    PatchStats {
        sum_x,
        sum_y,
        sum_xx,
        sum_yy,
        sum_xy,
        gradient,
        count: actual_buf.len(),
    }
}

#[inline]
fn luma_from_rgba(px: &[u8]) -> f32 {
    debug_assert!(px.len() == 4);
    let alpha = px[3] as f32 / 255.0;
    if alpha == 0.0 {
        return 0.0;
    }

    let r = srgb_to_linear(px[0] as f32 / 255.0);
    let g = srgb_to_linear(px[1] as f32 / 255.0);
    let b = srgb_to_linear(px[2] as f32 / 255.0);

    alpha * (0.2126 * r + 0.7152 * g + 0.0722 * b)
}

#[inline]
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
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
    let mut tinted = vec![0u8; (width * height * 4) as usize];
    let actual_raw = actual.as_raw();
    let baseline_raw = baseline.as_raw();

    for (dst, (a, b)) in tinted
        .chunks_exact_mut(4)
        .zip(actual_raw.chunks_exact(4).zip(baseline_raw.chunks_exact(4)))
    {
        let diff_r = ((a[0] as f32 - b[0] as f32).abs()) / 255.0;
        let diff_g = ((a[1] as f32 - b[1] as f32).abs()) / 255.0;
        let diff_b = ((a[2] as f32 - b[2] as f32).abs()) / 255.0;
        let weight = diff_r.max(diff_g).max(diff_b).clamp(0.0, 1.0);

        if weight > 0.0 {
            let tinted_r = (a[0] as f32 * (1.0 - weight) + 255.0 * weight).clamp(0.0, 255.0);
            let tinted_g = (a[1] as f32 * (1.0 - weight * 0.9)).clamp(0.0, 255.0);
            let tinted_b = (a[2] as f32 * (1.0 - weight * 0.9)).clamp(0.0, 255.0);
            dst.copy_from_slice(&[tinted_r as u8, tinted_g as u8, tinted_b as u8, 255]);
        } else {
            dst.copy_from_slice(&[a[0], a[1], a[2], 255]);
        }
    }

    RgbaImage::from_raw(width, height, tinted).expect("image dimensions should match")
}

fn blit_image(target: &mut RgbaImage, source: &RgbaImage, offset_x: u32) {
    let bytes_per_pixel = 4usize;
    let target_width = target.width() as usize;
    let source_width = source.width() as usize;
    let offset_bytes = offset_x as usize * bytes_per_pixel;
    let target_stride = target_width * bytes_per_pixel;
    let source_stride = source_width * bytes_per_pixel;

    let mut target_samples = target.as_flat_samples_mut();
    let target_raw = target_samples.as_mut_slice();
    let source_raw = source.as_raw();

    for row in 0..source.height() as usize {
        let src_start = row * source_stride;
        let dst_start = row * target_stride + offset_bytes;
        target_raw[dst_start..dst_start + source_stride]
            .copy_from_slice(&source_raw[src_start..src_start + source_stride]);
    }
}

fn fill_separator(image: &mut RgbaImage, start_x: u32, width: u32, height: u32) {
    let bytes_per_pixel = 4usize;
    let image_width = image.width() as usize;
    let stride = image_width * bytes_per_pixel;
    let fill_width = width as usize * bytes_per_pixel;
    let offset_bytes = start_x as usize * bytes_per_pixel;
    let mut target_samples = image.as_flat_samples_mut();
    let target_raw = target_samples.as_mut_slice();
    let fill_pixel = [54u8, 54, 54, 255];

    for row in 0..height as usize {
        let row_start = row * stride + offset_bytes;
        let row_slice = &mut target_raw[row_start..row_start + fill_width];
        for chunk in row_slice.chunks_exact_mut(bytes_per_pixel) {
            chunk.copy_from_slice(&fill_pixel);
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
