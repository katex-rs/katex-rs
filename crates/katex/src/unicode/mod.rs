//! Modules for handling Unicode characters encoding and conversion to LaTeX
//! commands.

/// Unicode accent characters and their LaTeX equivalents
/// This module provides mappings from Unicode combining diacritical marks
/// to their corresponding LaTeX commands in text and math modes.
pub mod unicode_accents;
pub mod unicode_scripts;
pub mod unicode_sup_or_sub;
pub mod unicode_symbols;

pub use unicode_accents::{AccentMapping, UNICODE_ACCENTS};
pub use unicode_scripts::Script;
pub use unicode_scripts::all_blocks_len;
pub use unicode_scripts::script_from_codepoint;
pub use unicode_scripts::supported_codepoint;
pub use unicode_sup_or_sub::{get_base_character, is_unicode_subscript, is_unicode_sup_or_sub};
pub use unicode_symbols::UNICODE_SYMBOLS;

/// Get the LaTeX mapping for a Unicode accent character
///
/// Returns `Some(AccentMapping)` if the character is a known Unicode accent,
/// or `None` if the character is not a recognized accent.
#[must_use]
pub fn get_accent_mapping(ch: char) -> Option<&'static AccentMapping> {
    UNICODE_ACCENTS.get(&ch)
}
