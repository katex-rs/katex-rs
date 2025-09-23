//! Parse error handling for KaTeX
//!
//! This module contains the ParseError implementation that mirrors the
//! JavaScript ParseError class functionality, providing detailed error context
//! with positioning information for where in the source string the problem
//! occurred.

extern crate alloc;

use crate::parser::ParseNodeError;
use crate::parser::parse_node::{AnyParseNode, ParseNodeOp};
use crate::types::SourceLocation;
use core::fmt::Write as _;
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use thiserror::Error;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Main error type thrown by KaTeX functions when something has gone wrong.
/// This is used to distinguish internal errors from errors in the expression
/// that the user provided.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Error)]
#[error("{message}")]
pub struct ParseError {
    /// The error message with context information
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
    pub message: String,
    /// The start position based on passed-in Token or ParseNode
    pub position: Option<usize>,
    /// The length of affected text based on passed-in Token or ParseNode
    pub length: Option<usize>,
    /// The underlying error message without any context added
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
    pub raw_message: String,
    /// Backtrace of the error stack
    #[cfg(feature = "backtrace")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub backtrace: Box<Backtrace>,
}

impl ParseError {
    /// Create a new ParseError with the given message
    pub fn new<T: Into<String>>(message: T) -> Self {
        let raw_message = message.into();
        let error_message = format!("KaTeX parse error: {raw_message}");
        Self {
            message: error_message,
            position: None,
            length: None,
            raw_message,
            #[cfg(feature = "backtrace")]
            backtrace: Box::new(Backtrace::force_capture()),
        }
    }

    /// Create a new ParseError with context from a Token or ParseNode
    pub fn with_token<T: Into<String>>(message: T, token: &dyn ErrorLocationProvider) -> Self {
        let raw_message = message.into();
        let mut error_message = format!("KaTeX parse error: {raw_message}");

        let (position, length) = token.loc().map_or((None, None), |loc| {
            if loc.start() <= loc.end() {
                let input = loc.input();
                let start = loc.start();
                let end = loc.end();
                if start == input.len() {
                    error_message += " at end of input: ";
                } else {
                    let _ = write!(error_message, " at position {}: ", start + 1);
                }
                if start > 15 {
                    let mut boundary = start - 15;
                    while !input.is_char_boundary(boundary) && boundary > 0 {
                        // Move to the next char boundary
                        boundary -= 1;
                    }
                    let _ = write!(error_message, "\u{2026}{}", &input[boundary..start]);
                } else {
                    let _ = write!(error_message, "{}", &input[..start]);
                }
                if end > start {
                    input[start..end].chars().for_each(|c| {
                        let _ = write!(error_message, "{c}\u{0332}");
                    });
                }
                if end + 15 < input.len() {
                    let mut boundary = end + 15;
                    while !input.is_char_boundary(boundary) && boundary < input.len() {
                        boundary += 1;
                    }
                    let _ = write!(error_message, "{}\u{2026}", &input[end..boundary]);
                } else {
                    let _ = write!(error_message, "{}", &input[end..]);
                }
                (Some(start), Some(end - start))
            } else {
                (None, None)
            }
        });

        Self {
            message: error_message,
            position,
            length,
            raw_message,
            #[cfg(feature = "backtrace")]
            backtrace: Box::new(Backtrace::force_capture()),
        }
    }

    /// Get the raw error message without context
    #[must_use]
    pub fn raw_message(&self) -> &str {
        &self.raw_message
    }
}

impl From<strum::ParseError> for ParseError {
    fn from(err: strum::ParseError) -> Self {
        Self::new(format!("Enum parse error: {err}"))
    }
}

/// Trait for types that can provide error location information for ParseError
pub trait ErrorLocationProvider {
    /// Get the source location if available
    fn loc(&self) -> Option<&SourceLocation>;
}

/// Implementation of [`ErrorLocationProvider`] for [`AnyParseNode`].
///
/// This implementation extracts the source location from various parse node
/// types used in mathematical expression parsing. It handles all variants of
/// [`AnyParseNode`] to provide accurate error positioning in LaTeX/KaTeX
/// expressions.
///
/// The location information is crucial for generating user-friendly error
/// messages that highlight the exact position in the input string where a
/// parsing error occurred.
///
/// # Error Handling
/// Returns `None` if the parse node does not have location information
/// available.
///
/// # See Also
/// - [`ParseError::with_token`] for creating errors with location context
/// - [`SourceLocation`] for detailed location tracking
/// - [`AnyParseNode`] for all supported parse node types
// Implement ErrorLocationProvider for AnyParseNode
impl ErrorLocationProvider for AnyParseNode {
    fn loc(&self) -> Option<&SourceLocation> {
        // Extract location from the various parse node types
        match self {
            Self::Array(node) => node.loc.as_ref(),
            Self::CdLabel(node) => node.loc.as_ref(),
            Self::CdLabelParent(node) => node.loc.as_ref(),
            Self::Color(node) => node.loc.as_ref(),
            Self::ColorToken(node) => node.loc.as_ref(),
            Self::Op(node) => match node {
                ParseNodeOp::Symbol { loc, .. } | ParseNodeOp::Body { loc, .. } => loc.as_ref(),
            },
            Self::OrdGroup(node) => node.loc.as_ref(),
            Self::Raw(node) => node.loc.as_ref(),
            Self::Size(node) => node.loc.as_ref(),
            Self::Styling(node) => node.loc.as_ref(),
            Self::SupSub(node) => node.loc.as_ref(),
            Self::Tag(node) => node.loc.as_ref(),
            Self::Text(node) => node.loc.as_ref(),
            Self::Url(node) => node.loc.as_ref(),
            Self::Verb(node) => node.loc.as_ref(),
            Self::Atom(node) => node.loc.as_ref(),
            Self::MathOrd(node) => node.loc.as_ref(),
            Self::Spacing(node) => node.loc.as_ref(),
            Self::TextOrd(node) => node.loc.as_ref(),
            Self::AccentToken(node) => node.loc.as_ref(),
            Self::OpToken(node) => node.loc.as_ref(),
            Self::Accent(node) => node.loc.as_ref(),
            Self::AccentUnder(node) => node.loc.as_ref(),
            Self::Cr(node) => node.loc.as_ref(),
            Self::Delimsizing(node) => node.loc.as_ref(),
            Self::Enclose(node) => node.loc.as_ref(),
            Self::Environment(node) => node.loc.as_ref(),
            Self::Font(node) => node.loc.as_ref(),
            Self::Genfrac(node) => node.loc.as_ref(),
            Self::Hbox(node) => node.loc.as_ref(),
            Self::HorizBrace(node) => node.loc.as_ref(),
            Self::Href(node) => node.loc.as_ref(),
            Self::Html(node) => node.loc.as_ref(),
            Self::HtmlMathMl(node) => node.loc.as_ref(),
            Self::Includegraphics(node) => node.loc.as_ref(),
            Self::Infix(node) => node.loc.as_ref(),
            Self::Internal(node) => node.loc.as_ref(),
            Self::Kern(node) => node.loc.as_ref(),
            Self::Lap(node) => node.loc.as_ref(),
            Self::LeftRight(node) => node.loc.as_ref(),
            Self::LeftRightRight(node) => node.loc.as_ref(),
            Self::MathChoice(node) => node.loc.as_ref(),
            Self::Middle(node) => node.loc.as_ref(),
            Self::Mclass(node) => node.loc.as_ref(),
            Self::OperatorName(node) => node.loc.as_ref(),
            Self::Overline(node) => node.loc.as_ref(),
            Self::Phantom(node) => node.loc.as_ref(),
            Self::Hphantom(node) => node.loc.as_ref(),
            Self::Vphantom(node) => node.loc.as_ref(),
            Self::Pmb(node) => node.loc.as_ref(),
            Self::Raisebox(node) => node.loc.as_ref(),
            Self::Rule(node) => node.loc.as_ref(),
            Self::Sizing(node) => node.loc.as_ref(),
            Self::Smash(node) => node.loc.as_ref(),
            Self::Sqrt(node) => node.loc.as_ref(),
            Self::Underline(node) => node.loc.as_ref(),
            Self::Vcenter(node) => node.loc.as_ref(),
            Self::XArrow(node) => node.loc.as_ref(),
        }
    }
}

/// Implementation of [`ErrorLocationProvider`] for `Option<AnyParseNode>`.
///
/// Handles optional parse nodes, allowing error location extraction even when
/// the node might be absent. This is useful in complex parsing scenarios where
/// nodes are conditionally created or when parsing optional components of
/// mathematical expressions.
///
/// The implementation delegates to the inner node's location if present,
/// providing a consistent interface for error reporting.
///
/// # Error Handling
/// Returns `None` if the option is `None` or if the contained node has no
/// location.
///
/// # See Also
/// - [`AnyParseNode`] for all supported parse node variants
/// - [`ErrorLocationProvider`] trait for location interface
/// - [`ParseError`] for error types in mathematical parsing
impl ErrorLocationProvider for Option<AnyParseNode> {
    fn loc(&self) -> Option<&SourceLocation> {
        let n = self.as_ref()?;
        n.loc()
    }
}

/// Convert ParseNodeError to ParseError
impl From<ParseNodeError> for ParseError {
    fn from(err: ParseNodeError) -> Self {
        match err {
            ParseNodeError::TypeMismatch { expected, actual } => Self::new(format!(
                "Expected node of type {expected}, but got {actual}"
            )),
            ParseNodeError::NotSymbolNode { actual } => Self::new(format!(
                "Expected node of symbol group type, but got {actual}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Token;
    use alloc::sync::Arc;

    #[test]
    fn test_parse_error_creation() {
        let error = ParseError::new("Invalid syntax");
        assert!(error.message.contains("KaTeX parse error: Invalid syntax"));
        assert_eq!(error.raw_message, "Invalid syntax");
        assert_eq!(error.position, None);
        assert_eq!(error.length, None);
    }

    #[test]
    fn test_parse_error_with_token_context() {
        let input = Arc::from("This is a test expression with invalid syntax");
        let loc = SourceLocation::new(Arc::clone(&input), 10, 14); // "test"
        let token = Token::new("test".to_owned(), Some(loc));

        let error = ParseError::with_token("Invalid syntax", &token);
        assert!(error.message.contains("KaTeX parse error: Invalid syntax"));
        assert!(error.message.contains("at position 11"));
        assert_eq!(error.position, Some(10));
        assert_eq!(error.length, Some(4));
    }
}
