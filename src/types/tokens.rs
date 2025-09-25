use crate::types::{ErrorLocationProvider, SourceLocation};

/// Represents a single token in the lexing process of LaTeX/KaTeX mathematical
/// expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The raw text content of the token as extracted from the input string.
    ///
    /// This preserves the original characters without any processing or
    /// normalization. For commands, this includes the backslash and command
    /// name (e.g., "\\alpha").
    pub text: String,
    /// Optional source location information for error reporting and debugging.
    ///
    /// Provides context about where in the original input this token
    /// originated. Used to generate meaningful error messages when parsing
    /// fails.
    ///
    /// # See Also
    /// - [`SourceLocation`] for location details
    pub loc: Option<SourceLocation>,
    /// Flag indicating whether this token should not be expanded during macro
    /// processing.
    ///
    /// When set to `true`, prevents the parser from applying macro expansions
    /// to this token. This is useful for tokens that should be treated
    /// literally, such as in verbatim environments.
    pub noexpand: Option<bool>,
    /// Flag indicating whether this token should be treated as a `\relax`
    /// command.
    ///
    /// `\relax` is a LaTeX primitive that does nothing but can be used to
    /// prevent unwanted expansions. When this flag is set, the token
    /// behaves as if it were a `\relax` command.
    pub treat_as_relax: Option<bool>,
}

impl Token {
    /// Creates a new token with the specified text and optional source
    /// location.
    #[must_use]
    pub const fn new(text: String, loc: Option<SourceLocation>) -> Self {
        Self {
            text,
            loc,
            noexpand: None,
            treat_as_relax: None,
        }
    }

    /// Computes a new token that encompasses the range from this token to the
    /// end_token.
    #[must_use]
    pub fn range(self, end_token: Self, text: String) -> Option<Self> {
        let loc = SourceLocation::range(self.loc, end_token.loc)?;
        Some(Self {
            text,
            loc: Some(loc),
            noexpand: None,
            treat_as_relax: None,
        })
    }
}

/// Implementation of `ErrorLocationProvider` for `Token`.
///
/// This implementation allows `Token` to be used as an error location
/// provider in the KaTeX parsing pipeline. It simply returns the token's
/// location information for error reporting.
///
/// # Cross-references
///
/// - Part of the error reporting system in
///   `ParseError`(crate::types::ParseError).
/// - Used by parsers to provide location context for syntax errors.
impl ErrorLocationProvider for Token {
    fn loc(&self) -> Option<&SourceLocation> {
        self.loc.as_ref()
    }
}

/// Implementation of `ErrorLocationProvider` for `Option<Token>`.
///
/// This implementation allows optional tokens to be used as error location
/// providers. It delegates to the inner token's location if present.
///
/// # Cross-references
///
/// - Part of the error reporting system in
///   `ParseError`(crate::types::ParseError).
/// - Used when tokens might be absent but location information is still needed.
impl ErrorLocationProvider for Option<Token> {
    fn loc(&self) -> Option<&SourceLocation> {
        let t = self.as_ref()?;
        t.loc.as_ref()
    }
}
