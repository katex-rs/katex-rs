//! Unicode script classification and support detection
//!
//! This module defines the Unicode scripts and script families that KaTeX
//! supports. Scripts defined here can appear in \text{} environments.

/// Unicode script definition with name and character blocks
///
/// Represents a Unicode script or script family supported in KaTeX's \text{}
/// environments. Each script contains multiple character blocks defining the
/// ranges of Unicode codepoints that belong to this script, used for font
/// selection and rendering in mathematical typesetting.
///
/// # Fields
/// * `name` - The human-readable name of the script (e.g., "latin", "cyrillic")
/// * `blocks` - Array of [start, end] codepoint ranges (inclusive) for this
///   script
///
/// # Examples
/// ```
/// use katex::unicode::Script;
///
/// let latin = Script {
///     name: "latin",
///     blocks: &[[0x0100, 0x024F]],
/// };
/// ```
///
/// # See Also
/// - [`SCRIPT_DATA`] for predefined script definitions
/// - [`script_from_codepoint`] for classification
#[derive(Debug, Clone)]
pub struct Script {
    /// Script name
    pub name: &'static str,
    /// Array of character ranges [start, end] (inclusive)
    pub blocks: &'static [[u32; 2]],
}

/// Unicode block data for the families of scripts we support in \text{}.
/// Scripts only need to appear here if they do not have font metrics.
pub const SCRIPT_DATA: &[Script] = &[
    Script {
        // Latin characters beyond the Latin-1 characters we have metrics for.
        // Needed for Czech, Hungarian and Turkish text, for example.
        name: "latin",
        blocks: &[
            [0x0100, 0x024f], // Latin Extended-A and Latin Extended-B
            [0x0300, 0x036f], // Combining Diacritical marks
        ],
    },
    Script {
        // The Cyrillic script used by Russian and related languages.
        // A Cyrillic subset used to be supported as explicitly defined
        // symbols in symbols.js
        name: "cyrillic",
        blocks: &[[0x0400, 0x04ff]],
    },
    Script {
        // Armenian
        name: "armenian",
        blocks: &[[0x0530, 0x058F]],
    },
    Script {
        // The Brahmic scripts of South and Southeast Asia
        // Devanagari (0900–097F)
        // Bengali (0980–09FF)
        // Gurmukhi (0A00–0A7F)
        // Gujarati (0A80–0AFF)
        // Oriya (0B00–0B7F)
        // Tamil (0B80–0BFF)
        // Telugu (0C00–0C7F)
        // Kannada (0C80–0CFF)
        // Malayalam (0D00–0D7F)
        // Sinhala (0D80–0DFF)
        // Thai (0E00–0E7F)
        // Lao (0E80–0EFF)
        // Tibetan (0F00–0FFF)
        // Myanmar (1000–109F)
        name: "brahmic",
        blocks: &[[0x0900, 0x109F]],
    },
    Script {
        name: "georgian",
        blocks: &[[0x10A0, 0x10ff]],
    },
    Script {
        // Chinese and Japanese.
        // The "k" in cjk is for Korean, but we've separated Korean out
        name: "cjk",
        blocks: &[
            [0x3000, 0x30FF], // CJK symbols and punctuation, Hiragana, Katakana
            [0x4E00, 0x9FAF], // CJK ideograms
            [0xFF00, 0xFF60], /* Fullwidth punctuation
                               * TODO: add halfwidth Katakana and Romanji glyphs */
        ],
    },
    Script {
        // Korean
        name: "hangul",
        blocks: &[[0xAC00, 0xD7AF]],
    },
];

/// Calculates the total number of Unicode codepoint boundaries across all
/// supported script blocks.
///
/// This function computes the length of the flattened array that would contain
/// all start and end codepoints from the SCRIPT_DATA blocks, used for building
/// the ALL_BLOCKS constant. The result represents the total number of u32
/// values needed to store all block boundaries as pairs [start, end] for
/// efficient lookup in mathematical typesetting.
///
/// # Returns
/// The total number of u32 values needed to represent all block boundaries
/// (start and end pairs).
///
/// # Examples
/// ```
/// use katex::unicode::all_blocks_len;
///
/// let len = all_blocks_len();
/// // len will be the total number of boundary values across all scripts
/// assert!(len > 0);
/// ```
///
/// # See Also
/// - [`build_all_blocks`] for constructing the array
/// - [`ALL_BLOCKS`] for the precomputed constant
/// - [`SCRIPT_DATA`] for the source script definitions
#[must_use]
pub const fn all_blocks_len() -> usize {
    let mut len = 0;
    let mut i = 0;
    while i < SCRIPT_DATA.len() {
        let mut j = 0;
        while j < SCRIPT_DATA[i].blocks.len() {
            len += 2;
            j += 1;
        }
        i += 1;
    }
    len
}

/// Builds a flattened array of all Unicode codepoint block boundaries from
/// SCRIPT_DATA.
///
/// This function iterates through all script definitions and collects their
/// block ranges into a single array where each pair represents [start, end] of
/// a Unicode block. Used at compile time to generate the ALL_BLOCKS constant
/// for optimal performance in script detection during mathematical typesetting.
///
/// # Returns
/// An array of u32 values representing all block boundaries in the format
/// [start1, end1, start2, end2, ...]
///
/// # Panics
/// This function does not panic as it operates on compile-time constants with
/// known bounds.
///
/// # See Also
/// - [`all_blocks_len`] for the array length
/// - [`ALL_BLOCKS`] for the precomputed constant
/// - [`SCRIPT_DATA`] for the source data
#[must_use]
pub const fn build_all_blocks() -> [u32; all_blocks_len()] {
    let mut arr = [0u32; all_blocks_len()];
    let mut k = 0;
    let mut i = 0;
    while i < SCRIPT_DATA.len() {
        let mut j = 0;
        while j < SCRIPT_DATA[i].blocks.len() {
            arr[k] = SCRIPT_DATA[i].blocks[j][0];
            arr[k + 1] = SCRIPT_DATA[i].blocks[j][1];
            k += 2;
            j += 1;
        }
        i += 1;
    }
    arr
}

/// Precomputed array of all Unicode codepoint block boundaries for supported
/// scripts.
///
/// This constant contains flattened start and end codepoints for all Unicode
/// blocks defined in SCRIPT_DATA, optimized for fast lookup in mathematical
/// typesetting. Each pair [start, end] represents a contiguous range of
/// supported Unicode characters that can appear in KaTeX \text{} environments.
///
/// Used by [`supported_codepoint`] for efficient script detection and font
/// selection. The array is built at compile time for optimal runtime
/// performance.
///
/// # See Also
/// - [`build_all_blocks`] for how this is constructed
/// - [`supported_codepoint`] for usage in script detection
/// - [`SCRIPT_DATA`] for the source script definitions
pub const ALL_BLOCKS: [u32; all_blocks_len()] = build_all_blocks();

/// Given a codepoint, return the name of the script or script family
/// it is from, or None if it is not part of a known block
#[must_use]
pub fn script_from_codepoint(codepoint: u32) -> Option<&'static str> {
    for script in SCRIPT_DATA {
        for block in script.blocks {
            if codepoint >= block[0] && codepoint <= block[1] {
                return Some(script.name);
            }
        }
    }
    None
}

/// Given a codepoint, return true if it falls within one of the
/// scripts or script families defined above and false otherwise.
///
/// This implementation is optimized for performance, similar to the
/// JavaScript version which showed better performance than regex.
///
/// !TODO: Figure out if this is necessary to use in Rust
#[must_use]
pub const fn supported_codepoint(codepoint: u32) -> bool {
    let blocks = ALL_BLOCKS;
    let mut i = 0;
    while i < blocks.len() {
        if codepoint >= blocks[i] && codepoint <= blocks[i + 1] {
            return true;
        }
        i += 2;
    }
    false
}

/// Helper function to get script from a character
#[must_use]
pub fn script_from_char(ch: char) -> Option<&'static str> {
    script_from_codepoint(ch as u32)
}

/// Helper function to check if a character is supported
#[must_use]
pub const fn supported_char(ch: char) -> bool {
    supported_codepoint(ch as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latin_characters() {
        // Latin Extended-A
        assert_eq!(script_from_codepoint(0x0100), Some("latin")); // Ā
        assert_eq!(script_from_codepoint(0x024f), Some("latin")); // ɏ

        // Combining Diacritical marks
        assert_eq!(script_from_codepoint(0x0300), Some("latin")); // ̀ (grave accent)
        assert_eq!(script_from_codepoint(0x036f), Some("latin")); // ͯ 

        assert!(supported_codepoint(0x0100));
        assert!(supported_codepoint(0x0300));
    }

    #[test]
    fn test_cyrillic_characters() {
        assert_eq!(script_from_codepoint(0x0400), Some("cyrillic")); // Ѐ
        assert_eq!(script_from_codepoint(0x04ff), Some("cyrillic")); // ӿ
        assert!(supported_codepoint(0x0400));
    }

    #[test]
    fn test_armenian_characters() {
        assert_eq!(script_from_codepoint(0x0530), Some("armenian")); // Ծ
        assert_eq!(script_from_codepoint(0x058F), Some("armenian")); // ֏
        assert!(supported_codepoint(0x0530));
    }

    #[test]
    fn test_brahmic_characters() {
        assert_eq!(script_from_codepoint(0x0900), Some("brahmic")); // Devanagari
        assert_eq!(script_from_codepoint(0x109F), Some("brahmic")); // Myanmar
        assert!(supported_codepoint(0x0900));
    }

    #[test]
    fn test_georgian_characters() {
        assert_eq!(script_from_codepoint(0x10A0), Some("georgian")); // Ⴀ
        assert_eq!(script_from_codepoint(0x10ff), Some("georgian")); // Ი
        assert!(supported_codepoint(0x10A0));
    }

    #[test]
    fn test_cjk_characters() {
        assert_eq!(script_from_codepoint(0x3000), Some("cjk")); // CJK symbols
        assert_eq!(script_from_codepoint(0x30FF), Some("cjk")); // Katakana
        assert_eq!(script_from_codepoint(0x4E00), Some("cjk")); // CJK ideograms
        assert_eq!(script_from_codepoint(0x9FAF), Some("cjk")); // CJK ideograms
        assert_eq!(script_from_codepoint(0xFF00), Some("cjk")); // Fullwidth punctuation
        assert_eq!(script_from_codepoint(0xFF60), Some("cjk")); // Fullwidth punctuation

        assert!(supported_codepoint(0x3000));
        assert!(supported_codepoint(0x4E00));
        assert!(supported_codepoint(0xFF00));
    }

    #[test]
    fn test_hangul_characters() {
        assert_eq!(script_from_codepoint(0xAC00), Some("hangul")); // 가
        assert_eq!(script_from_codepoint(0xD7AF), Some("hangul")); // 힯
        assert!(supported_codepoint(0xAC00));
    }

    #[test]
    fn test_unsupported_characters() {
        // Basic Latin (not in the extended sets)
        assert_eq!(script_from_codepoint(0x0041), None); // A
        assert!(!supported_codepoint(0x0041));

        // Random Unicode outside supported ranges
        assert_eq!(script_from_codepoint(0x2000), None); // General punctuation
        assert!(!supported_codepoint(0x2000));
    }

    #[test]
    fn test_char_helpers() {
        assert_eq!(script_from_char('\u{100}'), Some("latin")); // 0x0100
        assert!(supported_char('\u{100}'));

        assert_eq!(script_from_char('\u{410}'), Some("cyrillic")); // Cyrillic A
        assert!(supported_char('\u{410}'));

        assert_eq!(script_from_char('A'), None); // Basic Latin
        assert!(!supported_char('A'));
    }

    #[test]
    fn test_boundary_conditions() {
        // Test boundaries of blocks
        assert!(supported_codepoint(0x0100)); // Start of Latin Extended-A
        assert!(supported_codepoint(0x024f)); // End of Latin Extended-B
        assert!(!supported_codepoint(0x00FF)); // Just before Latin Extended-A
        assert!(!supported_codepoint(0x0250)); // Just after Latin Extended-B
    }
}
