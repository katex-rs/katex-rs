//! Unicode symbol mapping and generation utilities
//!
//! This module provides functionality for generating combined Unicode symbols
//! by applying combining diacritical marks to base characters and normalizing
//! them using Unicode NFC normalization.

include!(concat!(env!("OUT_DIR"), "/unicode_symbols_phf.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace::KeySet;

    #[test]
    fn test_generate_unicode_symbols() {
        let symbols = &UNICODE_SYMBOLS;

        // Should generate some combined symbols
        assert!(!symbols.is_empty());

        // Verify that common accented characters are present
        assert!(symbols.contains_key(&'\u{e1}')); // a with acute
        assert!(symbols.contains_key(&'\u{eb}')); // e with diaeresis  
        assert!(symbols.contains_key(&'\u{fb}')); // u with circumflex

        // Verify the component mapping is correct
        if let Some(components) = symbols.get(&'\u{e1}') {
            assert!(components.contains('a'));
            assert!(components.contains('\u{0301}'));
        }

        if let Some(components) = symbols.get(&'\u{eb}') {
            assert!(components.contains('e'));
            assert!(components.contains('\u{0308}'));
        }
    }

    #[test]
    fn test_no_duplicate_symbols() {
        let symbols = &UNICODE_SYMBOLS;
        let keys: KeySet<char> = symbols.keys().copied().collect();

        // Ensure no duplicate keys (each normalized char should be unique)
        assert_eq!(symbols.len(), keys.len());
    }
}
