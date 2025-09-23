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
/// Utilities for working with parse trees and converting them to [`ParseNode`]
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
pub mod wide_character;

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
/// * `expression` - The LaTeX mathematical expression to render
/// * `options` - Optional `SettingsOptions` to customize rendering behavior
///   such as:
///   - Display mode vs inline mode
///   - Output format (HTML, MathML, or both)
///   - Error handling preferences
///   - Font and styling options
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
/// use katex::*;
///
/// let ctx = KatexContext::default();
/// match render_to_string(&ctx, r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", None) {
///     Ok(html) => println!("{}", html),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
///
/// With custom options:
/// ```rust
/// use katex::*;
///
/// let ctx = KatexContext::default();
/// let options = SettingsOptions {
///     display_mode: Some(true),
///     ..Default::default()
/// };
/// match render_to_string(&ctx, r"\sum_{i=1}^{n} x_i", Some(options)) {
///     Ok(html) => println!("{}", html),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
///
/// # Error Handling
///
/// The function provides detailed error messages with position information:
/// ```rust
/// use katex::*;
///
/// let ctx = KatexContext::default();
/// match render_to_string(&ctx, r"\frac{a}{", None) {
///     Ok(_) => println!("Success"),
///     Err(e) => println!("Error at position {}: {}", e.position.unwrap_or(0), e),
/// }
/// ```
///
/// # Performance
///
/// For optimal performance in high-throughput applications, consider reusing
/// `Settings` objects rather than creating new ones for each render call.
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
/// * `character` - The character or symbol to look up (as a string)
/// * `font` - The font family name (e.g., "Main-Regular", "AMS-Regular",
///   "Caligraphic-Regular")
/// * `mode` - The mathematical mode (`Mode::Math` or `Mode::Text`) affecting
///   metric selection
///
/// # Returns
///
/// Returns `Option<FontMetrics>` containing:
/// - `depth`: How far below the baseline the character extends
/// - `height`: How far above the baseline the character extends
/// - `italic`: Italic correction for proper spacing after italic characters
/// - `skew`: Skew adjustment for superscript positioning
/// - `width`: Total width of the character
///
/// Returns `None` if the character or font is not found, or if metrics are
/// unavailable.
///
/// # Examples
///
/// ```rust
/// use katex::types::Mode;
/// use katex::*;
///
/// let metrics = get_character_metrics('A', "Main-Regular", Mode::Math);
/// if let Some(m) = metrics {
///     println!("Character width: {}", m.width);
///     println!("Character height: {}", m.height);
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
/// metrics for individual characters across different font families. It
/// provides methods to retrieve width, height, depth, and other typographic
/// measurements for mathematical symbols and text characters.
///
/// The metrics are generated at compile time from TeX font files and stored
/// in perfect hash tables for efficient lookup during rendering.
///
/// # Methods
///
/// * `get_metrics()` - Retrieves complete metrics for a character in a font
///   family
/// * `get_width()` - Gets just the width of a character
/// * `get_height()` - Gets just the height of a character
/// * `get_depth()` - Gets just the depth of a character
///
/// # Supported Font Families
///
/// The following font families are supported:
/// - `AMS-Regular`: American Mathematical Society symbols
/// - `Caligraphic-Regular`: Calligraphic/script letters
/// - `Fraktur-Regular`: Fraktur/Gothic letters
/// - `Main-Bold`, `Main-BoldItalic`, `Main-Italic`, `Main-Regular`: Primary
///   fonts
/// - `Math-BoldItalic`, `Math-Italic`: Mathematical alphabets
/// - `SansSerif-Bold`, `SansSerif-Italic`, `SansSerif-Regular`: Sans-serif
///   variants
/// - `Script-Regular`: Script style letters
/// - `Size1-Regular` through `Size4-Regular`: Size-specific fonts
/// - `Typewriter-Regular`: Monospace font
///
/// # Examples
///
/// ```rust
/// use katex::*;
///
/// // Get complete metrics for 'A' in main font
/// let metrics = FontMetricsData::get_metrics("Main-Regular", 'A' as u32);
/// if let Some(m) = metrics {
///     println!("Width: {}, Height: {}", m.width, m.height);
/// }
///
/// // Get just the width
/// let width = FontMetricsData::get_width("Main-Regular", 'A' as u32);
/// ```
///
/// # Character Encoding
///
/// Characters are specified by their Unicode code points (u32). For ASCII
/// characters, you can use `'A' as u32` or the numeric code point directly.
///
/// # Performance
///
/// Font metric lookups use perfect hash tables generated at compile time,
/// making them extremely fast and suitable for use in rendering loops.
pub use crate::font_metrics_data::FontMetricsData;

/// Error type for parsing and rendering failures in KaTeX expressions.
///
/// This struct represents errors that occur during the parsing or rendering of
/// LaTeX mathematical expressions. It provides detailed context about what went
/// wrong, including the error message, position in the source string, and
/// surrounding context for debugging.
///
/// ParseError implements the standard `Error` and `Display` traits, making it
/// compatible with Rust's error handling ecosystem.
///
/// # Fields
///
/// * `message` - The complete error message with context and position
///   information
/// * `position` - Optional byte position in the source string where the error
///   occurred
/// * `length` - Optional length of the problematic text segment
/// * `raw_message` - The original error message without added context
///
/// # Examples
///
/// ```rust
/// use katex::*;
///
/// let ctx = KatexContext::default();
/// match render_to_string(&ctx, r"\frac{a}{", None) {
///     Ok(_) => println!("Success"),
///     Err(e) => {
///         println!("Parse error: {}", e);
///         if let Some(pos) = e.position {
///             println!("Error at position: {}", pos);
///         }
///         println!("Raw message: {}", e.raw_message());
///     }
/// }
/// ```
///
/// # Error Context
///
/// When created with location information (via `ParseError::with_token()`),
/// the error message includes:
/// - The position in the source string (1-indexed)
/// - Surrounding context with the problematic text underlined
/// - Visual indicators showing exactly where the error occurred
///
/// # Integration
///
/// ParseError integrates seamlessly with `?` operator and error handling:
/// ```rust
/// use katex::*;
///
/// fn render_math(ctx: &KatexContext, expr: &str) -> Result<String, ParseError> {
///     render_to_string(ctx, expr, None)
/// }
/// ```
pub use crate::types::ParseError;

pub use crate::types::OutputFormat;
/// Main configuration structure for KaTeX rendering behavior.
///
/// This struct contains all the settings that control how mathematical
/// expressions are parsed and rendered. It provides comprehensive control over
/// display modes, error handling, font options, macro definitions, and other
/// rendering parameters.
///
/// Settings are typically created from `SettingsOptions` using the
/// `Settings::new()` method, which provides default values for any unspecified
/// options.
///
/// # Fields
///
/// * `display_mode` - Whether to render in display (block) or inline mode
/// * `output` - Output format: HTML, MathML, or both
/// * `leqno` - Left-align equation numbers
/// * `fleqn` - Flush left equations
/// * `throw_on_error` - Whether to throw errors or render error messages
/// * `error_color` - Color for error message rendering
/// * `macros` - Custom macro definitions
/// * `min_rule_thickness` - Minimum thickness for rules and lines
/// * `color_is_text_color` - Color interpretation mode
/// * `strict` - Strict mode settings for LaTeX compatibility checking
/// * `trust` - Trust settings for potentially dangerous content
/// * `max_size` - Maximum allowed size for expressions
/// * `max_expand` - Maximum expansion limit for macros
/// * `global_group` - Whether to use global grouping
///
/// # Examples
///
/// ```rust
/// use katex::*;
///
/// let options = SettingsOptions {
///     display_mode: Some(true),
///     output: Some(OutputFormat::HtmlAndMathml),
///     throw_on_error: Some(false),
///     ..Default::default()
/// };
///
/// let settings = Settings::new(options);
/// ```
///
/// # Default Values
///
/// - `display_mode`: `false` (inline mode)
/// - `output`: `OutputFormat::HtmlAndMathml`
/// - `throw_on_error`: `true`
/// - `error_color`: `"#cc0000"` (red)
/// - `max_size`: `f64::INFINITY`
/// - `max_expand`: `1000`
/// - Other fields have appropriate defaults
///
/// # Performance Considerations
///
/// For high-performance applications, create `Settings` objects once and reuse
/// them rather than creating new ones for each render operation.
pub use crate::types::Settings;
/// Options structure for configuring KaTeX rendering behavior.
///
/// This struct provides a convenient way to specify rendering options when
/// creating a `Settings` instance. All fields are optional, allowing you to
/// specify only the options you want to customize while using defaults for the
/// rest.
///
/// The fields correspond directly to the fields in `Settings`, but are wrapped
/// in `Option` to allow partial configuration.
///
/// # Fields
///
/// * `display_mode` - Render in display (block) mode if `Some(true)`
/// * `output` - Output format specification
/// * `leqno` - Left-align equation numbers
/// * `fleqn` - Flush left equations
/// * `throw_on_error` - Error handling behavior
/// * `error_color` - Error message color
/// * `macros` - Custom macro definitions
/// * `min_rule_thickness` - Minimum rule thickness
/// * `color_is_text_color` - Color interpretation mode
/// * `strict` - Strict mode configuration
/// * `trust` - Trust settings for security
/// * `max_size` - Maximum expression size limit
/// * `max_expand` - Maximum macro expansion limit
/// * `global_group` - Global grouping setting
///
/// # Examples
///
/// Basic usage with defaults:
/// ```rust
/// use katex::*;
///
/// let options = SettingsOptions {
///     display_mode: Some(true),
///     throw_on_error: Some(false),
///     ..Default::default()
/// };
/// ```
///
/// Advanced configuration:
/// ```rust
/// use katex::namespace::KeyMap;
/// use katex::*;
///
/// let mut macros = KeyMap::default();
/// macros.insert("\\RR".to_string(), "\\mathbb{R}".to_string());
///
/// let options = SettingsOptions {
///     display_mode: Some(true),
///     output: Some(OutputFormat::Html),
///     macros: Some(macros),
///     max_size: Some(1000.0),
///     ..Default::default()
/// };
/// ```
///
/// # Default Behavior
///
/// When `SettingsOptions` is created with `Default::default()`, all fields are
/// `None`, which means the `Settings::new()` method will use its own defaults
/// for all values.
///
/// # Conversion
///
/// Use `Settings::new(options)` to convert `SettingsOptions` into a complete
/// `Settings` struct:
///
/// ```rust
/// use katex::*;
///
/// let options = SettingsOptions::default();
/// let settings = Settings::new(options);
/// ```
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
