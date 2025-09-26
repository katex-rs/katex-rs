//! Spacing data constants for KaTeX Rust implementation
//!
//! This module provides precomputed spacing data for mathematical typesetting
//! and layout. It contains spacing relationships between different classes of
//! atoms.

use phf::{Map, phf_map};

/// Measurement structure representing a size with number and unit in
/// mathematical typesetting.
///
/// This struct is used to represent dimensions in KaTeX's layout system,
/// equivalent to the JavaScript Measurement type from units.js. It supports
/// various units commonly used in TeX and LaTeX typesetting, particularly for
/// spacing and sizing in mathematical expressions.
///
/// # Units
/// - `"mu"`: Math unit, 1/18 of an em in the current math style
/// - `"em"`: Relative to the current font size
/// - `"ex"`: Relative to the height of the letter 'x'
/// - `"pt"`: Points (absolute unit)
/// - `"px"`: Pixels (for screen rendering)
///
/// # See Also
/// - [LaTeX spacing commands](https://www.overleaf.com/learn/latex/Spacing_in_math_mode)
/// - [`THINSPACE`], [`MEDIUMSPACE`], [`THICKSPACE`] for standard spacing
///   constants
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement<T>
where
    T: AsRef<str>,
{
    /// Numeric value of the measurement
    pub number: f64,
    /// Unit of measurement (e.g., "mu", "em", "ex")
    pub unit: T,
}

/// Type alias for measurements with owned string units.
///
/// This is useful when creating measurements dynamically or when the unit
/// string needs to be computed at runtime. The owned string allows for
/// flexibility in unit specification but has higher memory overhead compared to
/// static strings.
pub type MeasurementOwned = Measurement<String>;

/// Type alias for measurements with static string slice units.
///
/// This is the preferred type for compile-time constant measurements, as it
/// avoids heap allocation and provides better performance. Most spacing
/// constants in this module use this type.
pub type MeasurementStatic = Measurement<&'static str>;

/// Thin space measurement (3 mu) - corresponds to \, in LaTeX.
///
/// In mathematical typesetting, thin space is used for subtle separation
/// between elements. In LaTeX, this is equivalent to the `\,` command, which
/// inserts a space of 3/18 em (or 1/6 em).
///
/// # LaTeX Equivalent
/// ```latex
/// a\,b  % thin space between a and b
/// ```
///
/// # Usage
/// Used in spacing tables for relationships between ordinary atoms and
/// operators, punctuation, and inner atoms.
pub const THINSPACE: MeasurementStatic = MeasurementStatic {
    number: 3.0,
    unit: "mu",
};

/// Medium space measurement (4 mu) - corresponds to \: in LaTeX.
///
/// Medium space provides moderate separation in mathematical expressions. In
/// LaTeX, this is equivalent to the `\:` command, which inserts a space of 4/18
/// em.
///
/// # LaTeX Equivalent
/// ```latex
/// a\:b  % medium space between a and b
/// ```
///
/// # Usage
/// Used for spacing around binary operators and between certain atom classes
/// in display and text styles.
pub const MEDIUMSPACE: MeasurementStatic = MeasurementStatic {
    number: 4.0,
    unit: "mu",
};

/// Thick space measurement (5 mu) - corresponds to \; in LaTeX.
///
/// Thick space provides significant separation in mathematical expressions. In
/// LaTeX, this is equivalent to the `\;` command, which inserts a space of 5/18
/// em.
///
/// # LaTeX Equivalent
/// ```latex
/// a\;b  % thick space between a and b
/// ```
///
/// # Usage
/// Used for spacing around relation symbols and between certain atom classes
/// where more pronounced separation is needed.
pub const THICKSPACE: MeasurementStatic = MeasurementStatic {
    number: 5.0,
    unit: "mu",
};

/// Atom class types for spacing relationships in mathematical typesetting.
///
/// In TeX and LaTeX, mathematical atoms are classified into different types
/// that determine how much space should be inserted between them. This enum
/// represents the standard atom classes used in KaTeX for spacing calculations.
///
/// The spacing between atoms depends on their classes according to predefined
/// rules in [`SPACINGS`] and [`TIGHT_SPACINGS`]. These rules follow traditional
/// TeX spacing conventions to ensure proper visual appearance of mathematical
/// expressions.
///
/// # Atom Classes
/// - `Mord`: Ordinary symbols (letters, numbers, ordinary operators like +)
/// - `Mop`: Large operators (∑, ∏, ∫, etc.)
/// - `Mbin`: Binary operators (+, -, ×, etc.)
/// - `Mrel`: Relations (=, <, >, ⊂, etc.)
/// - `Mopen`: Opening delimiters ((, [, {, etc.)
/// - `Mclose`: Closing delimiters ), ], }, etc.)
/// - `Mpunct`: Punctuation (,, ;, !, etc.)
/// - `Minner`: Inner expressions (fractions, etc.)
///
/// # See Also
/// - [`SPACINGS`] for display/text style spacing rules
/// - [`TIGHT_SPACINGS`] for script style spacing rules
/// - [TeXbook Chapter 18](https://www.ctan.org/pkg/texbook) for detailed
///   spacing rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomClass {
    /// Ordinary atom (letters, numbers, ordinary operators)
    Mord,
    /// Operator atom (large operators like ∑, ∏, ∫)
    Mop,
    /// Binary operator atom (+, -, ×, ÷, etc.)
    Mbin,
    /// Relation atom (=, <, >, ⊂, ∈, etc.)
    Mrel,
    /// Opening delimiter atom ((, [, {, etc.)
    Mopen,
    /// Closing delimiter atom ), ], }, etc.)
    Mclose,
    /// Punctuation atom (,, ;, !, etc.)
    Mpunct,
    /// Inner atom (fractions, nested expressions)
    Minner,
}

/// Type alias for spacing relationships between atom classes.
///
/// This type represents a nested map structure that defines the spacing rules
/// between different pairs of atom classes. The outer map keys are the left
/// atom classes, and the inner map keys are the right atom classes. The values
/// are the [`MeasurementStatic`] spacing to insert between them.
///
/// # Structure
/// ```text
/// Spacings = Map<LeftAtomClass, Map<RightAtomClass, Spacing>>
/// ```
///
/// # See Also
/// - [`SPACINGS`] for display/text style spacing table
/// - [`TIGHT_SPACINGS`] for script style spacing table
/// - [`AtomClass`] for the atom classification system
pub type Spacings = Map<&'static str, Map<&'static str, MeasurementStatic>>;

/// Spacing relationships for display and text styles in mathematical
/// typesetting.
///
/// This constant defines the standard spacing rules between different atom
/// classes in display and text styles (normal-sized math). The spacing values
/// follow traditional TeX conventions and are measured in math units ("mu").
///
/// In LaTeX, these spacings correspond to the automatic spacing inserted
/// between mathematical elements based on their semantic classes. For example:
/// - No space between ordinary symbols: `ab`
/// - Thin space around operators: `a + b` (but actually handled by atom
///   classes)
/// - Medium space around binary operators: `a = b + c`
/// - Thick space around relations: `a < b`
///
/// # Spacing Rules Summary
/// | Left \ Right | Mord | Mop  | Mbin | Mrel | Mopen | Mclose | Mpunct | Minner |
/// |--------------|------|------|------|------|-------|--------|--------|--------|
/// | Mord         | -    | thin | med  | thick| -     | -      | thin   | thin   |
/// | Mop          | thin | thin | -    | thick| -     | -      | -      | thin   |
/// | Mbin         | med  | med  | -    | -    | med   | -      | -      | med    |
/// | Mrel         | thick| thick| -    | -    | thick | -      | -      | thick  |
/// | Mopen        | -    |  -   | -    | -    | -     | -      | -      | -      |
/// | Mclose       | -    | thin | med  | thick| -     | -      | -      | thin   |
/// | Mpunct       | thin | thin | -    | thick| thin  | thin   | thin   | thin   |
/// | Minner       | thin | thin | med  | thick| thin  | -      | thin   | thin   |
///
/// # Examples
/// ```
/// use katex::spacing_data::{MEDIUMSPACE, SPACINGS, THICKSPACE, THINSPACE};
///
/// // Spacing between ordinary atom and operator
/// let mord_mop = SPACINGS.get("mord").unwrap().get("mop");
/// assert_eq!(mord_mop, Some(&THINSPACE));
///
/// // Spacing between binary operator and ordinary atom
/// let mbin_mord = SPACINGS.get("mbin").unwrap().get("mord");
/// assert_eq!(mbin_mord, Some(&MEDIUMSPACE));
/// ```
///
/// # See Also
/// - [`TIGHT_SPACINGS`] for script and scriptscript styles
/// - [`AtomClass`] for atom classification details
/// - [TeX spacing rules](https://www.tug.org/TUGboat/tb30-1/tb94vieth.pdf)
pub const SPACINGS: Spacings = phf_map! {
    "mord" => phf_map! {
        "mop" => THINSPACE,
        "mbin" => MEDIUMSPACE,
        "mrel" => THICKSPACE,
        "minner" => THINSPACE,
    },
    "mop" => phf_map! {
        "mord" => THINSPACE,
        "mop" => THINSPACE,
        "mrel" => THICKSPACE,
        "minner" => THINSPACE,
    },
    "mbin" => phf_map! {
        "mord" => MEDIUMSPACE,
        "mop" => MEDIUMSPACE,
        "mopen" => MEDIUMSPACE,
        "minner" => MEDIUMSPACE,
    },
    "mrel" => phf_map! {
        "mord" => THICKSPACE,
        "mop" => THICKSPACE,
        "mopen" => THICKSPACE,
        "minner" => THICKSPACE,
    },
    "mopen" => phf_map!{},
    "mclose" => phf_map! {
        "mop" => THINSPACE,
        "mbin" => MEDIUMSPACE,
        "mrel" => THICKSPACE,
        "minner" => THINSPACE,
    },
    "mpunct" => phf_map! {
        "mord" => THINSPACE,
        "mop" => THINSPACE,
        "mrel" => THICKSPACE,
        "mopen" => THINSPACE,
        "mclose" => THINSPACE,
        "mpunct" => THINSPACE,
        "minner" => THINSPACE,
    },
    "minner" => phf_map! {
        "mord" => THINSPACE,
        "mop" => THINSPACE,
        "mbin" => MEDIUMSPACE,
        "mrel" => THICKSPACE,
        "mopen" => THINSPACE,
        "mpunct" => THINSPACE,
        "minner" => THINSPACE,
    },
};

/// Spacing relationships for script and scriptscript styles in mathematical
/// typesetting.
///
/// This constant defines the reduced spacing rules used in superscript,
/// subscript, and nested script contexts. In these smaller styles, spacing is
/// generally tighter to maintain readability while conserving space.
///
/// In LaTeX, script styles (script and scriptscript) use condensed spacing to
/// prevent expressions from becoming too spread out when rendered in smaller
/// sizes. Most spacing relationships are omitted, leaving only essential
/// separations.
///
/// # Key Differences from Display Style
/// - Binary operators and relations get no spacing in script styles
/// - Only thin spaces around operators and some delimiters remain
/// - Most other combinations have zero spacing
///
/// # See Also
/// - [`SPACINGS`] for display and text style spacing
/// - [`AtomClass`] for atom classification
/// - LaTeX's `\scriptstyle` and `\scriptscriptstyle` commands
pub const TIGHT_SPACINGS: Spacings = phf_map! {
    "mord" => phf_map! {
        "mop" => THINSPACE,
    },
    "mop" => phf_map! {
        "mord" => THINSPACE,
        "mop" => THINSPACE,
    },
    "mbin" => phf_map!{},
    "mrel" => phf_map!{},
    "mopen" => phf_map!{},
    "mclose" => phf_map! {
        "mop" => THINSPACE,
    },
    "mpunct" => phf_map!{},
    "minner" => phf_map! {
        "mop" => THINSPACE,
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_constants() {
        assert_eq!(THINSPACE.number, 3.0);
        assert_eq!(THINSPACE.unit, "mu");

        assert_eq!(MEDIUMSPACE.number, 4.0);
        assert_eq!(MEDIUMSPACE.unit, "mu");

        assert_eq!(THICKSPACE.number, 5.0);
        assert_eq!(THICKSPACE.unit, "mu");
    }

    #[test]
    fn test_spacings_structure() {
        // Test that all atom classes are present in spacings
        let atom_classes = vec![
            "mord", "mop", "mbin", "mrel", "mopen", "mclose", "mpunct", "minner",
        ];

        for atom_class in atom_classes {
            assert!(
                SPACINGS.contains_key(atom_class),
                "Missing atom class: {atom_class}"
            );
        }

        // Test specific spacing relationships
        let mord_spacings = SPACINGS.get("mord").unwrap();
        assert_eq!(mord_spacings.get("mop"), Some(&THINSPACE));
        assert_eq!(mord_spacings.get("mbin"), Some(&MEDIUMSPACE));
        assert_eq!(mord_spacings.get("mrel"), Some(&THICKSPACE));
    }

    #[test]
    fn test_tight_spacings_structure() {
        // Test that tight spacings has fewer relationships
        let mord_tight = TIGHT_SPACINGS.get("mord").unwrap();
        assert_eq!(mord_tight.len(), 1); // Only mop relationship
        assert_eq!(mord_tight.get("mop"), Some(&THINSPACE));

        // mbin should be empty in tight spacings
        let mbin_tight = TIGHT_SPACINGS.get("mbin").unwrap();
        assert_eq!(mbin_tight.len(), 0);
    }
}
