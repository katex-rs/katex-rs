//! Font metrics module for KaTeX
//!
//! This module contains metrics regarding fonts and individual symbols. The
//! sigma and xi variables, as well as the metric map contain data extracted
//! from TeX, TeX font metrics, and the TTF files. These data are then exposed
//! via the `FontMetrics` struct and the character metrics functions.
//!
//! In TeX, there are actually three sets of dimensions, one for each of
//! textstyle (size index 5 and higher: >=9pt), scriptstyle (size index 3 and 4:
//! 7-8pt), and scriptscriptstyle (size index 1 and 2: 5-6pt).
//!
//! The font metrics are stored in fonts cmsy10, cmsy7, and cmsy5 respectively.

include!(concat!(env!("OUT_DIR"), "/sigmas_and_xis_generated.rs"));

use crate::font_metrics_data::CharacterMetrics;
use crate::namespace::KeyMap;
use crate::types::Mode;
use crate::unicode::supported_codepoint;
use crate::{KatexContext, ParseError};
use phf::phf_map;

/// Font size index for different styles
/// 0 = textstyle (>=9pt), 1 = scriptstyle (7-8pt), 2 = scriptscriptstyle
/// (5-6pt)
pub type FontSizeIndex = usize;

/// Character metrics structure (re-exported from font_metrics_data for
/// consistency)
pub use crate::font_metrics_data::CharacterMetrics as CharacterMetric;

/// Type alias for metric maps (font family -> character code -> metrics array)
pub type MetricMap = KeyMap<u32, CharacterMetric>;

/// Mapping for characters that don't have direct font metrics
///
/// These are rough approximations, defaulting to Times New Roman which
/// should have Latin-1 and Cyrillic characters, but may not depending on the
/// operating system. The metrics do not account for extra height from the
/// accents. In the case of Cyrillic characters which have both ascenders and
/// descenders we prefer approximations with ascenders, primarily to prevent
/// the fraction bar or root line from intersecting the glyph.
pub const EXTRA_CHARACTER_MAP: phf::Map<char, char> = phf_map! {
    // Latin-1
    '\u{c5}' => 'A',
    '\u{d0}' => 'D',
    '\u{de}' => 'o',
    '\u{e5}' => 'a',
    '\u{f0}' => 'd',
    '\u{fe}' => 'o',

    // Cyrillic
    '\u{410}' => 'A',
    '\u{411}' => 'B',
    '\u{412}' => 'B',
    '\u{413}' => 'F',
    '\u{414}' => 'A',
    '\u{415}' => 'E',
    '\u{416}' => 'K',
    '\u{417}' => '3',
    '\u{418}' => 'N',
    '\u{419}' => 'N',
    '\u{41a}' => 'K',
    '\u{41b}' => 'N',
    '\u{41c}' => 'M',
    '\u{41d}' => 'H',
    '\u{41e}' => 'O',
    '\u{41f}' => 'N',
    '\u{420}' => 'P',
    '\u{421}' => 'C',
    '\u{422}' => 'T',
    '\u{423}' => 'y',
    '\u{424}' => 'O',
    '\u{425}' => 'X',
    '\u{426}' => 'U',
    '\u{427}' => 'h',
    '\u{428}' => 'W',
    '\u{429}' => 'W',
    '\u{42a}' => 'B',
    '\u{42b}' => 'X',
    '\u{42c}' => 'B',
    '\u{42d}' => '3',
    '\u{42e}' => 'X',
    '\u{42f}' => 'R',
    '\u{430}' => 'a',
    '\u{431}' => 'b',
    '\u{432}' => 'a',
    '\u{433}' => 'r',
    '\u{434}' => 'y',
    '\u{435}' => 'e',
    '\u{436}' => 'm',
    '\u{437}' => 'e',
    '\u{438}' => 'n',
    '\u{439}' => 'n',
    '\u{43a}' => 'n',
    '\u{43b}' => 'n',
    '\u{43c}' => 'm',
    '\u{43d}' => 'n',
    '\u{43e}' => 'o',
    '\u{43f}' => 'n',
    '\u{440}' => 'p',
    '\u{441}' => 'c',
    '\u{442}' => 'o',
    '\u{443}' => 'y',
    '\u{444}' => 'b',
    '\u{445}' => 'x',
    '\u{446}' => 'n',
    '\u{447}' => 'n',
    '\u{448}' => 'w',
    '\u{449}' => 'w',
    '\u{44a}' => 'a',
    '\u{44b}' => 'm',
    '\u{44c}' => 'a',
    '\u{44d}' => 'e',
    '\u{44e}' => 'm',
    '\u{44f}' => 'r',
};

/// This function is a convenience function for looking up information in the
/// metric map table. It takes a character as a string, and a font.
///
/// Note: the `width` property may be undefined if fontMetricsData.js wasn't
/// built using `Make extended_metrics`.
pub fn get_character_metrics<'a>(
    ctx: &'a KatexContext,
    character: char,
    font: &str,
    mode: Mode,
) -> Result<Option<&'a CharacterMetrics>, ParseError> {
    let mut ch = character as u32;

    // Try to get metrics directly
    if let Some(metrics) = ctx.font_metrics.get_metric(font, ch)? {
        return Ok(Some(metrics));
    }

    // Try extra character mapping
    if let Some(&replacement_char) = EXTRA_CHARACTER_MAP.get(&character) {
        ch = replacement_char as u32;
        if let Some(metrics) = ctx.font_metrics.get_metric(font, ch)? {
            return Ok(Some(metrics));
        }
    }

    // For text mode, use fallback for supported Asian scripts
    if mode == Mode::Text && supported_codepoint(ch) {
        // Use metrics for 'M' (charcode 77) as fallback
        if let Some(metrics) = ctx.font_metrics.get_metric(font, 77)? {
            return Ok(Some(metrics));
        }
    }

    Ok(None)
}
