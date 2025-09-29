//! Style definitions and calculations for KaTeX Rust implementation
//!
//! This module contains information and classes for the various kinds of styles
//! used in TeX. It provides a generic `Style` struct, which holds information
//! about a specific style. It then provides instances of all the different
//! kinds of styles possible, and provides functions to move between them and
//! get information about them.

/// The main style class. Contains a unique id for the style, a size (which is
/// the same for cramped and uncramped version of a style), and a cramped flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    /// Unique identifier for the style
    pub id: usize,
    /// Size level (0=display, 1=text, 2=script, 3=scriptscript)
    pub size: usize,
    /// Whether the style is cramped
    pub cramped: bool,
}

impl Style {
    /// Create a new style instance (const function for use in constants)
    #[must_use]
    pub const fn new(id: usize, size: usize, cramped: bool) -> Self {
        Self { id, size, cramped }
    }

    /// Get the style of a superscript given a base in the current style.
    #[must_use]
    pub const fn sup(&self) -> &'static Self {
        &STYLES[SUP[self.id]]
    }

    /// Get the style of a subscript given a base in the current style.
    #[must_use]
    pub const fn sub(&self) -> &'static Self {
        &STYLES[SUB[self.id]]
    }

    /// Get the style of a fraction numerator given the fraction in the current
    /// style.
    #[must_use]
    pub const fn frac_num(&self) -> &'static Self {
        &STYLES[FRAC_NUM[self.id]]
    }

    /// Get the style of a fraction denominator given the fraction in the
    /// current style.
    #[must_use]
    pub const fn frac_den(&self) -> &'static Self {
        &STYLES[FRAC_DEN[self.id]]
    }

    /// Get the cramped version of a style (in particular, cramping a cramped
    /// style doesn't change the style).
    #[must_use]
    pub const fn cramp(&self) -> &'static Self {
        &STYLES[CRAMP[self.id]]
    }

    /// Get a text or display version of this style.
    #[must_use]
    pub const fn text(&self) -> &'static Self {
        &STYLES[TEXT_LOOKUP[self.id]]
    }

    /// Return true if this style is tightly spaced
    /// (scriptstyle/scriptscriptstyle)
    #[inline]
    #[must_use]
    pub const fn is_tight(&self) -> bool {
        self.size >= 2
    }
}

// IDs of the different styles
const D: usize = 0;
const DC: usize = 1;
const T: usize = 2;
const TC: usize = 3;
const S: usize = 4;
const SC: usize = 5;
const SS: usize = 6;
const SSC: usize = 7;

// Instances of the different styles
const STYLES: [Style; 8] = [
    Style::new(D, 0, false),  // DISPLAY
    Style::new(DC, 0, true),  // DISPLAY_CRAMPED
    Style::new(T, 1, false),  // TEXT
    Style::new(TC, 1, true),  // TEXT_CRAMPED
    Style::new(S, 2, false),  // SCRIPT
    Style::new(SC, 2, true),  // SCRIPT_CRAMPED
    Style::new(SS, 3, false), // SCRIPTSCRIPT
    Style::new(SSC, 3, true), // SCRIPTSCRIPT_CRAMPED
];

// Lookup tables for switching from one style to another
const SUP: [usize; 8] = [S, SC, S, SC, SS, SSC, SS, SSC];
const SUB: [usize; 8] = [SC, SC, SC, SC, SSC, SSC, SSC, SSC];
const FRAC_NUM: [usize; 8] = [T, TC, S, SC, SS, SSC, SS, SSC];
const FRAC_DEN: [usize; 8] = [TC, TC, SC, SC, SSC, SSC, SSC, SSC];
const CRAMP: [usize; 8] = [DC, DC, TC, TC, SC, SC, SSC, SSC];
const TEXT_LOOKUP: [usize; 8] = [D, DC, T, TC, T, TC, T, TC];

// Public style exports

/// The display style, used for top-level mathematical expressions in display
/// mode.
///
/// This is the largest font size in KaTeX's style hierarchy, equivalent to
/// LaTeX's `\displaystyle`. Font size is typically 1.21 times the text size,
/// with full-sized operators and symbols. Used for equations displayed on their
/// own line, providing maximum readability and proper spacing.
///
/// In mathematical typesetting, display style ensures that fractions, sums,
/// integrals, and other constructs are rendered at their full size, making
/// complex expressions more legible.
///
/// # Style Hierarchy
/// - Size level: 0 (largest)
/// - Cramped: false
/// - Font scaling: 1.0 (baseline)
///
/// # See Also
/// - [`TEXT`]: For inline mathematical expressions
/// - [`SCRIPT`]: For superscript/subscript in display style
/// - [`SCRIPTSCRIPT`]: For nested scripts
pub const DISPLAY: &Style = &STYLES[D];

/// The text style, used for inline mathematical expressions within text.
///
/// This style matches the surrounding text font size, equivalent to LaTeX's
/// `\textstyle`. Font size is the same as the surrounding text, with
/// appropriately sized operators. Used for mathematical expressions embedded
/// within paragraphs or sentences.
///
/// In mathematical typesetting, text style ensures that inline math blends
/// seamlessly with the surrounding text while maintaining proper mathematical
/// notation.
///
/// # Style Hierarchy
/// - Size level: 1
/// - Cramped: false
/// - Font scaling: ~0.83 (smaller than display)
///
///
/// # See Also
/// - [`DISPLAY`]: For standalone equations
/// - [`SCRIPT`]: For superscript/subscript in text style
/// - [`SCRIPTSCRIPT`]: For nested scripts
pub const TEXT: &Style = &STYLES[T];

/// The script style, used for superscript and subscript expressions.
///
/// This is the first level of size reduction in KaTeX's style hierarchy,
/// equivalent to LaTeX's `\scriptstyle`. Font size is reduced to approximately
/// 0.7 times the text size, with smaller operators. Used for superscripts,
/// subscripts, and other secondary mathematical elements.
///
/// In mathematical typesetting, script style provides appropriate size
/// reduction for exponents, indices, and other subordinate elements while
/// maintaining readability.
///
/// # Style Hierarchy
/// - Size level: 2
/// - Cramped: false
/// - Font scaling: ~0.7 (further reduced from text)
///
///
/// # See Also
/// - [`DISPLAY`]: Base style for display math
/// - [`TEXT`]: Base style for inline math
/// - [`SCRIPTSCRIPT`]: For nested scripts within scripts
pub const SCRIPT: &Style = &STYLES[S];

/// The scriptscript style, used for nested superscript and subscript
/// expressions.
///
/// This is the smallest font size in KaTeX's style hierarchy, equivalent to
/// LaTeX's `\scriptscriptstyle`. Font size is further reduced to approximately
/// 0.5 times the text size, with minimal operators. Used for deeply nested
/// superscripts, subscripts, and other tertiary mathematical elements.
///
/// In mathematical typesetting, scriptscript style handles the most deeply
/// nested elements, ensuring that complex expressions remain legible even at
/// small sizes. This style is crucial for rendering multi-level expressions
/// like `x^{y^{z}}`.
///
/// # Style Hierarchy
/// - Size level: 3 (smallest)
/// - Cramped: false
/// - Font scaling: ~0.5 (most reduced)
///
///
/// # See Also
/// - [`DISPLAY`]: Base style for display math
/// - [`TEXT`]: Base style for inline math
/// - [`SCRIPT`]: For first-level scripts
pub const SCRIPTSCRIPT: &Style = &STYLES[SS];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_creation() {
        let style = Style::new(0, 0, false);
        assert_eq!(style.id, 0);
        assert_eq!(style.size, 0);
        assert!(!style.cramped);
    }

    #[test]
    fn test_superscript() {
        assert_eq!(DISPLAY.sup().id, S);
        assert_eq!(TEXT.sup().id, S);
        assert_eq!(SCRIPT.sup().id, SS);
    }

    #[test]
    fn test_subscript() {
        assert_eq!(DISPLAY.sub().id, SC);
        assert_eq!(TEXT.sub().id, SC);
        assert_eq!(SCRIPT.sub().id, SSC);
    }

    #[test]
    fn test_fraction_numerator() {
        assert_eq!(DISPLAY.frac_num().id, T);
        assert_eq!(TEXT.frac_num().id, S);
        assert_eq!(SCRIPT.frac_num().id, SS);
    }

    #[test]
    fn test_fraction_denominator() {
        assert_eq!(DISPLAY.frac_den().id, TC);
        assert_eq!(TEXT.frac_den().id, SC);
        assert_eq!(SCRIPT.frac_den().id, SSC);
    }

    #[test]
    fn test_cramp() {
        assert_eq!(DISPLAY.cramp().id, DC);
        assert_eq!(TEXT.cramp().id, TC);
        assert_eq!(SCRIPT.cramp().id, SC);
    }

    #[test]
    fn test_text() {
        assert_eq!(DISPLAY.text().id, D);
        assert_eq!(TEXT.text().id, T);
        assert_eq!(SCRIPT.text().id, T);
    }

    #[test]
    fn test_is_tight() {
        assert!(!DISPLAY.is_tight());
        assert!(!TEXT.is_tight());
        assert!(SCRIPT.is_tight());
        assert!(SCRIPTSCRIPT.is_tight());
    }
}
