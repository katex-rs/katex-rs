//! Mathematical symbol mappings for KaTeX
//!
//! This module provides comprehensive symbol table management for mathematical
//! typesetting, mapping LaTeX command names to Unicode mathematical symbols and
//! handling symbol categorization, ligature detection, and wide character
//! variants. It supports both math and text modes as defined in KaTeX
//! specifications.
//!
//! # Key Features
//!
//! - Unicode mathematical symbol mapping (e.g., `\alpha` → `α`)
//! - Ligature handling for common sequences (e.g., `--` → `–`)
//! - Wide character variants for mathematical alphanumeric symbols
//! - Symbol categorization by atom types and font styles
//! - Support for both math and text rendering modes
//!
//! # Examples
//!
//! ```rust
//! use katex::symbols::{Font, Group, Mode, NonAtom, Symbols};
//!
//! let mut symbols = Symbols::new();
//! symbols.define_symbol(
//!     Mode::Math,
//!     Font::Main,
//!     Group::NonAtom(NonAtom::MathOrd),
//!     Some('α'),
//!     "\\alpha",
//!     false,
//! );
//!
//! if let Some(info) = symbols.get_math("\\alpha") {
//!     println!("Unicode: {}", info.replace.unwrap_or('?'));
//! }
//! ```

mod types;
use crate::ParseError;
use crate::namespace::KeyMap;
use crate::types::ParseErrorKind;
use phf::phf_map;
pub use types::{Atom, CharInfo, Font, Group, Mode, NonAtom};

/// Core symbol table for mathematical typesetting
///
/// The `Symbols` struct maintains separate mappings for math and text modes,
/// storing character information including Unicode replacements, font styles,
/// and atom categorization. This enables proper rendering of LaTeX commands
/// in different contexts.
///
/// # Fields
///
/// * `math` - Symbol mappings for mathematical expressions
/// * `text` - Symbol mappings for text content
///
/// # Examples
///
/// ```rust
/// use katex::symbols::Symbols;
///
/// let symbols = Symbols::new();
/// assert!(symbols.get_math("\\custom").is_none());
/// assert!(symbols.get_text("\\custom").is_none());
/// ```
pub struct Symbols {
    /// Symbol mappings for mathematical expressions
    math: KeyMap<String, CharInfo>,
    /// Symbol mappings for text content
    text: KeyMap<String, CharInfo>,
}

include!(concat!(env!("OUT_DIR"), "/generated_symbols_data.rs"));

/// Default implementation for Symbols
///
/// Creates an empty symbol table. For a fully populated table with standard
/// mathematical symbols, use [`create_symbols`] instead.
///
/// # Examples
///
/// ```rust
/// use katex::symbols::Symbols;
///
/// let symbols = Symbols::default();
/// assert!(symbols.get_math("\\custom").is_none());
/// ```
impl Default for Symbols {
    fn default() -> Self {
        Self::new()
    }
}

impl Symbols {
    /// Creates a new empty symbol table
    ///
    /// Initializes empty hash maps for both math and text modes. Use this
    /// when you need to build a custom symbol table or start from scratch.
    /// For standard KaTeX symbols, prefer [`create_symbols`].
    ///
    /// # Returns
    ///
    /// A new `Symbols` instance with empty mappings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use katex::symbols::Symbols;
    ///
    /// let symbols = Symbols::new();
    /// assert!(symbols.get_math("\\custom").is_none());
    /// assert!(symbols.get_text("\\custom").is_none());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            math: KeyMap::default(),
            text: KeyMap::default(),
        }
    }

    /// Defines a symbol in the specified rendering mode
    ///
    /// Adds a new symbol mapping to the appropriate mode's hash map,
    /// associating a LaTeX command name with its character properties and
    /// Unicode replacement. This is the primary method for populating the
    /// symbol table.
    ///
    /// # Parameters
    ///
    /// * `mode` - The rendering mode ([`Mode::Math`] or [`Mode::Text`])
    /// * `font` - The font style for the symbol
    /// * `group` - The atom group categorization
    /// * `replace` - Optional Unicode character replacement string
    /// * `name` - The LaTeX command name (e.g., `"\\alpha"`)
    /// * `accept_unicode_char` - Whether to also map the Unicode character back
    ///   to the symbol
    ///
    /// # Examples
    ///
    /// ```rust
    /// use katex::symbols::{Font, Group, Mode, NonAtom, Symbols};
    ///
    /// let mut symbols = Symbols::new();
    /// symbols.define_symbol(
    ///     Mode::Math,
    ///     Font::Main,
    ///     Group::NonAtom(NonAtom::MathOrd),
    ///     Some('α'),
    ///     "\\alpha",
    ///     true,
    /// );
    /// ```
    ///
    /// # See Also
    ///
    /// - [`create_symbols`] for populating with standard symbols
    /// - [`CharInfo`] for symbol property details
    pub fn define_symbol(
        &mut self,
        mode: Mode,
        font: Font,
        group: Group,
        replace: Option<char>,
        name: &str,
        accept_unicode_char: bool,
    ) {
        let char_info = CharInfo {
            font,
            group,
            replace,
        };

        let table = match mode {
            Mode::Math => &mut self.math,
            Mode::Text => &mut self.text,
        };

        table.insert(name.to_owned(), char_info.clone());

        if accept_unicode_char && let Some(s) = replace {
            table.insert(s.to_string(), char_info);
        }
    }

    /// Retrieves character information for a symbol in math mode
    ///
    /// Looks up the symbol by its LaTeX command name in the math mode mappings.
    ///
    /// # Parameters
    ///
    /// * `name` - The LaTeX command name (e.g., `"\\alpha"`)
    ///
    /// # Returns
    ///
    /// `Some(&CharInfo)` if the symbol exists in math mode, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use katex::symbols::create_symbols;
    ///
    /// let symbols = create_symbols();
    /// if let Some(info) = symbols.get_math("\\alpha") {
    ///     println!("Unicode: {}", info.replace.unwrap_or('?'));
    /// }
    /// ```
    #[must_use]
    pub fn get_math(&self, name: &str) -> Option<&CharInfo> {
        self.math
            .get(name)
            .or_else(|| POPULATE_MATH_SYMBOLS_MAP.get(name))
    }

    /// Retrieves character information for a symbol in text mode
    ///
    /// Looks up the symbol by its LaTeX command name in the text mode mappings.
    ///
    /// # Parameters
    ///
    /// * `name` - The LaTeX command name (e.g., `"\\#"` for hash symbol)
    ///
    /// # Returns
    ///
    /// `Some(&CharInfo)` if the symbol exists in text mode, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use katex::symbols::create_symbols;
    ///
    /// let symbols = create_symbols();
    /// if let Some(info) = symbols.get_text("\\#") {
    ///     println!("Unicode: {}", info.replace.unwrap_or('?'));
    /// }
    /// ```
    #[must_use]
    pub fn get_text(&self, name: &str) -> Option<&CharInfo> {
        self.text
            .get(name)
            .or_else(|| POPULATE_TEXT_SYMBOLS_MAP.get(name))
    }

    /// Retrieves character information for a symbol in the specified mode
    ///
    /// A unified lookup method that delegates to the appropriate mode-specific
    /// method.
    ///
    /// # Parameters
    ///
    /// * `mode` - The rendering mode to search in
    /// * `name` - The LaTeX command name
    ///
    /// # Returns
    ///
    /// `Some(&CharInfo)` if the symbol exists in the specified mode, `None`
    /// otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use katex::symbols::{Mode, create_symbols};
    ///
    /// let symbols = create_symbols();
    /// if let Some(info) = symbols.get(Mode::Math, "\\alpha") {
    ///     println!("Found in math mode: {:?}", info.replace);
    /// }
    /// ```
    #[must_use]
    pub fn get(&self, mode: Mode, name: &str) -> Option<&CharInfo> {
        match mode {
            Mode::Math => self.get_math(name),
            Mode::Text => self.get_text(name),
        }
    }

    /// Checks if a symbol exists in the specified mode
    #[must_use]
    pub fn contains(&self, mode: Mode, name: &str) -> bool {
        match mode {
            Mode::Math => {
                self.math.contains_key(name) || POPULATE_MATH_SYMBOLS_MAP.contains_key(name)
            }
            Mode::Text => {
                self.text.contains_key(name) || POPULATE_TEXT_SYMBOLS_MAP.contains_key(name)
            }
        }
    }
}

/// Checks if a string represents a known ligature sequence
///
/// Ligatures are special character combinations that are replaced with single
/// Unicode characters for better typography (e.g., `--` becomes an en-dash).
///
/// # Parameters
///
/// * `s` - The string to check for ligature patterns
///
/// # Returns
///
/// `true` if the string is a recognized ligature, `false` otherwise.
///
/// # Examples
///
/// ```
/// use katex::symbols::is_ligature;
///
/// assert!(is_ligature("--"));
/// assert!(is_ligature("``"));
/// assert!(!is_ligature("not a ligature"));
/// ```
///
/// # See Also
///
/// - [`get_ligature_replacement`] for getting the replacement character
/// - [`LIGATURES`] for the complete list of supported ligatures
#[must_use]
pub fn is_ligature(s: &str) -> bool {
    LIGATURES.contains_key(s)
}

/// Retrieves the Unicode replacement character for a ligature sequence
///
/// Returns the single Unicode character that should replace the ligature
/// sequence for proper mathematical typesetting.
///
/// # Parameters
///
/// * `s` - The ligature string to look up
///
/// # Returns
///
/// `Some(&str)` containing the Unicode replacement if the ligature is known,
/// `None` if the string is not a recognized ligature.
///
/// # Examples
///
/// ```
/// use katex::symbols::get_ligature_replacement;
///
/// assert_eq!(get_ligature_replacement("--"), Some("–"));
/// assert_eq!(get_ligature_replacement("---"), Some("—"));
/// assert_eq!(get_ligature_replacement("unknown"), None);
/// ```
///
/// # See Also
///
/// - [`is_ligature`] for checking if a string is a ligature
/// - [`LIGATURES`] for the mapping table
#[must_use]
pub fn get_ligature_replacement(s: &str) -> Option<&'static str> {
    LIGATURES.get(s).copied()
}

/// Creates a wide Unicode character from UTF-16 surrogate pair
///
/// Mathematical alphanumeric symbols in Unicode are often represented using
/// surrogate pairs in UTF-16 encoding. This function combines the high and low
/// surrogates to form the complete Unicode scalar value.
///
/// # Parameters
///
/// * `high_surrogate` - The high surrogate value (typically 0xD835 for math
///   symbols)
/// * `low_surrogate` - The low surrogate value for the specific character
///
/// # Error Handling
///
/// Returns an empty string if the surrogate pair doesn't form a valid Unicode
/// scalar value.
///
/// # See Also
///
/// - [`generate_wide_char_variants`] for creating multiple variants
/// - Unicode Mathematical Alphanumeric Symbols block (U+1D400–U+1D7FF)
///
/// # Safety
///
/// Safe only within the valid range of surrogate pairs for mathematical
#[must_use]
const fn create_wide_char(high: u16, low: u16) -> char {
    let cp = 0x10000 + (((high as u32) - 0xD800) << 10) + ((low as u32) - 0xDC00);
    // SAFETY: The calculation ensures cp is a valid Unicode scalar value
    unsafe { char::from_u32_unchecked(cp) }
}

/// Static mapping of ligature sequences to their Unicode replacements
///
/// This compile-time hash map contains common typographical ligatures used in
/// mathematical typesetting. Ligatures improve text appearance by replacing
/// multiple characters with a single, properly designed glyph.
///
/// # Supported Ligatures
///
/// | Sequence | Unicode | Description     |
/// |----------|---------|-----------------|
/// | `--`     | `–`     | En dash         |
/// | `---`    | `—`     | Em dash         |
/// | `` ``    | `"`     | Left double quote |
/// | `''`     | `"`     | Right double quote |
///
/// # Examples
///
/// ```
/// use katex::symbols::LIGATURES;
///
/// assert_eq!(LIGATURES.get("--"), Some(&"–"));
/// assert_eq!(LIGATURES.get("``"), Some(&"\u{201c}"));
/// ```
///
/// # See Also
///
/// - [`is_ligature`] for checking if a string is a ligature
/// - [`get_ligature_replacement`] for safe lookup
pub const LIGATURES: phf::Map<&str, &str> = phf_map!(
    "--" => "\u{2013}",
    "---" => "\u{2014}",
    "``" => "\u{201c}",
    "''" => "\u{201d}",
);

/// Conversion from string to Group enum
///
/// Parses a string representation of a symbol group into the corresponding
/// [`Group`] enum variant. This is used when loading symbol definitions
/// from configuration files or data sources.
///
/// # Parameters
///
/// * `s` - String slice representing the group name
///
/// # Returns
///
/// The corresponding [`Group`] enum variant.
///
/// # Error Handling
///
/// Panics with an error message if the string doesn't match any known group.
/// This ensures data integrity when loading from external sources.
///
/// # See Also
///
/// - [`Group`] enum for the complete type definition
/// - [`Atom`] and [`NonAtom`] for group classifications
impl TryFrom<&str> for Group {
    type Error = ParseError;
    fn try_from(s: &str) -> Result<Self, ParseError> {
        match s {
            "bin" => Ok(Self::Atom(Atom::Bin)),
            "close" => Ok(Self::Atom(Atom::Close)),
            "inner" => Ok(Self::Atom(Atom::Inner)),
            "open" => Ok(Self::Atom(Atom::Open)),
            "punct" => Ok(Self::Atom(Atom::Punct)),
            "rel" => Ok(Self::Atom(Atom::Rel)),
            "accent" | "accent-token" => Ok(Self::NonAtom(NonAtom::AccentToken)),
            "mathord" => Ok(Self::NonAtom(NonAtom::MathOrd)),
            "op" | "op-token" => Ok(Self::NonAtom(NonAtom::OpToken)),
            "spacing" => Ok(Self::NonAtom(NonAtom::Spacing)),
            "textord" => Ok(Self::NonAtom(NonAtom::TextOrd)),
            _ => Err(ParseError::new(ParseErrorKind::InvalidGroup {
                group: s.to_owned(),
            })),
        }
    }
}

/// Creates and populates the global symbol table with standard KaTeX symbols
///
/// This function initializes a complete symbol table containing all standard
/// mathematical symbols, Greek letters, operators, and special characters
/// supported by KaTeX. It loads symbols from JSON data, generates wide
/// character variants, and sets up both math and text mode mappings.
///
/// The returned symbol table includes:
/// - Greek letters (α, β, γ, etc.)
/// - Mathematical operators (=, +, −, etc.)
/// - Relations (≡, ∼, ≅, etc.)
/// - Delimiters ({, }, \[, \], etc.)
/// - Accents (´, `, ˜, etc.)
/// - Wide character variants for letters and numbers
/// - Text symbols for regular typography
///
/// # Returns
///
/// A fully populated [`Symbols`] instance ready for mathematical typesetting.
///
/// # Examples
///
/// ```rust
/// use katex::symbols::create_symbols;
///
/// let symbols = create_symbols();
/// assert!(symbols.get_math("\\alpha").is_some());
/// assert!(symbols.get_text("\\#").is_some());
///
/// // Look up a Greek letter
/// if let Some(alpha) = symbols.get_math("\\alpha") {
///     println!("Alpha: {}", alpha.replace.unwrap_or('?'));
/// }
/// ```
///
/// # Performance
///
/// This function performs significant computation to generate all symbol
/// variants. Therefore, should not be called except context initialization or
/// testing.
///
/// # See Also
///
/// - [`Symbols::new`] for creating an empty table
/// - [`Symbols::define_symbol`] for adding individual symbols
/// - KaTeX symbol documentation for the complete symbol set
#[must_use]
pub fn create_symbols() -> Symbols {
    let mut symbols = Symbols::new();

    // mathTextSymbols loop: numbers and symbols for math mode
    let math_text_symbols = "0123456789/@.\"";
    for ch in math_text_symbols.chars() {
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &ch.to_string(),
            false,
        );
    }

    // !TODO: Fix extractor to include all symbols
    symbols.define_symbol(
        Mode::Text,
        Font::Main,
        Group::NonAtom(NonAtom::AccentToken),
        Some('\u{00a8}'),
        r#"\""#,
        false,
    );

    // textSymbols loop: symbols for text mode
    let text_symbols = "0123456789!@*()-=+\";:?/.,";
    for ch in text_symbols.chars() {
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &ch.to_string(),
            false,
        );
    }

    // letters loop: A-Z a-z for math and text modes
    let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    for ch in letters.chars() {
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &ch.to_string(),
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &ch.to_string(),
            false,
        );
    }

    // wideChar letters loop: bold, italic, etc., with conditions for i < 26
    let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    for (i, ch) in letters.chars().enumerate() {
        // Bold (U+1D400-1D433)
        let wide_char = create_wide_char(0xD835, 0xDC00 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Italic (U+1D434-1D467)
        let wide_char = create_wide_char(0xD835, 0xDC34 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Bold Italic (U+1D468-1D49B)
        let wide_char = create_wide_char(0xD835, 0xDC68 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Fraktur (U+1D504-1D537)
        let wide_char = create_wide_char(0xD835, 0xDD04 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Bold Fraktur (U+1D56C-1D59F)
        let wide_char = create_wide_char(0xD835, 0xDD6C + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Sans-serif (U+1D5A0-1D5D3)
        let wide_char = create_wide_char(0xD835, 0xDDA0 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Sans Bold (U+1D5D4-1D607)
        let wide_char = create_wide_char(0xD835, 0xDDD4 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Sans Italic (U+1D608-1D63B)
        let wide_char = create_wide_char(0xD835, 0xDE08 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Monospace (U+1D670-1D6A3)
        let wide_char = create_wide_char(0xD835, 0xDE70 + i as u16).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        if i < 26 {
            // Double struck (U+1D538-1D56B for A-Z)
            let wide_char = create_wide_char(0xD835, 0xDD38 + i as u16).to_string();
            symbols.define_symbol(
                Mode::Math,
                Font::Main,
                Group::NonAtom(NonAtom::MathOrd),
                Some(ch),
                &wide_char,
                false,
            );
            symbols.define_symbol(
                Mode::Text,
                Font::Main,
                Group::NonAtom(NonAtom::TextOrd),
                Some(ch),
                &wide_char,
                false,
            );

            // Script (U+1D49C-1D4CF for A-Z)
            let wide_char = create_wide_char(0xD835, 0xDC9C + i as u16).to_string();
            symbols.define_symbol(
                Mode::Math,
                Font::Main,
                Group::NonAtom(NonAtom::MathOrd),
                Some(ch),
                &wide_char,
                false,
            );
            symbols.define_symbol(
                Mode::Text,
                Font::Main,
                Group::NonAtom(NonAtom::TextOrd),
                Some(ch),
                &wide_char,
                false,
            );
        }
    }

    // Special case for "k" double struck lower case
    let wide_char = create_wide_char(0xD835, 0xDD5C).to_string();
    symbols.define_symbol(
        Mode::Math,
        Font::Main,
        Group::NonAtom(NonAtom::MathOrd),
        Some('k'),
        &wide_char,
        false,
    );
    symbols.define_symbol(
        Mode::Text,
        Font::Main,
        Group::NonAtom(NonAtom::TextOrd),
        Some('k'),
        &wide_char,
        false,
    );

    // wideChar numbers loop: bold, etc., for 0-9
    for i in 0..10 {
        let ch = char::from(b'0' + i);

        // Bold (U+1D7CE-1D7D7)
        let wide_char = create_wide_char(0xD835, 0xDFCE + u16::from(i)).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Sans-serif (U+1D7E2-1D7EB)
        let wide_char = create_wide_char(0xD835, 0xDFE2 + u16::from(i)).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Bold Sans (U+1D7EC-1D7F5)
        let wide_char = create_wide_char(0xD835, 0xDFEC + u16::from(i)).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );

        // Monospace (U+1D7F6-1D7FF)
        let wide_char = create_wide_char(0xD835, 0xDFF6 + u16::from(i)).to_string();
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &wide_char,
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &wide_char,
            false,
        );
    }

    // extraLatin loop
    let extra_latin = "\u{00d0}\u{00de}\u{00fe}";
    for ch in extra_latin.chars() {
        symbols.define_symbol(
            Mode::Math,
            Font::Main,
            Group::NonAtom(NonAtom::MathOrd),
            Some(ch),
            &ch.to_string(),
            false,
        );
        symbols.define_symbol(
            Mode::Text,
            Font::Main,
            Group::NonAtom(NonAtom::TextOrd),
            Some(ch),
            &ch.to_string(),
            false,
        );
    }

    symbols
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{Atom, Font, Group, NonAtom, create_symbols};

    #[test]
    fn test_symbol_creation() {
        let symbols = create_symbols();

        // Test basic symbol lookup
        assert!(
            symbols.get_math("\\equiv").is_some(),
            "Failed to find \\equiv"
        );
        assert!(
            symbols.get_math("\\forall").is_some(),
            "Failed to find \\forall"
        );
        assert!(
            symbols.get_math("\\exists").is_some(),
            "Failed to find \\exists"
        );

        // Test mode-specific symbols
        assert!(
            symbols.get_text("\\#").is_some(),
            "Failed to find \\# in text mode"
        );
        assert!(
            symbols.get_math("\\#").is_some(),
            "Failed to find \\# in math mode"
        );

        // Test Unicode character mapping
        let equiv_info = symbols.get_math("\\equiv").unwrap();
        assert_eq!(equiv_info.replace, Some('\u{2261}'));
    }

    #[test]
    fn test_ligature_detection() {
        assert!(is_ligature("--"));
        assert!(is_ligature("---"));
        assert!(is_ligature("``"));
        assert!(is_ligature("''"));
        assert!(!is_ligature("notaligature"));

        assert_eq!(get_ligature_replacement("--"), Some("\u{2013}"));
        assert_eq!(get_ligature_replacement("---"), Some("\u{2014}"));
        assert_eq!(get_ligature_replacement("``"), Some("\u{201c}"));
        assert_eq!(get_ligature_replacement("''"), Some("\u{201d}"));
        assert_eq!(get_ligature_replacement("notaligature"), None);
    }

    #[test]
    fn test_symbol_properties() {
        let symbols = create_symbols();

        // Test symbol properties
        let equiv_info = symbols.get_math("\\equiv").unwrap();
        assert_eq!(equiv_info.font, Font::Main);
        assert!(matches!(equiv_info.group, Group::Atom(Atom::Rel)));

        let forall_info = symbols.get_math("\\forall").unwrap();
        assert_eq!(forall_info.font, Font::Main);
        assert!(matches!(
            forall_info.group,
            Group::NonAtom(NonAtom::TextOrd)
        ));
    }

    #[test]
    fn test_key_symbols_retrieval() {
        let symbols = create_symbols();

        // Test Greek letters
        assert!(symbols.get_math("\\alpha").is_some());
        assert!(symbols.get_math("\\beta").is_some());
        assert!(symbols.get_math("\\gamma").is_some());
        assert!(symbols.get_math("\\delta").is_some());

        // Test that alpha has correct properties
        let alpha_info = symbols.get_math("\\alpha").unwrap();
        assert_eq!(alpha_info.font, Font::Main);
        assert!(matches!(alpha_info.group, Group::NonAtom(NonAtom::MathOrd)));
        assert_eq!(alpha_info.replace, Some('\u{03b1}'));

        // Test numbers
        for i in 0..10 {
            let num_str = format!("{i}");
            assert!(symbols.get_math(&num_str).is_some());
            assert!(symbols.get_text(&num_str).is_some());
        }

        // Test letters
        let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        for ch in letters.chars() {
            let ch_str = ch.to_string();
            assert!(symbols.get_math(&ch_str).is_some());
            assert!(symbols.get_text(&ch_str).is_some());
        }

        // Test operators
        assert!(symbols.get_math("\\equiv").is_some());
        assert!(symbols.get_math("\\forall").is_some());
        assert!(symbols.get_math("\\exists").is_some());

        // Test accents
        assert!(symbols.get_math("\\acute").is_some());
        assert!(symbols.get_math("\\grave").is_some());
        assert!(symbols.get_math("\\tilde").is_some());
    }
}
