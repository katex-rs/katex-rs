//! Core type definitions for KaTeX Rust implementation

mod source_location;
use core::fmt;
use core::option;

use alloc::rc::Rc;

use crate::define_environment::EnvSpec;
use crate::define_function::FunctionSpec;
use crate::parser::parse_node::NodeType;
pub use source_location::{LexerInterface, SourceLocation};
use strum::AsRefStr;
use strum::Display;
use strum::EnumCount;
use strum::EnumIter;
use strum::EnumString;

/// Parse error type - imported from parse_error module
mod parse_error;
pub use parse_error::{ErrorLocationProvider, ParseError, ParseErrorKind};
use strum::FromRepr;

mod tokens;
pub use crate::symbols::Mode;
pub use tokens::{Token, TokenText};

mod settings;
pub use settings::{
    OutputFormat, Settings, StrictFunction, StrictMode, StrictReturn, StrictSetting, TrustContext,
    TrustFunction, TrustSetting,
};

pub use source_location::SourceRangeRef;

/// Represents a set of standard CSS property names.
///
/// This enum is serialized in `kebab-case` (the standard CSS syntax) via
/// `strum`. It can be used for mapping strongly-typed property names to their
/// CSS strings and is especially useful when generating styles
/// programmatically.
#[derive(
    EnumIter, Debug, Copy, AsRefStr, PartialEq, Eq, Hash, Clone, Display, EnumCount, FromRepr,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u8)]
pub enum CssProperty {
    /// Sets the background color of an element. See: <https://developer.mozilla.org/docs/Web/CSS/background-color>
    BackgroundColor,
    /// Sets the width of the bottom border of an element. See: <https://developer.mozilla.org/docs/Web/CSS/border-bottom-width>
    BorderBottomWidth,
    /// Sets the color of the border on all four sides of an element. See: <https://developer.mozilla.org/docs/Web/CSS/border-color>
    BorderColor,
    /// Sets the style of the right border. See: <https://developer.mozilla.org/docs/Web/CSS/border-right-style>
    BorderRightStyle,
    /// Sets the width of the right border of an element. See: <https://developer.mozilla.org/docs/Web/CSS/border-right-width>
    BorderRightWidth,
    /// Sets the width of the top border of an element. See: <https://developer.mozilla.org/docs/Web/CSS/border-top-width>
    BorderTopWidth,
    /// Sets the style of all four borders. See: <https://developer.mozilla.org/docs/Web/CSS/border-style>
    BorderStyle,
    /// Sets the width of all four borders. See: <https://developer.mozilla.org/docs/Web/CSS/border-width>
    BorderWidth,
    /// Specifies how far the bottom edge of an element is from the bottom edge of its containing block. See: <https://developer.mozilla.org/docs/Web/CSS/bottom>
    Bottom,
    /// Sets the color of the text content of an element. See: <https://developer.mozilla.org/docs/Web/CSS/color>
    Color,
    /// Specifies the height of an element. See: <https://developer.mozilla.org/docs/Web/CSS/height>
    Height,
    /// Specifies how far the left edge of an element is from the left edge of its containing block. See: <https://developer.mozilla.org/docs/Web/CSS/left>
    Left,
    /// Sets the margin area on all four sides of an element. See: <https://developer.mozilla.org/docs/Web/CSS/margin>
    Margin,
    /// Sets the margin area on the left side of an element. See: <https://developer.mozilla.org/docs/Web/CSS/margin-left>
    MarginLeft,
    /// Sets the margin area on the right side of an element. See: <https://developer.mozilla.org/docs/Web/CSS/margin-right>
    MarginRight,
    /// Sets the margin area on the top side of an element. See: <https://developer.mozilla.org/docs/Web/CSS/margin-top>
    MarginTop,
    /// Sets the minimum width of an element. See: <https://developer.mozilla.org/docs/Web/CSS/min-width>
    MinWidth,
    /// Sets the padding on the left side of an element. See: <https://developer.mozilla.org/docs/Web/CSS/padding-left>
    PaddingLeft,
    /// Specifies how an element is positioned in the document. See: <https://developer.mozilla.org/docs/Web/CSS/position>
    Position,
    /// Applies one or more shadows to text. See: <https://developer.mozilla.org/docs/Web/CSS/text-shadow>
    TextShadow,
    /// Specifies how far the top edge of an element is from the top edge of its containing block. See: <https://developer.mozilla.org/docs/Web/CSS/top>
    Top,
    /// Specifies the width of an element. See: <https://developer.mozilla.org/docs/Web/CSS/width>
    Width,
    /// Sets the vertical alignment of an inline or table-cell element. See: <https://developer.mozilla.org/docs/Web/CSS/vertical-align>
    VerticalAlign,
}

/// A type alias representing CSS style properties for HTML nodes in KaTeX
/// rendering.
///
/// This `HashMap` maps CSS property names (as strings) to their corresponding
/// values, enabling dynamic styling of mathematical expressions rendered as
/// HTML. It is used throughout the KaTeX rendering pipeline to apply visual
/// styles such as colors, fonts, spacing, and positioning to generated HTML
/// elements.
///
/// # LaTeX/KaTeX Context
/// In mathematical typesetting, CSS styles are crucial for controlling the
/// visual appearance of rendered math, including font variants (e.g., italic
/// for variables), display styles (inline vs. block), and layout adjustments.
/// This type supports KaTeX's approach to generating accessible and visually
/// consistent math output.
///
/// # Cross-references
/// - Used in [`TrustContext`] for inline style validation.
/// - See [`Settings`] for global styling options.
/// - Related to [`FontVariant`] for font-specific styling.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct CssStyle {
    map: [Option<Rc<str>>; CssProperty::COUNT],
}

impl fmt::Debug for CssStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use strum::IntoEnumIterator as _;

        let mut ds = f.debug_struct("CssStyle");
        for (property, value) in CssProperty::iter().zip(self.map.iter()) {
            if let Some(value) = value {
                ds.field(property.as_ref(), value);
            }
        }
        ds.finish()
    }
}

/// Iterator over CSS style properties
pub struct CssStyleIter<'a> {
    index: usize,
    data: &'a [Option<Rc<str>>; CssProperty::COUNT],
}

impl<'a> Iterator for CssStyleIter<'a> {
    type Item = (CssProperty, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < CssProperty::COUNT {
            let idx = self.index;
            self.index += 1;
            if let Some(v) = &self.data[idx]
                && let Some(prop) = CssProperty::from_repr(idx as u8)
            {
                return Some((prop, v.as_ref()));
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a CssStyle {
    type Item = (CssProperty, &'a str);
    type IntoIter = CssStyleIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        CssStyleIter {
            index: 0,
            data: &self.map,
        }
    }
}

impl CssStyle {
    /// Inserts or updates a CSS property with the given value.
    #[inline]
    pub fn insert<T>(&mut self, property: CssProperty, value: T)
    where
        T: Into<Rc<str>>,
    {
        self.map[property as usize] = Some(value.into());
    }

    /// Extends the current style with properties from another `CssStyle`.
    pub fn extend(&mut self, other: &Self) {
        for (i, value) in other.map.iter().enumerate() {
            if let Some(value) = value {
                self.map[i] = Some(Rc::clone(value));
            }
        }
    }

    #[inline]
    /// Checks if the style contains a specific CSS property.
    #[must_use]
    pub const fn contains_key(&self, property: CssProperty) -> bool {
        self.map[property as usize].is_some()
    }

    #[inline]
    /// Retrieves the value of a specific CSS property, if it exists.
    #[must_use]
    pub fn get(&self, property: CssProperty) -> Option<&str> {
        self.map[property as usize].as_deref()
    }

    /// Checks if the style is empty (contains no properties).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.iter().all(option::Option::is_none)
    }

    /// Iterates over all CSS properties and their values in the style.
    #[must_use]
    pub fn iter(&self) -> CssStyleIter<'_> {
        self.into_iter()
    }
}

/// Enumeration of LaTeX argument types used in KaTeX parsing and rendering.
///
/// This enum categorizes the different kinds of arguments that LaTeX commands
/// and functions can accept, enabling type-safe handling during mathematical
/// expression processing. Each variant represents a specific argument category
/// with distinct parsing and validation rules.
///
/// # LaTeX/KaTeX Context
/// In LaTeX, commands often require arguments of specific types (e.g., colors,
/// sizes, URLs). KaTeX extends this by providing structured argument types
/// that facilitate robust parsing and rendering of complex mathematical
/// expressions, including those with embedded URLs, custom styling, and
/// mode-specific content.
///
/// # Cross-references
/// - See [`Mode`] for mathematical rendering modes.
/// - Used in [`Settings`] for macro and command validation.
/// - Related to [`TrustContext`] for security validation of URLs and styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    /// Color specification argument (e.g., for `\color{red}`).
    ///
    /// Represents arguments that specify colors, such as named colors,
    /// hex values, or RGB specifications used in color commands.
    Color,
    /// Size specification argument (e.g., for `\fontsize{12pt}{14pt}`).
    ///
    /// Represents arguments that define sizes, including font sizes,
    /// spacing dimensions, and other length specifications.
    Size,
    /// URL string argument (e.g., for `\href{http://example.com}{text}`).
    ///
    /// Represents arguments containing URLs, subject to trust validation
    /// to prevent security vulnerabilities.
    Url,
    /// Raw string argument without special processing.
    ///
    /// Represents plain text arguments that should be used as-is,
    /// without LaTeX interpretation or escaping.
    Raw,
    /// Original mode type argument.
    ///
    /// Represents arguments that preserve the original parsing mode,
    /// useful for commands that need to maintain context.
    Original,
    /// Horizontal box argument (e.g., for `\hbox{content}`).
    ///
    /// Represents arguments that create horizontal boxes for layout control.
    Hbox,
    /// Primitive type argument.
    ///
    /// Represents low-level primitive arguments used in core LaTeX operations.
    Primitive,
    /// Mode-specific type argument.
    ///
    /// Represents arguments that are specific to a particular mathematical
    /// rendering mode (e.g., inline math vs. display math).
    ///
    /// # Parameters
    /// - `mode`: The specific [`Mode`] for this argument type.
    Mode(Mode),
}

/// Enumeration of style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum StyleVariant {
    /// Text style for inline mathematical expressions.
    ///
    /// Used for math embedded within text, with normal font size and spacing.
    /// Corresponds to LaTeX's `\textstyle`.
    Text,
    /// Display style for block mathematical expressions.
    ///
    /// Used for standalone equations, with larger font size and centered
    /// layout. Corresponds to LaTeX's `\displaystyle`.
    Display,
    /// Script style for first-level superscripts and subscripts.
    ///
    /// Used for exponents and indices, with reduced font size.
    /// Corresponds to LaTeX's `\scriptstyle`.
    Script,
    /// Scriptscript style for nested superscripts and subscripts.
    ///
    /// Used for exponents of exponents, with further reduced font size.
    /// Corresponds to LaTeX's `\scriptscriptstyle`.
    ScriptScript,
}

/// Enumeration of token types that can break or terminate parsing in KaTeX.
///
/// This enum represents special tokens that signal the end of a parsing
/// context, such as closing delimiters, command terminators, or end-of-input
/// markers. These tokens are crucial for maintaining proper parsing boundaries
/// in LaTeX mathematical expressions.
///
/// # LaTeX/KaTeX Context
/// In LaTeX parsing, certain tokens naturally terminate commands or groups.
/// KaTeX uses these break tokens to ensure correct parsing of nested
/// structures, preventing runaway parsing and ensuring proper error recovery.
///
///
/// # Cross-references
/// - See [`Token`] for general token representation.
/// - Used in `ParseError` for error location reporting.
/// - Related to parsing of groups and commands in the lexer.
#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsRefStr)]
pub enum BreakToken {
    /// Right bracket token `]` - terminates bracket groups.
    #[strum(serialize = "]")]
    RightBracket,
    /// Right brace token `}` - terminates brace groups.
    #[strum(serialize = "}")]
    RightBrace,
    /// End group command `\endgroup` - terminates LaTeX groups.
    #[strum(serialize = "\\endgroup")]
    EndGroup,
    /// Dollar sign `$` - terminates inline math mode.
    #[strum(serialize = "$")]
    Dollar,
    /// Right parenthesis `\)` - terminates display math mode.
    #[strum(serialize = "\\)")]
    RightParen,
    /// Double backslash `\\` - terminates table rows.
    #[strum(serialize = "\\\\")]
    DoubleBackslash,
    /// End command `\end` - terminates environments.
    #[strum(serialize = "\\end")]
    End,
    /// End of file marker - terminates input stream.
    #[strum(serialize = "EOF")]
    Eof,
}

/// Enumeration of font variants used in mathematical expression rendering.
///
/// This enum defines the various font styles available for rendering math
/// symbols and text in KaTeX. Each variant corresponds to a specific typeface
/// or style that can be applied to mathematical content for proper
/// typographical presentation.
///
/// # LaTeX/KaTeX Context
/// LaTeX provides extensive font support for mathematical typesetting,
/// including special symbol fonts (e.g., Blackboard Bold for sets, Fraktur for
/// certain notations). KaTeX implements these variants to ensure accurate
/// reproduction of mathematical documents with correct symbol rendering.
///
/// # Cross-references
/// - See [`CssStyle`] for CSS-based font styling.
/// - Related to [`StyleVariant`] for size and positioning adjustments.
/// - Used in symbol lookup and font metric calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontVariant {
    /// Bold font variant for emphasized mathematical symbols.
    Bold,
    /// Bold italic font variant for bold mathematical variables.
    BoldItalic,
    /// Bold sans-serif font variant for UI elements in math.
    BoldSansSerif,
    /// Double-struck (Blackboard Bold) font variant for sets (e.g., ℝ, ℕ).
    DoubleStruck,
    /// Fraktur font variant for special mathematical notation.
    Fraktur,
    /// Italic font variant for mathematical variables and functions.
    Italic,
    /// Monospace font variant for code and typewriter text in math.
    Monospace,
    /// Normal (roman) font variant for regular mathematical text.
    Normal,
    /// Sans-serif font variant for clean, modern mathematical text.
    SansSerif,
    /// Sans-serif bold italic font variant for emphasized sans-serif text.
    SansSerifBoldItalic,
    /// Sans-serif italic font variant for italic sans-serif text.
    SansSerifItalic,
    /// Script font variant for calligraphic mathematical symbols.
    Script,
}

/// General trait for Spec types, for arguments and properties
pub trait Spec {
    /// Returns the number of arguments required.
    fn num_args(&self) -> usize;
    /// Returns the number of optional arguments.
    fn num_optional_args(&self) -> usize;
    /// Returns the argument types if specified.
    fn arg_types(&self) -> Option<&Vec<ArgType>>;
    /// Returns true if the spec is primitive.
    fn primitive(&self) -> bool;
    /// Returns the node type if specified.
    fn node_type(&self) -> Option<&NodeType>;
}

impl Spec for FunctionSpec {
    fn num_args(&self) -> usize {
        self.num_args
    }
    fn num_optional_args(&self) -> usize {
        self.num_optional_args
    }
    fn arg_types(&self) -> Option<&Vec<ArgType>> {
        self.arg_types.as_ref()
    }
    fn primitive(&self) -> bool {
        self.primitive
    }
    fn node_type(&self) -> Option<&NodeType> {
        self.node_type.as_ref()
    }
}

impl Spec for EnvSpec {
    fn num_args(&self) -> usize {
        self.num_args
    }
    fn num_optional_args(&self) -> usize {
        self.num_optional_args
    }
    fn arg_types(&self) -> Option<&Vec<ArgType>> {
        self.arg_types.as_ref()
    }
    fn primitive(&self) -> bool {
        false
    }
    fn node_type(&self) -> Option<&NodeType> {
        Some(&self.node_type)
    }
}
