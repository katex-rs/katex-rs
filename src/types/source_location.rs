use alloc::sync::Arc;
use core::fmt::{self, Debug};
use core::ptr;

use crate::types::ErrorLocationProvider;

/// Interface for a lexer providing input string
pub trait LexerInterface {
    /// Returns the input string being lexed.
    ///
    /// This method provides access to the original LaTeX/KaTeX input string
    /// that is being tokenized. The returned string is used for source
    /// location tracking and error message generation.
    ///
    /// # Returns
    ///
    /// A reference to the input string.
    fn input(&self) -> &str;
}

/// Provides debug formatting for `LexerInterface` trait objects.
///
/// This implementation allows trait objects implementing `LexerInterface` to be
/// formatted for debugging purposes. Since trait objects don't have concrete
/// types, the debug output is generic and identifies the trait.
impl Debug for dyn LexerInterface + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LexerInterface")
    }
}

/// Provides equality comparison for `LexerInterface` trait objects.
///
/// This implementation compares trait objects by pointer equality, as trait
/// objects don't have a concrete type that can be compared directly. This
/// ensures that two trait object references are considered equal only if they
/// point to the same instance.
impl PartialEq for dyn LexerInterface + '_ {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

/// Represents a source location in a LaTeX/KaTeX mathematical expression.
///
/// This struct tracks the position of tokens or expressions within the input
/// string during parsing, enabling precise error reporting and debugging. It
/// stores a reference-counted copy of the input string and byte offsets for the
/// start and end positions.
///
/// The struct is immutable once created, ensuring thread safety and preventing
/// accidental modifications during parsing.
///
/// # Cross-references
///
/// - Used in `ParseError`(crate::types::ParseError) for error location
///   reporting.
/// - Integrated with [`ErrorLocationProvider`] for consistent error handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    /// Reference-counted input string that was processed.
    ///
    /// This field holds a shared reference to the original LaTeX/KaTeX input
    /// string that was tokenized. It's used for location calculations and
    /// error messages.
    pub input: Arc<str>,

    /// Zero-based inclusive start offset in the input string.
    ///
    /// This represents the byte position where the token or expression begins.
    /// For multi-byte UTF-8 characters, this points to the start of the first
    /// byte.
    pub start: usize,

    /// Zero-based exclusive end offset in the input string.
    ///
    /// This represents the byte position immediately after the token or
    /// expression ends. The range `[start, end)` defines the exact span of
    /// the source location.
    pub end: usize,
}

impl SourceLocation {
    /// Creates a new `SourceLocation` with the given input string and position
    /// range.
    ///
    /// This constructor initializes a source location for tracking a specific
    /// span in the input string. The position range is defined by byte offsets
    /// relative to the input string.
    ///
    /// # Parameters
    ///
    /// - `input`: Reference-counted input string that was processed
    /// - `start`: Zero-based inclusive start offset
    /// - `end`: Zero-based exclusive end offset
    ///
    /// # Returns
    ///
    /// A new `SourceLocation` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use katex::types::SourceLocation;
    /// use std::sync::Arc;
    ///
    /// let input = Arc::from("x^2");
    /// let loc = SourceLocation::new(input, 0, 3);
    /// ```
    #[must_use]
    pub const fn new(input: Arc<str>, start: usize, end: usize) -> Self {
        Self { input, start, end }
    }

    /// Creates a new `SourceLocation` from a string slice and position range.
    ///
    /// This is a convenience method that creates a reference-counted string
    /// from the provided string slice and initializes a source location.
    ///
    /// # Parameters
    ///
    /// - `input`: String slice containing the input that was processed
    /// - `start`: Zero-based inclusive start offset
    /// - `end`: Zero-based exclusive end offset
    ///
    /// # Returns
    ///
    /// A new `SourceLocation` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use katex::types::SourceLocation;
    ///
    /// let loc = SourceLocation::from_str("x^2", 0, 3);
    /// ```
    #[must_use]
    pub fn from_str(input: &str, start: usize, end: usize) -> Self {
        Self::new(Arc::from(input), start, end)
    }

    /// Returns the start offset of this source location.
    ///
    /// This method provides access to the zero-based inclusive start position
    /// within the input string where the tracked element begins.
    ///
    /// # Returns
    ///
    /// The start offset as a `usize`.
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Returns the end offset of this source location.
    ///
    /// This method provides access to the zero-based exclusive end position
    /// within the input string where the tracked element ends.
    ///
    /// # Returns
    ///
    /// The end offset as a `usize`.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Returns a reference to the input string associated with this source
    /// location.
    ///
    /// This method allows access to the original input string that was
    /// processed, which can be used for location calculations and error
    /// messages.
    ///
    /// # Returns
    ///
    /// A string slice containing the input text.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Returns the input string as an `Arc<String>` for sharing.
    ///
    /// This method provides access to the reference-counted input string,
    /// allowing it to be shared with other SourceLocation instances.
    ///
    /// # Returns
    ///
    /// A clone of the `Arc<str>` containing the input text.
    #[must_use]
    pub fn input_arc(&self) -> Arc<str> {
        Arc::clone(&self.input)
    }

    /// Merges two `SourceLocation`s into a single range.
    ///
    /// This method combines two source locations into one that spans from the
    /// start of the first to the end of the second. Both locations must use the
    /// same input string. If either location is `None`, the other is returned.
    /// If the input strings differ, `None` is returned.
    ///
    /// # Parameters
    ///
    /// - `first`: Optional first source location
    /// - `second`: Optional second source location
    ///
    /// # Returns
    ///
    /// An `Option` containing a `SourceLocation` representing the merged range,
    /// or `None` if merging is not possible.
    ///
    /// # Error Handling
    ///
    /// Returns `None` if:
    /// - Both locations are `None`
    /// - The locations use different input strings
    #[must_use]
    pub fn range(first: Option<Self>, second: Option<Self>) -> Option<Self> {
        match (first, second) {
            (Some(fp), None) => Some(fp),
            (None, Some(sp)) => Some(sp),
            (Some(fp), Some(sp)) => {
                if !Arc::ptr_eq(&fp.input, &sp.input) {
                    return None;
                }
                Some(Self {
                    input: Arc::clone(&fp.input),
                    start: fp.start,
                    end: sp.end,
                })
            }
            _ => None,
        }
    }
}

/// Trait for creating a `SourceLocation` range from two references.
pub trait SourceRangeRef {
    /// Create a new `SourceLocation` by reference
    #[must_use]
    fn range_ref(self, second: Self) -> Option<SourceLocation>;
}

impl SourceRangeRef for Option<&SourceLocation> {
    fn range_ref(self, second: Self) -> Option<SourceLocation> {
        match (self, second) {
            (Some(fp), None) => Some(fp.clone()),
            (None, Some(sp)) => Some(sp.clone()),
            (Some(fp), Some(sp)) => {
                if !Arc::ptr_eq(&fp.input, &sp.input) {
                    return None;
                }
                Some(SourceLocation {
                    input: Arc::clone(&fp.input),
                    start: fp.start,
                    end: sp.end,
                })
            }
            _ => None,
        }
    }
}

/// Implementation of `ErrorLocationProvider` for `SourceLocation`.
///
/// This implementation allows `SourceLocation` to be used as an error location
/// provider in the KaTeX parsing pipeline. It simply returns a reference to
/// itself, as `SourceLocation` already contains the necessary location
/// information.
///
/// # Cross-references
///
/// - Part of the error reporting system in
///   `ParseError`(crate::types::ParseError).
/// - Used by parsers to provide location context for syntax errors.
impl ErrorLocationProvider for SourceLocation {
    fn loc(&self) -> Option<&SourceLocation> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_creation() {
        let input = Arc::from("This is a test expression with invalid syntax");
        let loc = SourceLocation::new(Arc::clone(&input), 10, 20);

        assert_eq!(loc.start(), 10);
        assert_eq!(loc.end(), 20);
        assert_eq!(loc.input(), "This is a test expression with invalid syntax");
    }

    #[test]
    fn test_from_str_creation() {
        let loc = SourceLocation::from_str("test input", 5, 15);

        assert_eq!(loc.start(), 5);
        assert_eq!(loc.end(), 15);
        assert_eq!(loc.input(), "test input");
    }

    #[test]
    fn test_range_method() {
        let input = Arc::from("test input");

        let loc1 = SourceLocation::new(Arc::clone(&input), 5, 15);
        let result = SourceLocation::range(Some(loc1.clone()), None);
        assert_eq!(result.as_ref().unwrap().end(), 15);

        let loc2 = SourceLocation::new(Arc::clone(&input), 20, 30);
        let result = SourceLocation::range(Some(loc1.clone()), Some(loc2));
        assert_eq!(result.as_ref().unwrap().end(), 30);

        let different_input = Arc::from("different input");
        let loc3 = SourceLocation::new(Arc::clone(&different_input), 40, 50);
        let result = SourceLocation::range(Some(loc1), Some(loc3));
        assert!(result.is_none()); // Different inputs should not merge
    }
}
