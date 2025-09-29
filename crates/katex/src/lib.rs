//! KaTeX Rust implementation - Fast math typesetting for the web
//!
//! This is a Rust port of the KaTeX JavaScript library, providing
//! fast LaTeX math rendering capabilities.
#![warn(missing_docs)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::str_to_string)]
#![warn(clippy::non_ascii_literal)]
#![warn(clippy::pointer_format)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]
#![warn(clippy::absolute_paths)]
#![warn(clippy::panic)]
#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_in_result)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::unused_trait_names)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::impl_trait_in_params)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::unimplemented)]
#![warn(clippy::return_and_then)]
#![warn(clippy::needless_raw_strings)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::rc_buffer)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::map_with_unused_argument_over_ranges)]
#![warn(clippy::missing_asserts_for_indexing)]
#![warn(clippy::separated_literal_suffix)]
#![warn(clippy::ref_patterns)]
// Not sure
#![allow(clippy::indexing_slicing)]
#![allow(clippy::string_slice)]
#![allow(clippy::pub_use)]
// clippy exceptions
#![allow(clippy::float_cmp)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::default_numeric_fallback)]
#![allow(clippy::single_call_fn)]

extern crate alloc;
pub mod build_common;
pub mod build_html;
pub mod build_mathml;
pub mod build_tree;
pub mod context;
pub mod core;
pub mod define_environment;
pub mod define_function;
pub mod delimiter;
pub mod dom_tree;
pub mod font_metrics;
pub mod font_metrics_data;
pub mod functions;
pub mod lexer;
pub mod macro_expander;
pub mod macros;
pub mod mathml_tree;
pub mod options;
/// Utilities for working with parse trees and converting them to ParseNode
pub mod parse_tree;
/// Core parsing logic for LaTeX mathematical expressions.
pub mod parser;
pub mod spacing_data;
pub mod stretchy;
pub mod style;
pub mod svg_geometry;
pub mod symbols;
pub mod tree;
pub mod types;
pub mod unicode;
pub mod units;
pub mod utils;
#[cfg(feature = "wasm")]
pub mod wasm;
/// Shared web context for DOM operation
#[cfg(feature = "wasm")]
pub mod web_context;
pub mod wide_character;

pub use crate::types::ClassList;

/// Global context for KaTeX operations, containing all registered functions,
/// HTML/MathML builders, symbols, environments, and macros. This context is
/// essential for parsing and rendering mathematical expressions and provides
/// the foundation for KaTeX's extensibility.
///
/// The context manages:
/// - Functions: All registered LaTeX commands and their parsing behavior
/// - Builders: HTML and MathML generation functions for different node types
/// - Symbols: Character mappings for mathematical symbols across different
///   modes
/// - Environments: Support for LaTeX environments like matrices and arrays
/// - Macros: User-defined and system macros for expression expansion
///
/// # Examples
///
/// ```rust
/// use katex::*;
///
/// let mut ctx = KatexContext::default();
/// // Context is now ready with default functions and symbols
/// ```
pub use crate::context::KatexContext;

/// Parses and renders a LaTeX mathematical expression to an HTML string.
///
/// This function takes a LaTeX math expression and converts it into HTML markup
/// that can be displayed in web browsers. It handles the complete pipeline from
/// parsing to HTML generation, including symbol lookup, layout calculations,
/// and CSS styling.
///
/// # Parameters
///
/// * `ctx` - The [`KatexContext`] containing registered functions and symbols
/// * `expression` - The LaTeX mathematical expression to render
/// * `settings` - Rendering configuration controlling output format, strict
///   mode, custom macros, and other behaviour
///
/// # Returns
///
/// Returns `Result<String, ParseError>` where:
/// - `Ok(html_string)` contains the rendered HTML markup
/// - `Err(error)` contains detailed parsing or rendering error information
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// use katex::{KatexContext, Settings, render_to_string};
///
/// fn main() -> Result<(), katex::ParseError> {
///     let ctx = KatexContext::default();
///     let settings = Settings::default();
///
///     let html = render_to_string(&ctx, r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", &settings)?;
///     println!("{html}");
///     Ok(())
/// }
/// ```
///
/// With custom settings:
/// ```rust
/// use katex::{KatexContext, Settings, render_to_string};
///
/// let ctx = KatexContext::default();
/// let mut settings = Settings::default();
/// settings.display_mode = true; // Render in display/block mode
///
/// let html = render_to_string(&ctx, r"\sum_{i=1}^{n} x_i", &settings).unwrap();
/// println!("{html}");
/// ```
///
/// Here's an example that triggers an error:
///
/// The function provides detailed error messages with position information:
/// ```rust
/// use katex::{KatexContext, Settings, render_to_string};
///
/// let ctx = KatexContext::default();
/// let settings = Settings::default();
///
/// match render_to_string(&ctx, r"\frac{a}{", &settings) {
///     Ok(_) => println!("Success"),
///     Err(e) => println!("Error at position {}: {}", e.position.unwrap_or(0), e),
/// }
/// ```
///
/// # Performance
///
/// For optimal performance in high-throughput applications, consider reusing
/// [`Settings`] objects rather than creating new ones for each render call.
pub use crate::core::render_to_string;

/// Parse an expression and return the parse tree
///
/// This function parses a LaTeX expression and returns the raw parse tree,
/// equivalent to the `__parse` function in the JavaScript KaTeX API.
///
/// NOTE: This method is not currently recommended for public use.
/// The internal tree representation is unstable and is very likely
/// to change. Use at your own risk.
///
/// # Parameters
/// * `ctx` - The KaTeX context
/// * `expression` - The LaTeX expression to parse
/// * `settings` - Optional settings for parsing
///
/// # Returns
/// A `Result` containing the parse tree as a vector of `AnyParseNode`s or a
/// `ParseError`
pub use crate::core::parse;

/// Render an expression to a DOM tree
///
/// This function parses and builds a LaTeX expression, returning the DOM tree
/// representation, equivalent to the `__renderToDomTree` function in the
/// JavaScript KaTeX API.
///
/// NOTE: This method is not currently recommended for public use.
/// The internal tree representation is unstable and is very likely
/// to change. Use at your own risk.
///
/// # Parameters
/// * `ctx` - The KaTeX context
/// * `expression` - The LaTeX expression to render
/// * `settings` - Optional settings for rendering
///
/// # Returns
/// A `Result` containing the DOM tree or a `ParseError`
pub use crate::core::render_to_dom_tree;

/// Render an expression to an HTML-only DOM tree
///
/// This function parses and builds a LaTeX expression, returning the HTML-only
/// DOM tree representation, equivalent to the `__renderToHTMLTree` function in
/// the JavaScript KaTeX API.
///
/// NOTE: This method is not currently recommended for public use.
/// The internal tree representation is unstable and is very likely
/// to change. Use at your own risk.
///
/// # Parameters
/// * `ctx` - The KaTeX context
/// * `expression` - The LaTeX expression to render
/// * `settings` - Optional settings for rendering
///
/// # Returns
/// A `Result` containing the HTML DOM tree or a `ParseError`
pub use crate::core::render_to_html_tree;

/// Retrieves character metrics for a specific character in a given font family
/// and mode.
///
/// This function provides access to detailed font metrics used in mathematical
/// typesetting, including character dimensions, spacing, and positioning
/// information. The metrics are essential for proper alignment and spacing in
/// mathematical expressions.
///
/// # Parameters
///
/// * `ctx` - Shared [`KatexContext`] containing font metric tables
/// * `character` - The character or symbol to look up
/// * `font` - The font family name (e.g., "Main-Regular", "AMS-Regular",
///   "Caligraphic-Regular")
/// * `mode` - The mathematical mode (Mode::Math or Mode::Text) affecting metric
///   selection
///
/// # Returns
///
/// Returns `Result<Option<&CharacterMetrics>, ParseError>`.
/// - `Ok(Some(metrics))` contains the character dimensions
/// - `Ok(None)` indicates that no metrics are available for the requested
///   character/font combination
/// - `Err(error)` indicates that the font family itself was unknown
///
/// # Examples
///
/// ```rust
/// use katex::types::Mode;
/// use katex::{KatexContext, ParseError, get_character_metrics};
///
/// fn main() -> Result<(), ParseError> {
///     let ctx = KatexContext::default();
///
///     if let Some(m) = get_character_metrics(&ctx, 'A', "Main-Regular", Mode::Math)? {
///         println!("Character width: {}", m.width);
///         println!("Character height: {}", m.height);
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Font Families
///
/// Supported font families include:
/// - `Main-Regular`: Primary mathematical symbols and Latin letters
/// - `AMS-Regular`: Additional mathematical symbols from AMS
/// - `Caligraphic-Regular`: Calligraphic/script style letters
/// - `Fraktur-Regular`: Fraktur/Gothic style letters
/// - `SansSerif-Regular`: Sans-serif mathematical symbols
/// - And many others for specialized mathematical notation
///
/// # Notes
///
/// Character metrics are pre-computed from TeX font files and stored at compile
/// time. For characters not directly available, the function may use fallback
/// approximations or character mappings to provide reasonable spacing
/// estimates.
pub use crate::font_metrics::get_character_metrics;

/// Font metrics structure containing TeX font parameters for mathematical
/// typesetting.
///
/// This struct encapsulates the complete set of font metrics derived from TeX's
/// font parameters, providing all the spacing, sizing, and positioning
/// information needed for high-quality mathematical rendering. The metrics are
/// organized by font size index and include both general layout parameters and
/// size-specific measurements.
///
/// # Fields
///
/// * `css_em_per_mu` - Conversion factor from CSS em units to mathematical mu
///   units
/// * `metrics` - KeyMap containing named metric values indexed by parameter
///   name
///
/// # Available Metrics
///
/// The metrics include TeX parameters such as:
/// - **Spacing**: `slant`, `space`, `stretch`, `shrink`, `extraSpace`
/// - **Sizing**: `xHeight`, `quad`, `ptPerEm`
/// - **Superscripts**: `sup1`, `sup2`, `sup3`, `supDrop`
/// - **Subscripts**: `sub1`, `sub2`, `subDrop`
/// - **Fractions**: `num1`, `num2`, `num3`, `denom1`, `denom2`
/// - **Delimiters**: `delim1`, `delim2`
/// - **Rules**: `axisHeight`, `defaultRuleThickness`, `sqrtRuleThickness`
/// - **Operators**: `bigOpSpacing1` through `bigOpSpacing5`
///
/// # Size Indices
///
/// Font metrics are available for three size indices:
/// - Index 0: Text style (normal size, >= 9pt)
/// - Index 1: Script style (medium size, 7-8pt)
/// - Index 2: Scriptscript style (small size, 5-6pt)
///
/// Use `get_global_metrics()` to obtain the appropriate metrics for a given
/// font size.
pub use crate::font_metrics_data::CharacterMetrics;

/// Font metrics data structure providing access to character-specific
/// measurements.
///
/// This struct serves as the main interface for accessing pre-computed font
/// metrics for individual characters across different font families. The
/// metrics are generated at compile time from KaTeX's data files and stored in
/// perfect hash tables for efficient lookup during rendering.
///
/// # Examples
///
/// ```rust
/// use katex::{FontMetricsData, ParseError};
///
/// fn main() -> Result<(), ParseError> {
///     let data = FontMetricsData::default();
///
///     if let Some(metrics) = data.get_metric("Main-Regular", 'A' as u32)? {
///         println!("Width: {}", metrics.width);
///     }
///
///     Ok(())
/// }
/// ```
pub use crate::font_metrics_data::FontMetricsData;

/// Error type for parsing and rendering failures in KaTeX expressions.
///
/// This struct represents errors that occur during the parsing or rendering of
/// LaTeX mathematical expressions. It provides detailed context about what went
/// wrong, including the error message, position in the source string, and
/// surrounding context for debugging.
///
/// `ParseError` implements the standard `Error` and `Display` traits, making
/// it compatible with Rust's error handling ecosystem.
///
/// # Examples
///
/// ```rust
/// use katex::{KatexContext, ParseError, Settings, render_to_string};
///
/// fn try_render() -> Result<(), ParseError> {
///     let ctx = KatexContext::default();
///     let settings = Settings::default();
///
///     render_to_string(&ctx, r"\frac{a}{", &settings)?;
///     Ok(())
/// }
///
/// fn main() {
///     if let Err(err) = try_render() {
///         println!("Parse error: {}", err);
///         if let Some(pos) = err.position {
///             println!("Error at position: {pos}");
///         }
///         println!("Error kind: {:?}", err.kind);
///     }
/// }
/// ```
///
/// # Error Context
///
/// When created with location information (via [`ParseError::with_token`]),
/// the error message includes:
/// - The position in the source string (1-indexed)
/// - The high-level error category via [`ParseError::kind`]
/// - Surrounding context with the problematic text underlined
///
/// # Integration
///
/// `ParseError` integrates seamlessly with the `?` operator and standard
/// error handling patterns.
pub use crate::types::ParseError;

pub use crate::types::OutputFormat;
/// Main configuration structure for KaTeX rendering behavior.
///
/// This struct contains all the settings that control how mathematical
/// expressions are parsed and rendered. It provides comprehensive control over
/// display modes, error handling, font options, macro definitions, and other
/// rendering parameters.
///
/// [`Settings`] exposes the same configuration knobs as the JavaScript KaTeX
/// API, including display mode, output format, strict-mode validation, trust
/// callbacks, and custom macro definitions. All fields are public, so they can
/// be adjusted directly or via the generated builder (`Settings::builder()`).
///
/// # Examples
///
/// ```rust
/// use katex::Settings;
///
/// let mut settings = Settings::default();
/// settings.display_mode = true;
/// settings.throw_on_error = false;
/// ```
///
/// Using the builder for ergonomic construction:
/// ```rust
/// use katex::Settings;
///
/// let settings = Settings::builder()
///     .display_mode(true)
///     .throw_on_error(false)
///     .build();
/// assert!(settings.display_mode);
/// assert!(!settings.throw_on_error);
/// ```
///
/// # Default Values
///
/// - `display_mode`: `false` (inline mode)
/// - `output`: [`OutputFormat::HtmlAndMathml`]
/// - `throw_on_error`: `true`
/// - `error_color`: `"#cc0000"` (red)
/// - `max_size`: `f64::INFINITY`
/// - `max_expand`: `1000`
/// - Other fields have appropriate defaults
///
/// # Performance Considerations
///
/// For high-performance applications, create [`Settings`] objects once and
/// reuse them rather than creating new ones for each render operation.
pub use crate::types::Settings;
/// Strictness and trust configuration types used by [`Settings`].
///
/// These enums and callback types mirror KaTeX's JavaScript configuration and
/// control validation of potentially unsafe commands (`trust`) as well as how
/// strictly to enforce LaTeX syntax (`strict`).
pub use crate::types::{
    StrictFunction, StrictMode, StrictReturn, StrictSetting, TrustContext, TrustFunction,
    TrustSetting,
};

// Build utilities for advanced users creating custom DOM structures
/// Creates a line span with the given className, options, and thickness.
/// See [`build_common::make_line_span`] for detailed documentation.
pub use crate::build_common::make_line_span;

/// Creates an anchor element with the given href, classes, children, and
/// options. See [`build_common::make_anchor`] for detailed documentation.
pub use crate::build_common::make_anchor;

/// Creates a document fragment with the given list of children.
/// See [`build_common::make_fragment`] for detailed documentation.
pub use crate::build_common::make_fragment;

/// Wraps a group in a span if it's a document fragment.
/// See [`build_common::wrap_fragment`] for detailed documentation.
pub use crate::build_common::wrap_fragment;

pub mod namespace;

/// Current version of the KaTeX Rust implementation
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
