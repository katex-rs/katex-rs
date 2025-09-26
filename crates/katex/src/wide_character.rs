//! Wide character (Mathematical Alphanumeric Symbols) support
//!
//! This module provides support for Unicode range U+1D400 to U+1D7FF
//! (Mathematical Alphanumeric Symbols). It exposes a function that,
//! given a wide character and rendering mode, returns the font metrics
//! name and the CSS class needed to render the character properly.

use crate::types::{Mode, ParseError, ParseErrorKind};

/// Mapping rows for Latin letters. Each entry is a triple of
/// (math CSS class, text CSS class, font metrics name).
///
/// Data derived from KaTeX/src/wide-character.js and
/// https://www.unicode.org/charts/PDF/U1D400.pdf
const WIDE_LATIN_LETTER_DATA: [(&str, &str, &str); 26] = [
    ("mathbf", "textbf", "Main-Bold"),               // A–Z bold upright
    ("mathbf", "textbf", "Main-Bold"),               // a–z bold upright
    ("mathnormal", "textit", "Math-Italic"),         // A–Z italic
    ("mathnormal", "textit", "Math-Italic"),         // a–z italic
    ("boldsymbol", "boldsymbol", "Main-BoldItalic"), // A–Z bold italic
    ("boldsymbol", "boldsymbol", "Main-BoldItalic"), // a–z bold italic
    // Map fancy A–Z letters to script, not calligraphic.
    ("mathscr", "textscr", "Script-Regular"), // A–Z script
    ("", "", ""),                             // a–z script. No font
    ("", "", ""),                             // A–Z bold script. No font
    ("", "", ""),                             // a–z bold script. No font
    ("mathfrak", "textfrak", "Fraktur-Regular"), // A–Z Fraktur
    ("mathfrak", "textfrak", "Fraktur-Regular"), // a–z Fraktur
    ("mathbb", "textbb", "AMS-Regular"),      // A–Z double-struck
    ("mathbb", "textbb", "AMS-Regular"),      // k double-struck
    // Note: using a bold font visually, but metrics for regular Fraktur.
    ("mathboldfrak", "textboldfrak", "Fraktur-Regular"), // A–Z bold Fraktur
    ("mathboldfrak", "textboldfrak", "Fraktur-Regular"), // a–z bold Fraktur
    ("mathsf", "textsf", "SansSerif-Regular"),           // A–Z sans-serif
    ("mathsf", "textsf", "SansSerif-Regular"),           // a–z sans-serif
    ("mathboldsf", "textboldsf", "SansSerif-Bold"),      // A–Z bold sans-serif
    ("mathboldsf", "textboldsf", "SansSerif-Bold"),      // a–z bold sans-serif
    ("mathitsf", "textitsf", "SansSerif-Italic"),        // A–Z italic sans-serif
    ("mathitsf", "textitsf", "SansSerif-Italic"),        // a–z italic sans-serif
    ("", "", ""),                                        // A–Z bold italic sans-serif. No font
    ("", "", ""),                                        // a–z bold italic sans-serif. No font
    ("mathtt", "texttt", "Typewriter-Regular"),          // A–Z monospace
    ("mathtt", "texttt", "Typewriter-Regular"),          // a–z monospace
];

/// Mapping rows for numerals. Each entry is a triple of
/// (math CSS class, text CSS class, font metrics name).
const WIDE_NUMERAL_DATA: [(&str, &str, &str); 5] = [
    ("mathbf", "textbf", "Main-Bold"),              // 0–9 bold
    ("", "", ""),                                   // 0–9 double-struck. No KaTeX font
    ("mathsf", "textsf", "SansSerif-Regular"),      // 0–9 sans-serif
    ("mathboldsf", "textboldsf", "SansSerif-Bold"), // 0–9 bold sans-serif
    ("mathtt", "texttt", "Typewriter-Regular"),     // 0–9 monospace
];

/// Given a wide character (U+1D400–U+1D7FF) and mode, return
/// (font metrics name, CSS class) describing how to render it.
///
/// For unsupported Greek letters in this block (U+1D6A7–U+1D7CD),
/// returns ("", ""). For code points outside the block, returns a ParseError.
pub fn wide_character_font_from_char(
    wide_char: char,
    mode: Mode,
) -> Result<(&'static str, &'static str), ParseError> {
    let code_point = wide_char as u32;
    let css_col = i32::from(mode != Mode::Math);

    if (0x1D400..0x1D6A4).contains(&code_point) {
        // Latin letters, groups of 26 per row
        let i = ((code_point - 0x1D400) / 26) as usize;
        let (math_cls, text_cls, font_name) = WIDE_LATIN_LETTER_DATA[i];
        let css_class = if css_col == 0 { math_cls } else { text_cls };
        Ok((font_name, css_class))
    } else if (0x1D7CE..=0x1D7FF).contains(&code_point) {
        // Numerals, groups of 10 per row
        let i = ((code_point - 0x1D7CE) / 10) as usize;
        let (math_cls, text_cls, font_name) = WIDE_NUMERAL_DATA[i];
        let css_class = if css_col == 0 { math_cls } else { text_cls };
        Ok((font_name, css_class))
    } else if code_point == 0x1D6A5 || code_point == 0x1D6A6 {
        // Dotless i or j
        let (math_cls, text_cls, font_name) = WIDE_LATIN_LETTER_DATA[0];
        let css_class = if css_col == 0 { math_cls } else { text_cls };
        Ok((font_name, css_class))
    } else if (0x1D6A7..0x1D7CE).contains(&code_point) {
        // Greek letters. Not supported yet.
        Ok(("", ""))
    } else {
        Err(ParseError::new(ParseErrorKind::UnsupportedWideCharacter {
            character: wide_char.to_string(),
        }))
    }
}

/// Convenience wrapper that accepts a string and uses its first scalar value.
pub fn get_wide_character_font(
    wide_char: &str,
    mode: Mode,
) -> Result<(&'static str, &'static str), ParseError> {
    wide_char.chars().next().map_or_else(
        || Err(ParseError::new(ParseErrorKind::EmptyWideCharacterInput)),
        |ch| wide_character_font_from_char(ch, mode),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_capital_a() {
        // U+1D400 MATHEMATICAL BOLD CAPITAL A
        let ch = char::from_u32(0x1D400).unwrap();
        let (font, class) = wide_character_font_from_char(ch, Mode::Math).unwrap();
        assert_eq!(font, "Main-Bold");
        assert_eq!(class, "mathbf");
    }

    #[test]
    fn test_bold_digit_zero() {
        // U+1D7CE MATHEMATICAL BOLD DIGIT ZERO
        let ch = char::from_u32(0x1D7CE).unwrap();
        let (font, class) = wide_character_font_from_char(ch, Mode::Math).unwrap();
        assert_eq!(font, "Main-Bold");
        assert_eq!(class, "mathbf");
    }

    #[test]
    fn test_dotless_i_j() {
        for cp in [0x1D6A5u32, 0x1D6A6u32] {
            let ch = char::from_u32(cp).unwrap();
            let (font, class) = wide_character_font_from_char(ch, Mode::Text).unwrap();
            assert_eq!(font, "Main-Bold");
            assert_eq!(class, "textbf");
        }
    }

    #[test]
    fn test_greek_block_unsupported_returns_empty() {
        // Pick a codepoint inside 0x1D6A7..0x1D7CD
        let ch = char::from_u32(0x1D6B5).unwrap();
        let (font, class) = wide_character_font_from_char(ch, Mode::Math).unwrap();
        assert_eq!(font, "");
        assert_eq!(class, "");
    }

    #[test]
    fn test_outside_block_errors() {
        let res = wide_character_font_from_char('A', Mode::Math);
        assert!(res.is_err());
    }
}
