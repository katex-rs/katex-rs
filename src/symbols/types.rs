use strum::AsRefStr;
use strum_macros::EnumString;

/// Rendering mode for KaTeX mathematical expressions.
///
/// This enum distinguishes between mathematical and text rendering contexts,
/// which affects how symbols are processed, spaced, and displayed. KaTeX
/// follows LaTeX conventions where math mode uses specialized typography
/// rules, while text mode uses standard text formatting.
///
/// # LaTeX Context
///
/// In LaTeX, math mode is entered with `$...$` for inline math or
/// `\[...\]` for display math, while text mode is the default outside
/// these delimiters.
///
/// # See Also
///
/// - KaTeX documentation on math vs text rendering
/// - LaTeX math mode specifications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Mathematical rendering mode with specialized typography.
    ///
    /// In math mode, symbols follow mathematical typesetting conventions:
    /// operators have specific spacing, variables are italicized, and
    /// special symbols are used for mathematical notation.
    ///
    /// # Examples
    ///
    /// LaTeX: `$a + b = c$`, `\[ \int f(x) dx \]`
    Math,
    /// Standard text rendering mode.
    ///
    /// In text mode, symbols are rendered as regular text without
    /// mathematical spacing or special formatting. This is appropriate
    /// for textual content within or outside mathematical expressions.
    ///
    /// # Examples
    ///
    /// LaTeX: `The value of \text{x} is important.`
    Text,
}

/// Font types for symbols in KaTeX mathematical rendering.
///
/// This enum categorizes the font families used to render mathematical symbols.
/// KaTeX supports multiple font families to ensure comprehensive coverage of
/// mathematical notation, drawing from both standard and specialized symbol
/// sets.
///
/// # See Also
///
/// - [`CharInfo`](struct.CharInfo.html) for how fonts are associated with
///   symbols
/// - KaTeX font documentation for glyph coverage details
#[derive(Debug, PartialEq, Eq, EnumString, Clone)]
#[strum(ascii_case_insensitive)]
pub enum Font {
    /// The primary font family for standard mathematical symbols.
    ///
    /// This font contains the most commonly used mathematical symbols and
    /// operators in LaTeX/KaTeX, including basic operators (+, -, =), Greek
    /// letters, and standard mathematical notation. It serves as the
    /// default font for most symbols.
    ///
    /// # Examples
    ///
    /// Used for symbols like `+`, `α`, `∑` in mathematical expressions.
    Main,
    /// The AMS (American Mathematical Society) font family for extended
    /// symbols.
    ///
    /// This font provides additional mathematical symbols not available in the
    /// main font, particularly those defined by the American Mathematical
    /// Society. It includes specialized symbols for advanced mathematics,
    /// such as additional operators, arrows, and mathematical constructs.
    ///
    /// # Examples
    ///
    /// Used for symbols like `≠`, `⇒`, `∀` when they require AMS-specific
    /// glyphs.
    Ams,
    /// A custom or user-defined font family.
    #[strum(default)]
    Custom(String),
}

/// Atom types for symbols in mathematical expressions.
///
/// Atoms represent the fundamental building blocks of mathematical notation in
/// KaTeX. Each atom type defines how a symbol interacts with surrounding
/// elements in terms of spacing, precedence, and rendering behavior. This
/// categorization follows traditional TeX/LaTeX conventions for mathematical
/// typesetting.
///
/// # See Also
///
/// - [`Group`](enum.Group.html) for grouping atoms and non-atoms
/// - TeXbook Chapter 18 for detailed atom type specifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum Atom {
    /// Binary operators that combine two operands.
    ///
    /// Binary operators require spacing around them and have lower precedence
    /// than relations. Examples include `+`, `-`, `×`, `÷` in mathematical
    /// expressions.
    ///
    /// # Examples
    ///
    /// In LaTeX: `a + b`, `x \times y`
    Bin,
    /// Closing delimiters for mathematical constructs.
    ///
    /// These symbols close paired constructs and may affect spacing of adjacent
    /// elements. Common examples include `)`, `]`, `}`, and various closing
    /// brackets.
    ///
    /// # Examples
    ///
    /// In LaTeX: `(a + b)`, `[x, y]`
    Close,
    /// Inner mathematical constructs or operators.
    ///
    /// Inner atoms represent operators or symbols that appear within
    /// mathematical expressions, often with special spacing rules. Examples
    /// include integrals, sums, and other large operators.
    ///
    /// # Examples
    ///
    /// In LaTeX: `\int`, `\sum`, `\prod`
    Inner,
    /// Opening delimiters for mathematical constructs.
    ///
    /// These symbols begin paired constructs and influence the spacing and
    /// rendering of subsequent elements. Common examples include `(`, `[`, `{`.
    ///
    /// # Examples
    ///
    /// In LaTeX: `(a + b)`, `[x, y]`
    Open,
    /// Punctuation marks in mathematical expressions.
    ///
    /// Punctuation atoms provide structural breaks in mathematical notation,
    /// such as commas, semicolons, and periods used in lists or sequences.
    ///
    /// # Examples
    ///
    /// In LaTeX: `f(x, y)`, `a, b, c`
    Punct,
    /// Relational operators that compare values.
    ///
    /// Relations have higher precedence than binary operators and typically
    /// require spacing around them. Examples include `=`, `<`, `>`, `≠`.
    ///
    /// # Examples
    ///
    /// In LaTeX: `a = b`, `x < y`, `p \neq q`
    Rel,
}

/// Non-atom types for symbols in mathematical and text contexts.
///
/// Non-atoms represent symbols that don't follow the traditional atom spacing
/// rules but are still essential for mathematical typesetting. These include
/// accents, ordinary symbols, operators, and spacing elements that require
/// special handling in KaTeX rendering.
///
/// # See Also
///
/// - [`Atom`](enum.Atom.html) for traditional atom types
/// - [`Group`](enum.Group.html) for combining atom and non-atom types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonAtom {
    /// Accent tokens that modify the appearance of base symbols.
    ///
    /// Accents are placed above or below symbols to indicate mathematical
    /// operations like derivatives, vectors, or special notation. Common
    /// examples include hats (^), bars (_), and dots.
    ///
    /// # Examples
    ///
    /// In LaTeX: `\hat{x}`, `\bar{y}`, `\dot{z}`
    AccentToken,
    /// Ordinary mathematical symbols without special spacing rules.
    ///
    /// Mathord symbols are treated as regular mathematical content that
    /// doesn't require additional spacing. This includes variables, numbers,
    /// and basic symbols that form the core of mathematical expressions.
    ///
    /// # Examples
    ///
    /// In LaTeX: `x`, `y`, `2`, `π`
    MathOrd,
    /// Operator tokens for mathematical operations.
    ///
    /// OpTokens represent operators that may have special rendering or
    /// spacing requirements, such as large operators or custom symbols
    /// that behave like operators.
    ///
    /// # Examples
    ///
    /// In LaTeX: `\lim`, `\log`, custom operator symbols
    OpToken,
    /// Spacing elements that control layout in mathematical expressions.
    ///
    /// Spacing non-atoms are used to fine-tune the horizontal and vertical
    /// spacing between symbols, ensuring proper mathematical typography.
    ///
    /// # Examples
    ///
    /// In LaTeX: `\,`, `\;`, `\quad` for horizontal spacing
    Spacing,
    /// Ordinary text symbols in mathematical contexts.
    ///
    /// Textord symbols are regular text characters used within mathematical
    /// expressions, maintaining their text-like spacing and behavior.
    ///
    /// # Examples
    ///
    /// In LaTeX: `a`, `b`, `c` when used as text in math mode
    TextOrd,
}

/// Group types for symbols, unifying Atom and NonAtom classifications.
///
/// The Group enum provides a unified way to handle both traditional atoms
/// and non-atoms in KaTeX's symbol processing. This allows for consistent
/// handling of spacing, rendering, and interaction rules across different
/// types of mathematical symbols.
///
/// # See Also
///
/// - [`Atom`](enum.Atom.html) for traditional atom types
/// - [`NonAtom`](enum.NonAtom.html) for non-traditional symbol types
/// - [`CharInfo`](struct.CharInfo.html) for complete symbol information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// A traditional atom type with standard spacing rules.
    ///
    /// Atoms follow TeX's conventional spacing and precedence rules.
    /// The wrapped `Atom` specifies the exact type (binary, relation, etc.).
    ///
    /// # Parameters
    ///
    /// * `atom` - The specific atom classification
    Atom(Atom),
    /// A non-atom type with custom spacing or rendering behavior.
    ///
    /// Non-atoms don't follow traditional atom spacing rules and may
    /// require special handling. The wrapped `NonAtom` specifies the type.
    ///
    /// # Parameters
    ///
    /// * `non_atom` - The specific non-atom classification
    NonAtom(NonAtom),
}

/// Character information for symbols in KaTeX mathematical rendering.
///
/// This struct encapsulates all the metadata needed to properly render a
/// mathematical symbol in KaTeX. It combines font information, symbol
/// categorization, and optional replacement strings to ensure accurate
/// typesetting of mathematical notation.
///
/// # See Also
///
/// - [`Font`](enum.Font.html) for font family details
/// - [`Group`](enum.Group.html) for symbol categorization
/// - Symbol lookup tables in `data/symbols.json`
#[derive(Debug, Clone)]
pub struct CharInfo {
    /// The font family used to render this symbol.
    ///
    /// Specifies which font contains the glyph for this symbol. KaTeX supports
    /// multiple font families to ensure comprehensive coverage of mathematical
    /// notation, with Main being the primary font and Ams providing additional
    /// symbols.
    pub font: Font,
    /// The categorization group for this symbol's typesetting behavior.
    ///
    /// Determines how this symbol interacts with surrounding elements in terms
    /// of spacing, precedence, and rendering rules. This follows traditional
    /// TeX conventions for mathematical typography.
    pub group: Group,
    /// Optional replacement string for this symbol.
    ///
    /// When present, this string should be used instead of the original symbol
    /// for rendering. This is useful for symbols that need special handling or
    /// transformation during the rendering process.
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` if a replacement is needed, `None` if the
    /// original symbol should be used directly.
    pub replace: Option<char>,
}
