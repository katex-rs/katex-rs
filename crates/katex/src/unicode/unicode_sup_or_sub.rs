//! Unicode superscript and subscript character detection and mapping
//!
//! This module provides utilities for detecting and mapping Unicode
//! superscript and subscript characters, used primarily for mathematical
//! notation in KaTeX.

use phf::Set;
use phf::phf_set;

/// Regular expression pattern for matching Unicode subscript characters
/// Equivalent to JavaScript: /^[₊₋₌₍₎₀₁₂₃₄₅₆₇₈₉ₐₑₕᵢⱼₖₗₘₙₒₚᵣₛₜᵤᵥₓᵦᵧᵨᵩᵪ]/
/// Note: In Rust, we'll use character set matching instead of regex for
/// performance
const UNICODE_SUB_CHARS: Set<char> = phf_set!(
    '\u{208a}', '\u{208b}', '\u{208c}', '\u{208d}', '\u{208e}', '\u{2080}', '\u{2081}', '\u{2082}',
    '\u{2083}', '\u{2084}', '\u{2085}', '\u{2086}', '\u{2087}', '\u{2088}', '\u{2089}', '\u{2090}',
    '\u{2091}', '\u{2095}', '\u{1d62}', '\u{2c7c}', '\u{2096}', '\u{2097}', '\u{2098}', '\u{2099}',
    '\u{2092}', '\u{209a}', '\u{1d63}', '\u{209b}', '\u{209c}', '\u{1d64}', '\u{1d65}', '\u{2093}',
    '\u{1d66}', '\u{1d67}', '\u{1d68}', '\u{1d69}', '\u{1d6a}',
);

use phf::phf_map;

/// Mapping table for subscript and superscript characters to their base forms
/// Equivalent to JavaScript uSubsAndSups object
pub static U_SUBS_AND_SUPS: phf::Map<char, &'static str> = phf_map! {
    '\u{208a}' => "+",
    '\u{208b}' => "-",
    '\u{208c}' => "=",
    '\u{208d}' => "(",
    '\u{208e}' => ")",
    '\u{2080}' => "0",
    '\u{2081}' => "1",
    '\u{2082}' => "2",
    '\u{2083}' => "3",
    '\u{2084}' => "4",
    '\u{2085}' => "5",
    '\u{2086}' => "6",
    '\u{2087}' => "7",
    '\u{2088}' => "8",
    '\u{2089}' => "9",
    '\u{2090}' => "a", // ₐ
    '\u{2091}' => "e", // ₑ
    '\u{2095}' => "h", // ₕ
    '\u{1D62}' => "i", // ᵢ
    '\u{2C7C}' => "j", // ⱼ
    '\u{2096}' => "k", // ₖ
    '\u{2097}' => "l", // ₗ
    '\u{2098}' => "m", // ₘ
    '\u{2099}' => "n", // ₙ
    '\u{2092}' => "o", // ₒ
    '\u{209A}' => "p", // ₚ
    '\u{1D63}' => "r", // ᵣ
    '\u{209B}' => "s", // ₛ
    '\u{209C}' => "t", // ₜ
    '\u{1D64}' => "u", // ᵤ
    '\u{1D65}' => "v", // ᵥ
    '\u{2093}' => "x", // ₓ
    '\u{1D66}' => "\u{3b2}", // ᵦ
    '\u{1D67}' => "\u{3b3}", // ᵧ
    '\u{1D68}' => "\u{3c1}", // ᵨ
    '\u{1D69}' => "\u{3c6}", // ᵩ (using φ instead of \u{03d5})
    '\u{1D6A}' => "\u{3c7}", // ᵪ

    // Superscript mappings
    '\u{207a}' => "+",
    '\u{207b}' => "-",
    '\u{207c}' => "=",
    '\u{207d}' => "(",
    '\u{207e}' => ")",
    '\u{2070}' => "0",
    '\u{b9}' => "1",
    '\u{b2}' => "2",
    '\u{b3}' => "3",
    '\u{2074}' => "4",
    '\u{2075}' => "5",
    '\u{2076}' => "6",
    '\u{2077}' => "7",
    '\u{2078}' => "8",
    '\u{2079}' => "9",
    '\u{1D2C}' => "A",
    '\u{1D2E}' => "B",
    '\u{1D30}' => "D",
    '\u{1D31}' => "E",
    '\u{1D33}' => "G",
    '\u{1D34}' => "H",
    '\u{1D35}' => "I",
    '\u{1D36}' => "J",
    '\u{1D37}' => "K",
    '\u{1D38}' => "L",
    '\u{1D39}' => "M",
    '\u{1D3A}' => "N",
    '\u{1D3C}' => "O",
    '\u{1D3E}' => "P",
    '\u{1D3F}' => "R",
    '\u{1D40}' => "T",
    '\u{1D41}' => "U",
    '\u{2C7D}' => "V",
    '\u{1D42}' => "W",
    '\u{1D43}' => "a",
    '\u{1D47}' => "b",
    '\u{1D9C}' => "c",
    '\u{1D48}' => "d",
    '\u{1D49}' => "e",
    '\u{1DA0}' => "f",
    '\u{1D4D}' => "g",
    '\u{02B0}' => "h",
    '\u{2071}' => "i",
    '\u{02B2}' => "j",
    '\u{1D4F}' => "k",
    '\u{02E1}' => "l",
    '\u{1D50}' => "m",
    '\u{207F}' => "n",
    '\u{1D52}' => "o",
    '\u{1D56}' => "p",
    '\u{02B3}' => "r",
    '\u{02E2}' => "s",
    '\u{1D57}' => "t",
    '\u{1D58}' => "u",
    '\u{1D5B}' => "v",
    '\u{02B7}' => "w",
    '\u{02E3}' => "x",
    '\u{02B8}' => "y",
    '\u{1DBB}' => "z",
    '\u{1D5D}' => "\u{3b2}",
    '\u{1D5E}' => "\u{3b3}",
    '\u{1D5F}' => "\u{3b4}",
    '\u{1D60}' => "\u{3c6}", // using φ instead of \u{03d5}
    '\u{1D61}' => "\u{3c7}",
    '\u{1DBF}' => "\u{3b8}",
};

/// Check if a character is a Unicode subscript character
/// Equivalent to JavaScript: unicodeSubRegEx.test(char)
#[must_use]
pub fn is_unicode_subscript(ch: char) -> bool {
    UNICODE_SUB_CHARS.contains(&ch)
}

/// Get the base character for a subscript or superscript character
/// Returns None if the character is not in the mapping table
#[must_use]
pub fn get_base_character(ch: char) -> Option<&'static str> {
    U_SUBS_AND_SUPS.get(&ch).copied()
}

/// Check if a character is a Unicode superscript character
#[must_use]
pub fn is_unicode_superscript(ch: char) -> bool {
    match ch {
        '⁺' | '⁻' | '⁼' | '⁽' | '⁾' | '⁰' | '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' => {
            true
        }
        c if ('\u{1D2C}'..='\u{1D6A}').contains(&c) => true, // Superscript letters range
        c if ('\u{2070}'..='\u{207F}').contains(&c) => true, // Superscript numbers and symbols
        c if ('\u{02B0}'..='\u{02FF}').contains(&c) => true, // Spacing modifier letters
        _ => false,
    }
}

/// Check if a character is either a subscript or superscript character
#[must_use]
pub fn is_unicode_sup_or_sub(ch: char) -> bool {
    is_unicode_subscript(ch) || is_unicode_superscript(ch)
}

/// Convert a subscript or superscript character to its base form
/// Returns the original character if no mapping exists
pub fn to_base_character(ch: char) -> String {
    get_base_character(ch).map_or_else(|| ch.to_string(), ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscript_detection() {
        // Test subscript characters
        assert!(is_unicode_subscript('\u{2080}'));
        assert!(is_unicode_subscript('\u{2081}'));
        assert!(is_unicode_subscript('\u{2082}'));
        assert!(is_unicode_subscript('\u{208a}'));
        assert!(is_unicode_subscript('\u{208b}'));
        assert!(is_unicode_subscript('\u{2090}'));
        assert!(is_unicode_subscript('\u{2091}'));
        assert!(is_unicode_subscript('\u{2093}'));

        // Test non-subscript characters
        assert!(!is_unicode_subscript('0'));
        assert!(!is_unicode_subscript('1'));
        assert!(!is_unicode_subscript('a'));
        assert!(!is_unicode_subscript('+'));
        assert!(!is_unicode_subscript('-'));
    }

    #[test]
    fn test_superscript_detection() {
        // Test superscript characters
        assert!(is_unicode_superscript('\u{2070}'));
        assert!(is_unicode_superscript('\u{b9}'));
        assert!(is_unicode_superscript('\u{b2}'));
        assert!(is_unicode_superscript('\u{b3}'));
        assert!(is_unicode_superscript('\u{207a}'));
        assert!(is_unicode_superscript('\u{207b}'));
        assert!(is_unicode_superscript('\u{207c}'));

        // Test non-superscript characters
        assert!(!is_unicode_superscript('0'));
        assert!(!is_unicode_superscript('1'));
        assert!(!is_unicode_superscript('2'));
        assert!(!is_unicode_superscript('+'));
        assert!(!is_unicode_superscript('-'));
    }

    #[test]
    fn test_base_character_mapping() {
        // Test subscript mappings
        assert_eq!(get_base_character('\u{2080}'), Some("0"));
        assert_eq!(get_base_character('\u{2081}'), Some("1"));
        assert_eq!(get_base_character('\u{2082}'), Some("2"));
        assert_eq!(get_base_character('\u{208a}'), Some("+"));
        assert_eq!(get_base_character('\u{208b}'), Some("-"));
        assert_eq!(get_base_character('\u{2090}'), Some("a"));
        assert_eq!(get_base_character('\u{2091}'), Some("e"));

        // Test superscript mappings
        assert_eq!(get_base_character('\u{2070}'), Some("0"));
        assert_eq!(get_base_character('\u{b9}'), Some("1"));
        assert_eq!(get_base_character('\u{b2}'), Some("2"));
        assert_eq!(get_base_character('\u{207a}'), Some("+"));
        assert_eq!(get_base_character('\u{207b}'), Some("-"));

        // Test non-mapped characters
        assert_eq!(get_base_character('A'), None);
        assert_eq!(get_base_character('0'), None);
        assert_eq!(get_base_character('+'), None);
    }

    #[test]
    fn test_to_base_character() {
        // Test subscript conversion
        assert_eq!(to_base_character('\u{2080}'), "0");
        assert_eq!(to_base_character('\u{2081}'), "1");
        assert_eq!(to_base_character('\u{208a}'), "+");

        // Test superscript conversion
        assert_eq!(to_base_character('\u{2070}'), "0");
        assert_eq!(to_base_character('\u{b9}'), "1");
        assert_eq!(to_base_character('\u{207a}'), "+");

        // Test non-mapped characters (should return original)
        assert_eq!(to_base_character('A'), "A");
        assert_eq!(to_base_character('0'), "0");
        assert_eq!(to_base_character('+'), "+");
    }

    #[test]
    fn test_combined_sup_sub_detection() {
        // Test both subscript and superscript detection
        assert!(is_unicode_sup_or_sub('\u{2080}'));
        assert!(is_unicode_sup_or_sub('\u{2070}'));
        assert!(is_unicode_sup_or_sub('\u{2081}'));
        assert!(is_unicode_sup_or_sub('\u{b9}'));
        assert!(is_unicode_sup_or_sub('\u{208a}'));
        assert!(is_unicode_sup_or_sub('\u{207a}'));

        // Test non-sup/sub characters
        assert!(!is_unicode_sup_or_sub('0'));
        assert!(!is_unicode_sup_or_sub('1'));
        assert!(!is_unicode_sup_or_sub('a'));
        assert!(!is_unicode_sup_or_sub('A'));
        assert!(!is_unicode_sup_or_sub('+'));
        assert!(!is_unicode_sup_or_sub('-'));
    }

    #[test]
    fn test_unicode_codepoints() {
        // Test specific Unicode codepoints from the original JavaScript
        assert!(is_unicode_subscript('\u{2090}')); // ₐ
        assert!(is_unicode_subscript('\u{2091}')); // ₑ
        assert!(is_unicode_subscript('\u{2095}')); // ₕ
        assert!(is_unicode_subscript('\u{1D62}')); // ᵢ
        assert!(is_unicode_subscript('\u{2C7C}')); // ⱼ

        assert!(is_unicode_superscript('\u{1D2C}')); // ᴬ
        assert!(is_unicode_superscript('\u{1D2E}')); // ᴮ
        assert!(is_unicode_superscript('\u{1D30}')); // ᴰ
        assert!(is_unicode_superscript('\u{1D31}')); // ᴱ
        assert!(is_unicode_superscript('\u{1D33}')); // ᴳ
    }
}
