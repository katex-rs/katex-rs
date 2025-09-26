//! Font metric data storage
//! This file contains font metric data and measurements for KaTeX
//! Generated from the original JavaScript fontMetricsData.js using phf macros

use crate::{ParseError, font_metrics::MetricMap, namespace::KeyMap, types::ParseErrorKind};

/// Font metrics for a single character
/// The array contains: [depth, height, italic, skew, width]
#[derive(Debug, Clone, Copy)]
pub struct CharacterMetrics {
    /// Depth of the character
    pub depth: f64,
    /// Height of the character
    pub height: f64,
    /// Italic correction
    pub italic: f64,
    /// Skew of the character
    pub skew: f64,
    /// Width of the character
    pub width: f64,
}

impl CharacterMetrics {
    /// Create a new FontMetrics instance
    #[must_use]
    pub const fn new(depth: f64, height: f64, italic: f64, skew: f64, width: f64) -> Self {
        Self {
            depth,
            height,
            italic,
            skew,
            width,
        }
    }
}

// Include the generated phf maps from the build script
include!(concat!(env!("OUT_DIR"), "/font_metrics_data_phf.rs"));

/// Main font metrics data structure
#[derive(Default)]
pub struct FontMetricsData {
    /// Custom font metrics added at runtime
    pub custom: KeyMap<String, MetricMap>,
}

impl FontMetricsData {
    /// Get metrics for a specific character in a font family
    pub fn get_metric(
        &self,
        font_family: &str,
        char_code: u32,
    ) -> Result<Option<&CharacterMetrics>, ParseError> {
        if let Some(metrics) = FONT_METRICS_INDEX.get(font_family) {
            return Ok(metrics.get(&char_code));
        }
        if let Some(custom_metrics) = self.custom.get(font_family) {
            return Ok(custom_metrics.get(&char_code));
        }
        Err(ParseError::new(ParseErrorKind::FontMetricsNotFound {
            font_family: font_family.to_owned(),
        }))
    }

    /// Create a new FontMetricsData instance with optional custom metrics
    pub fn add_custom_metrics(
        &mut self,
        font_family: String,
        char_code: u32,
        metrics: CharacterMetrics,
    ) {
        self.custom
            .entry(font_family)
            .or_default()
            .insert(char_code, metrics);
    }
}
