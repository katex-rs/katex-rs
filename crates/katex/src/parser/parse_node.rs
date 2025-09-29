//! Parse node type definitions for KaTeX AST representation
//!
//! This module contains the Rust equivalents of the JavaScript Flow type
//! definitions for parse nodes, which form the core of KaTeX's Abstract Syntax
//! Tree (AST).

use crate::build_html::DomType;
use crate::spacing_data::MeasurementOwned;
use crate::style::Style;
use crate::symbols::Atom;
use crate::types::{Mode, SourceLocation, Token};

use crate::namespace::KeyMap;
use strum::{AsRefStr, Display, EnumDiscriminants};
use thiserror::Error;

/// Represents the different types of column separation used in array
/// environments within LaTeX/KaTeX mathematical expressions.
///
/// This enum defines the various alignment and separation styles that can be
/// applied to columns in array-like structures, such as matrices or tables in
/// mathematical typesetting. Each variant corresponds to a specific LaTeX array
/// column type and influences how spacing and alignment are handled during
/// rendering.
///
/// # Examples
///
/// In LaTeX source:
/// ```latex
/// \begin{pmatrix} a & b \\ c & d \end{pmatrix}  // Uses Align or Gather
/// \begin{alignat}{2} x &= 1 \\ y &= 2 \end{alignat}  // Uses Alignat
/// ```
///
/// # Usage
///
/// Used internally by the parser when constructing [`ParseNodeArray`] nodes to
/// specify how columns should be separated and aligned. The choice affects
/// spacing calculations and rendering behavior in the final output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColSeparationType {
    /// Standard alignment with default spacing, typically centered columns
    Align,
    /// Alignment with specific left/right positioning, often used for equation
    /// arrays
    Alignat,
    /// Gathering style with centered columns and appropriate spacing
    Gather,
    /// Compact spacing for dense arrays
    Small,
    /// Special handling for commutative diagram environments
    CD,
}

/// Specifies the alignment and spacing properties for individual columns in
/// array environments.
///
/// This enum provides detailed control over how each column in a mathematical
/// array (matrix, table, etc.) should be aligned and spaced. It supports both
/// simple separator-based alignment and more complex alignment specifications
/// with custom gaps.
///
/// # Usage
///
/// Used in [`ParseNodeArray`] to define column properties. The alignment
/// affects how mathematical expressions are positioned within each cell of the
/// array structure.
#[derive(Debug, Clone, PartialEq)]
pub enum AlignSpec {
    /// Simple alignment using a separator string (e.g., "|", ":", etc.)
    Separator {
        /// The string used to separate or align the column (e.g., "|", ":",
        /// etc.)
        separator: String,
    },
    /// Advanced alignment with custom spacing
    Align {
        /// The alignment string (e.g., "c", "l", "r" for center/left/right)
        align: String,
        /// Optional space before the column content (in em units)
        pregap: Option<f64>,
        /// Optional space after the column content (in em units)
        postgap: Option<f64>,
    },
}

/// The core Abstract Syntax Tree (AST) node type for KaTeX mathematical
/// expressions.
///
/// This enum represents all possible parse nodes that can appear in a parsed
/// LaTeX/KaTeX mathematical expression. Each variant corresponds to a specific
/// type of mathematical construct, from simple symbols to complex structures
/// like arrays and fractions.
///
/// The AST is built during parsing and represents the hierarchical structure of
/// mathematical expressions, which is then used for rendering and further
/// processing.
///
/// # Architecture
///
/// The enum uses the `strum` crate for discriminant generation, allowing
/// runtime type checking and serialization. The [`NodeType`] discriminant
/// provides a way to identify node types without pattern matching.
///
/// # Usage
///
/// Parse nodes are typically created by the parser from LaTeX input and then
/// processed by the rendering engine. They can be traversed, transformed, and
/// analyzed using the provided utility functions.
///
/// # See Also
///
/// * [`NodeType`] - The discriminant type for runtime type checking
/// * [`assert_node_type`] - Type assertion utility
/// * [`ParseNode`] - Type alias for this enum
#[derive(Debug, Clone, PartialEq, EnumDiscriminants)]
#[strum_discriminants(vis(pub))]
#[strum_discriminants(doc = "Discriminant type for runtime type checking of parse nodes")]
#[strum_discriminants(derive(Display, Hash, AsRefStr), strum(serialize_all = "lowercase"))]
#[strum_discriminants(name(NodeType))]
pub enum AnyParseNode {
    /// ## Structural Nodes
    /// Array/matrix environments with rows and columns
    Array(ParseNodeArray),
    /// Ordered groups of expressions (parentheses, etc.)
    OrdGroup(ParseNodeOrdGroup),
    /// Superscript/subscript combinations
    SupSub(ParseNodeSupSub),
    /// Generalized fractions (fractions, binomials)
    Genfrac(Box<ParseNodeGenfrac>),
    /// Left-right delimiter pairs
    LeftRight(ParseNodeLeftRight),
    #[strum_discriminants(strum(serialize = "leftright-right"))]
    /// Right delimiters in left-right delimiter pairs (\left...\right).
    LeftRightRight(ParseNodeLeftRightRight),
    /// Square roots and nth roots
    Sqrt(Box<ParseNodeSqrt>),

    /// ## Symbol Nodes
    /// Atomic symbols with specific mathematical meaning
    Atom(ParseNodeAtom),
    /// Ordinary mathematical symbols
    MathOrd(ParseNodeMathOrd),
    /// Mathematical operators
    Op(ParseNodeOp),
    /// Explicit spacing elements
    Spacing(ParseNodeSpacing),

    /// ## Text and Styling
    /// Text content within math
    Text(ParseNodeText),
    /// Style changes (bold, italic, etc.)
    Styling(ParseNodeStyling),
    /// Font family changes
    Font(ParseNodeFont),
    /// Color specifications
    Color(ParseNodeColor),

    /// ## Functions and Commands
    /// Accents over symbols (hats, bars, etc.)
    Accent(Box<ParseNodeAccent>),
    /// Horizontal lines on top of expressions
    Overline(ParseNodeOverline),
    /// Horizontal lines below expressions
    Underline(ParseNodeUnderline),
    /// Invisible content for spacing or alignment
    Phantom(ParseNodePhantom),
    /// Horizontal phantom content for spacing (\hphantom{...}).
    Hphantom(ParseNodeHphantom),
    /// Vertical phantom content for spacing (\vphantom{...}).
    Vphantom(ParseNodeVphantom),
    /// Horizontal/vertical rules
    Rule(ParseNodeRule),

    /// ## Miscellaneous Nodes
    /// Labels for arrows in commutative diagram environments
    /// (\begin{CD}...\end{CD}).
    CdLabel(ParseNodeCdLabel),
    /// Parent containers for CD labels in commutative diagrams.
    CdLabelParent(ParseNodeCdLabelParent),
    #[strum_discriminants(strum(serialize = "color-token"))]
    /// Color tokens for setting current color context (\color{name}).
    ColorToken(ParseNodeColorToken),
    /// Raw content passed through without mathematical processing.
    Raw(ParseNodeRaw),
    /// Size specifications for spacing and dimensions (\rule, spacing
    /// commands).
    Size(ParseNodeSize),
    /// Tagged equations with labels (\tag{label} or automatic numbering).
    Tag(ParseNodeTag),
    /// Hyperlinks in mathematical expressions (\url{...}).
    Url(ParseNodeUrl),
    /// Verbatim text preserving exact formatting (\verb|text| or
    /// \begin{verbatim}...\end{verbatim}).
    Verb(ParseNodeVerb),
    /// Ordinary text symbols in math mode (letters, punctuation).
    TextOrd(ParseNodeTextOrd),
    #[strum_discriminants(strum(serialize = "accent-token"))]
    /// Accent symbols for diacritical marks (\hat, \bar, \tilde, etc.).
    AccentToken(ParseNodeAccentToken),
    #[strum_discriminants(strum(serialize = "op-token"))]
    /// Operator symbols with special positioning (\sum, \int, \lim, etc.).
    OpToken(ParseNodeOpToken),
    #[strum_discriminants(strum(serialize = "accentUnder"))]
    /// Accent marks placed below expressions (\underline, \underbar, etc.).
    AccentUnder(Box<ParseNodeAccentUnder>),
    /// Carriage returns and line breaks (\\, \newline).
    Cr(ParseNodeCr),
    /// Delimiter sizing for proper enclosure (\big, \Big, \bigg, etc.).
    Delimsizing(ParseNodeDelimsizing),
    /// Enclosed expressions with styling (\boxed, \colorbox, etc.).
    Enclose(ParseNodeEnclose),
    /// Custom mathematical environments (\begin{env}...\end{env}).
    Environment(Box<ParseNodeEnvironment>),
    /// Horizontal boxes for grouping content (\hbox{...}).
    Hbox(ParseNodeHbox),
    /// Horizontal braces above/below expressions (\overbrace, \underbrace).
    HorizBrace(ParseNodeHorizBrace),
    /// Hyperlinks with custom text (\href{url}{text}).
    Href(ParseNodeHref),
    /// Embedded HTML content within math expressions.
    Html(ParseNodeHtml),
    /// Content renderable in both HTML and MathML formats.
    HtmlMathMl(ParseNodeHtmlMathMl),
    /// Included graphics/images (\includegraphics{...}).
    Includegraphics(ParseNodeIncludegraphics),
    /// Custom infix operators between operands.
    Infix(ParseNodeInfix),
    /// Internal parser nodes for implementation details.
    Internal(ParseNodeInternal),
    /// Explicit kerning/spacing adjustments (\kern, \mkern).
    Kern(ParseNodeKern),
    /// Overlapping content for annotations (\rlap, \llap, \clap).
    Lap(ParseNodeLap),
    /// Different renderings for display/text/script modes
    /// (\mathchoice{...}{...}{...}{...}).
    MathChoice(ParseNodeMathChoice),
    /// Middle delimiters in expressions (\middle|).
    Middle(ParseNodeMiddle),
    /// Math class specifications for spacing and rendering (mord, mbin, mrel,
    /// etc.).
    Mclass(ParseNodeMclass),
    /// Operator names with special formatting (\operatorname{...}).
    OperatorName(ParseNodeOperatorName),
    /// Poor man's bold text formatting (\pmb{...}).
    Pmb(ParseNodePmb),
    /// Raised or lowered content (\raisebox{...}{...}).
    Raisebox(ParseNodeRaisebox),
    /// Size changes for expressions (\scriptsize, \large, etc.).
    Sizing(ParseNodeSizing),
    /// Smashed content ignoring height/depth (\smash{...}).
    Smash(ParseNodeSmash),
    /// Vertically centered content (\vcenter{...}).
    Vcenter(ParseNodeVcenter),
    #[strum_discriminants(strum(serialize = "x-arrow"))]
    /// Extensible arrows with labels (\xleftarrow{...}, \xrightarrow{...}).
    XArrow(ParseNodeXArrow),
}

impl AnyParseNode {
    /// TeXbook algorithms often reference "character boxes", which are simply
    /// groups with a single character in them. To decide if something is a
    /// character box, we find its innermost group, and see if it is a
    /// single character.
    ///
    /// This is equivalent to `utils.isCharacterBox` in the JS codebase.
    pub fn is_character_box(&self) -> Result<bool, ParseNodeError> {
        let base_elem = self.to_base_elem()?;
        Ok(matches!(
            base_elem,
            Self::MathOrd { .. } | Self::TextOrd { .. } | Self::Atom { .. }
        ))
    }

    /// Sometimes we want to pull out the innermost element of a group. In most
    /// cases, this will just be the group itself, but when ordgroups and colors
    /// have a single element, we want to pull that out.
    ///
    /// This is equivalent to `utils.getBaseElem` in the JS codebase.
    pub fn to_base_elem(&self) -> Result<&Self, ParseNodeError> {
        match self {
            Self::OrdGroup(ord) => {
                if ord.body.len() == 1 {
                    ord.body[0].to_base_elem()
                } else {
                    Ok(self)
                }
            }
            Self::Color(color) => {
                if color.body.len() == 1 {
                    color.body[0].to_base_elem()
                } else {
                    Ok(self)
                }
            }
            Self::Font(font) => font.body.to_base_elem(),
            _ => Ok(self),
        }
    }

    /// Get the mode of current node
    #[must_use]
    pub fn mode(&self) -> Mode {
        match self {
            Self::Array(node) => node.mode,
            Self::OrdGroup(node) => node.mode,
            Self::SupSub(node) => node.mode,
            Self::Genfrac(node) => node.mode,
            Self::LeftRight(node) => node.mode,
            Self::LeftRightRight(node) => node.mode,
            Self::Sqrt(node) => node.mode,
            Self::Atom(node) => node.mode,
            Self::MathOrd(node) => node.mode,
            Self::Op(op) => match op {
                ParseNodeOp::Symbol { mode, .. } | ParseNodeOp::Body { mode, .. } => *mode,
            },
            Self::Spacing(node) => node.mode,
            Self::Text(node) => node.mode,
            Self::Styling(node) => node.mode,
            Self::Font(node) => node.mode,
            Self::Color(node) => node.mode,
            Self::Accent(node) => node.mode,
            Self::Overline(node) => node.mode,
            Self::Underline(node) => node.mode,
            Self::Phantom(node) => node.mode,
            Self::Hphantom(node) => node.mode,
            Self::Vphantom(node) => node.mode,
            Self::Rule(node) => node.mode,
            Self::CdLabel(node) => node.mode,
            Self::CdLabelParent(node) => node.mode,
            Self::ColorToken(node) => node.mode,
            Self::Raw(node) => node.mode,
            Self::Size(node) => node.mode,
            Self::Tag(node) => node.mode,
            Self::Url(node) => node.mode,
            Self::Verb(node) => node.mode,
            Self::TextOrd(node) => node.mode,
            Self::AccentToken(node) => node.mode,
            Self::OpToken(node) => node.mode,
            Self::AccentUnder(node) => node.mode,
            Self::Cr(node) => node.mode,
            Self::Delimsizing(node) => node.mode,
            Self::Enclose(node) => node.mode,
            Self::Environment(node) => node.mode,
            Self::Hbox(node) => node.mode,
            Self::HorizBrace(node) => node.mode,
            Self::Href(node) => node.mode,
            Self::Html(node) => node.mode,
            Self::HtmlMathMl(node) => node.mode,
            Self::Includegraphics(node) => node.mode,
            Self::Infix(node) => node.mode,
            Self::Internal(node) => node.mode,
            Self::Kern(node) => node.mode,
            Self::Lap(node) => node.mode,
            Self::MathChoice(node) => node.mode,
            Self::Middle(node) => node.mode,
            Self::Mclass(node) => node.mode,
            Self::OperatorName(node) => node.mode,
            Self::Pmb(node) => node.mode,
            Self::Raisebox(node) => node.mode,
            Self::Sizing(node) => node.mode,
            Self::Smash(node) => node.mode,
            Self::Vcenter(node) => node.mode,
            Self::XArrow(node) => node.mode,
        }
    }

    /// Get the text field of the node
    ///
    /// `checkSymbolNodeType` checks if the node is a symbol and returns its
    /// text later in Javascript.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            // Atom
            Self::Atom(node) => Some(&node.text),
            // Non-Atom
            Self::AccentToken(node) => Some(&node.text),
            Self::MathOrd(node) => Some(&node.text),
            Self::OpToken(node) => Some(&node.text),
            Self::Spacing(node) => Some(&node.text),
            Self::TextOrd(node) => Some(&node.text),
            _ => None,
        }
    }

    /// Get the label field of the node
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Accent(acc) => Some(&acc.label),
            Self::AccentUnder(acc_under) => Some(&acc_under.label),
            Self::HorizBrace(hb) => Some(&hb.label),
            Self::XArrow(xa) => Some(&xa.label),
            Self::Enclose(enclose) => Some(&enclose.label),
            _ => None,
        }
    }
}

/// The array item of tags in array
#[derive(Debug, Clone, PartialEq)]
pub enum ParseNodeArrayTag {
    /// A boolean value indicating if the tag is active
    Bool(bool),
    /// A list of nodes associated with the tag
    Nodes(Vec<AnyParseNode>),
}

impl From<Vec<AnyParseNode>> for ParseNodeArrayTag {
    fn from(nodes: Vec<AnyParseNode>) -> Self {
        Self::Nodes(nodes)
    }
}

impl From<bool> for ParseNodeArrayTag {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl ParseNodeArrayTag {
    /// If is true or has nodes
    #[must_use]
    pub const fn is_true(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            // In Javascript, `[]` is true
            Self::Nodes(_) => true,
        }
    }
}

/// Represents array and matrix environments in mathematical expressions.
///
/// This struct handles the parsing and representation of array-like structures
/// such as matrices, tables, and aligned equations in LaTeX/KaTeX. It supports
/// various column alignments, spacing, and formatting options.
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX environments like:
/// ```latex
/// \begin{pmatrix} a & b \\ c & d \end{pmatrix}  % Matrix
/// \begin{array}{cc} a & b \\ c & d \end{array}  % General array
/// \begin{align} x &= 1 \\ y &= 2 \end{align}    % Aligned equations
/// ```
///
/// # Usage
///
/// Arrays are fundamental for representing matrices, systems of equations, and
/// tabular data in mathematical typesetting. The parser creates these nodes
/// when encountering array environments.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeArray {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// Type of column separation ([`ColSeparationType`])
    pub col_separation_type: Option<ColSeparationType>,
    /// Whether to add horizontal skip before/after the array
    pub hskip_before_and_after: Option<bool>,
    /// Whether to add extra vertical spacing between rows
    pub add_jot: Option<bool>,
    /// Column alignment specifications ([`AlignSpec`])
    pub cols: Option<Vec<AlignSpec>>,
    /// Vertical stretching factor for the array
    pub arraystretch: f64,
    /// The array content as a vector of rows, each containing cells
    pub body: Vec<Vec<AnyParseNode>>,
    /// Vertical gaps between rows
    pub row_gaps: Vec<Option<MeasurementOwned>>,
    /// Horizontal lines to draw before each row
    pub h_lines_before_row: Vec<Vec<bool>>,
    /// Optional equation tags/numbers for each row
    pub tags: Option<Vec<ParseNodeArrayTag>>,
    /// Whether to place equation numbers on the left
    pub leqno: Option<bool>,
    /// Whether this is a commutative diagram array
    pub is_cd: Option<bool>,
}

/// Represents labels in commutative diagram (CD) environments.
///
/// This struct handles labels attached to arrows or objects in commutative
/// diagrams, which are specialized mathematical diagrams showing relationships
/// between objects.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `side` - The side of the arrow/object where the label appears ("top",
///   "bottom", "left", "right")
/// * `label` - The mathematical expression serving as the label
///
/// # LaTeX Correspondence
///
/// Used in CD environments:
/// ```latex
/// \begin{CD}
/// A @>f>> B
/// @VVgV @VVhV
/// C @>>k> D
/// \end{CD}
/// ```
///
/// # Usage
///
/// CD labels provide annotations for arrows and objects in commutative
/// diagrams, helping to clarify the mathematical relationships being depicted.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeCdLabel {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The side of the arrow/object where the label appears ("top", "bottom",
    /// "left", "right")
    pub side: String,
    /// The mathematical expression serving as the label
    pub label: Box<AnyParseNode>,
}

/// Represents parent containers for CD labels in commutative diagrams.
///
/// This struct serves as a wrapper for fragments that contain CD labels,
/// maintaining the hierarchical structure of commutative diagram elements.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `fragment` - The contained mathematical expression or diagram fragment
///
/// # Usage
///
/// Used internally to maintain proper nesting and structure in commutative
/// diagram parsing. The fragment typically contains the arrow or object being
/// labeled.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeCdLabelParent {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The contained mathematical expression or diagram fragment
    pub fragment: Box<AnyParseNode>,
}

/// Represents color changes applied to mathematical expressions.
///
/// This struct wraps mathematical content with color specifications, allowing
/// different parts of expressions to be rendered in different colors.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `color` - The color specification (name, hex code, or RGB values)
/// * `body` - The mathematical expressions to be colored
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX color commands:
/// ```latex
/// \color{red}{x + y}
/// {\color{blue} \frac{a}{b}}
/// ```
///
/// # Usage
///
/// Color nodes allow visual distinction of different parts of mathematical
/// expressions, useful for highlighting important terms or distinguishing
/// different variables.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeColor {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The color specification (name, hex code, or RGB values)
    pub color: String,
    /// The mathematical expressions to be colored
    pub body: Vec<AnyParseNode>,
}

impl From<ParseNodeColor> for AnyParseNode {
    fn from(node: ParseNodeColor) -> Self {
        Self::Color(node)
    }
}

/// Represents color tokens in mathematical expressions.
///
/// This struct handles standalone color specifications that don't wrap content,
/// typically used for setting the current color context.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `color` - The color specification string
///
/// # Usage
///
/// Used for color commands that set the current color without immediately
/// applying it to content, allowing subsequent expressions to inherit the
/// color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeColorToken {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The color specification string
    pub color: String,
}

/// Represents mathematical operators in KaTeX expressions, supporting both
/// symbolic and compound operators.
///
/// This enum handles operators that can appear in mathematical expressions,
/// including those with limits (subscripts/superscripts above/below) and those
/// that contain other expressions as their body.
///
/// # Usage
///
/// Used for operators like `\sum`, `\int`, `\lim`, etc. The `limits` field
/// controls whether sub/super scripts appear as limits (above/below) or as
/// regular scripts (right side).
///
/// # LaTeX Examples
///
/// ```latex
/// \sum_{i=1}^{n} x_i     % limits = true
/// \int_a^b f(x) dx      % limits = false (default)
/// \lim_{x \to 0} f(x)   % limits = true
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ParseNodeOp {
    /// A simple operator symbol with optional limit positioning
    Symbol {
        /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
        mode: Mode,
        /// Optional source location for error reporting
        loc: Option<SourceLocation>,
        /// Whether limits should be displayed above/below (true) or as
        /// sub/super scripts (false)
        limits: bool,
        /// Force special handling of adjacent superscripts/subscripts
        always_handle_sup_sub: Option<bool>,
        /// Suppress vertical shifting of the base symbol
        suppress_base_shift: Option<bool>,
        /// Whether this operator is part of a sup/sub script context
        parent_is_sup_sub: bool,
        /// The operator name/symbol (e.g., "sum", "lim", "int")
        name: String,
        /// Whether it is a symbol
        symbol: bool,
    },
    /// An operator that contains other expressions as its body
    Body {
        /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
        mode: Mode,
        /// Optional source location for error reporting
        loc: Option<SourceLocation>,
        /// Whether limits should be displayed above/below (true) or as
        /// sub/super scripts (false)
        limits: bool,
        /// Force special handling of adjacent superscripts/subscripts
        always_handle_sup_sub: Option<bool>,
        /// Suppress vertical shifting of the base symbol
        suppress_base_shift: Option<bool>,
        /// Whether this operator is part of a sup/sub script context
        parent_is_sup_sub: bool,
        /// The expressions that form the operator's content
        body: Vec<AnyParseNode>,
    },
}

impl ParseNodeOp {
    /// Get the limits field
    #[must_use]
    pub const fn limits(&self) -> bool {
        match self {
            Self::Symbol { limits, .. } | Self::Body { limits, .. } => *limits,
        }
    }

    /// Mutate the limits field
    #[must_use]
    pub const fn limits_mut(&mut self) -> &mut bool {
        match self {
            Self::Symbol { limits, .. } | Self::Body { limits, .. } => limits,
        }
    }

    /// Get the always_handle_sup_sub field
    #[must_use]
    pub fn always_handle_sup_sub(&self) -> bool {
        let field = match self {
            Self::Symbol {
                always_handle_sup_sub,
                ..
            }
            | Self::Body {
                always_handle_sup_sub,
                ..
            } => *always_handle_sup_sub,
        };
        field.unwrap_or(false)
    }

    /// Mutate the always_handle_sup_sub field
    #[must_use]
    pub const fn always_handle_sup_sub_mut(&mut self) -> &mut Option<bool> {
        match self {
            Self::Symbol {
                always_handle_sup_sub,
                ..
            }
            | Self::Body {
                always_handle_sup_sub,
                ..
            } => always_handle_sup_sub,
        }
    }

    /// Get the name field
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Symbol { name, .. } => Some(name),
            Self::Body { .. } => None,
        }
    }
}

/// Represents ordered groups of mathematical expressions, typically enclosed in
/// parentheses or brackets.
///
/// This struct handles grouped expressions that should be treated as a single
/// unit, such as parenthesized subexpressions or bracketed terms.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The expressions contained within the group
/// * `semisimple` - Whether this is a simple grouping (affects spacing and
///   rendering)
///
/// # LaTeX Correspondence
///
/// Corresponds to grouped expressions:
/// ```latex
/// (x + y)    % Parentheses
/// {a + b}    % Braces
/// [c + d]    % Brackets
/// ```
///
/// # Usage
///
/// OrdGroups ensure proper precedence and grouping in mathematical expressions,
/// affecting both parsing order and visual rendering with appropriate
/// delimiters.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeOrdGroup {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The expressions contained within the group
    pub body: Vec<AnyParseNode>,
    /// Whether this is a simple grouping (affects spacing and rendering)
    pub semisimple: Option<bool>,
}

/// Represents raw, unprocessed content in mathematical expressions.
///
/// This struct contains literal text or content that should be passed through
/// without further mathematical processing or rendering.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `string` - The raw string content
///
/// # Usage
///
/// Used for content that needs to be preserved exactly as-is, such as
/// literal text within math mode or special formatting commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeRaw {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The raw string content
    pub string: String,
}

/// Represents size specifications for spacing or dimensions in mathematical
/// expressions.
///
/// This struct handles explicit size measurements used for spacing, rules, or
/// other dimensional elements in mathematical typesetting.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `value` - The size measurement ([`MeasurementOwned`])
/// * `is_blank` - Whether this represents a blank/zero size
///
/// # Usage
///
/// Size nodes control the dimensions of various elements like rules, spacing,
/// and other measurable components in mathematical layouts.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeSize {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The size measurement ([`MeasurementOwned`])
    pub value: MeasurementOwned,
    /// Whether this represents a blank/zero size
    pub is_blank: bool,
}

/// Represents style changes applied to mathematical expressions.
///
/// This struct wraps mathematical content with style specifications like bold,
/// italic, or other text styling attributes.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `style` - The style specification ([`Style`])
/// * `body` - The expressions to be styled
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX styling commands:
/// ```latex
/// \mathbf{x}    % Bold
/// \mathit{f}    % Italic
/// \mathrm{dx}   % Roman/upright
/// ```
///
/// # Usage
///
/// Styling nodes allow different visual appearances for mathematical symbols,
/// useful for distinguishing vectors from scalars or emphasizing certain terms.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeStyling {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The style specification ([`Style`])
    pub style: &'static Style,
    /// The expressions to be styled
    pub body: Vec<AnyParseNode>,
}

/// Represents superscript and subscript combinations in mathematical
/// expressions.
///
/// This struct handles the attachment of superscript and/or subscript
/// expressions to a base expression, supporting complex nested structures.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `base` - The base expression being modified
/// * `sup` - Optional superscript expression
/// * `sub` - Optional subscript expression
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX superscript/subscript syntax:
/// ```latex
/// x^2        % Superscript
/// a_i        % Subscript
/// x^{n+1}    % Complex superscript
/// a_{i,j}    % Complex subscript
/// ```
///
/// # Usage
///
/// SupSub nodes are fundamental for mathematical notation, allowing expressions
/// like exponents, indices, and other modifiers to be properly positioned
/// relative to their base.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeSupSub {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The base expression being modified
    pub base: Option<Box<AnyParseNode>>,
    /// Optional superscript expression
    pub sup: Option<Box<AnyParseNode>>,
    /// Optional subscript expression
    pub sub: Option<Box<AnyParseNode>>,
}

/// Represents tagged equations or expressions with labels.
///
/// This struct handles equation tags, numbers, or labels that are associated
/// with mathematical expressions, typically for referencing.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The main mathematical expression
/// * `tag` - The tag/label expressions
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX equation tagging:
/// ```latex
/// \tag{1} E = mc^2
/// \begin{equation}\label{eq:energy}
/// E = mc^2
/// \end{equation}
/// ```
///
/// # Usage
///
/// Tag nodes enable equation numbering and referencing in mathematical
/// documents, allowing readers to cite specific equations.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeTag {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The main mathematical expression
    pub body: Vec<AnyParseNode>,
    /// The tag/label expressions
    pub tag: Vec<AnyParseNode>,
}

/// Represents text content within mathematical expressions.
///
/// This struct handles text that appears in math mode, with optional font
/// specifications for proper rendering of textual elements in mathematical
/// contexts.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The text content as parse nodes
/// * `font` - Optional font family specification
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX text commands in math:
/// ```latex
/// \text{if}        % Text in math
/// \mathrm{dx}      % Roman font
/// \mathbf{v}       % Bold font
/// ```
///
/// # Usage
///
/// Text nodes allow mixing of textual and mathematical content, ensuring
/// proper font rendering for words and phrases within mathematical expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeText {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The text content as parse nodes
    pub body: Vec<AnyParseNode>,
    /// Optional font family specification
    pub font: Option<String>,
}

/// Represents URL links in mathematical expressions.
///
/// This struct handles hyperlinks within mathematical content, allowing
/// mathematical expressions to contain clickable links.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `url` - The URL string
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX hyperlink commands:
/// ```latex
/// \url{https://example.com}
/// \href{https://example.com}{link text}
/// ```
///
/// # Usage
///
/// URL nodes enable interactive mathematical documents with embedded links,
/// useful for referencing external resources or documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeUrl {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The URL string
    pub url: String,
}

/// Represents verbatim text content that should be rendered exactly as written.
///
/// This struct handles text that must preserve its exact formatting and
/// spacing, without any mathematical interpretation or processing.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The verbatim text content
/// * `star` - Whether this is a starred version (affects rendering)
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX verbatim commands:
/// ```latex
/// \verb|exact text|
/// \begin{verbatim}
/// exact text
/// \end{verbatim}
/// ```
///
/// # Usage
///
/// Verb nodes are essential for including code snippets, file paths, or other
/// text that must maintain exact spacing and special characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeVerb {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The verbatim text content
    pub body: String,
    /// Whether this is a starred version (affects rendering)
    pub star: bool,
}

// Symbol parse nodes (from symbols.js)

/// Represents atomic symbols with specific mathematical meaning and spacing
/// rules.
///
/// This struct handles individual symbols that have special significance in
/// mathematical typesetting, such as operators, relations, and other atomic
/// elements with defined spacing behavior.
///
/// # Fields
///
/// * `family` - The atom type ([`Atom`]) determining spacing and behavior
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `text` - The symbol text
///
/// # Usage
///
/// Atoms are the fundamental building blocks of mathematical expressions,
/// with each atom type having specific spacing rules that affect layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeAtom {
    /// The atom type ([`Atom`]) determining spacing and behavior
    pub family: Atom,
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The symbol text
    pub text: String,
}

/// Represents ordinary mathematical symbols without special spacing rules.
///
/// This struct handles regular mathematical symbols that don't have special
/// spacing or positioning requirements, such as variables and digits.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `text` - The symbol text
///
///
/// # LaTeX Correspondence
///
/// Corresponds to ordinary symbols in math mode:
/// ```latex
/// x y z     % Variables
/// 1 2 3     % Digits
/// a b c     % Letters
/// ```
///
/// # Usage
///
/// MathOrd nodes represent the most common type of mathematical symbols,
/// forming the basic content of expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeMathOrd {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The symbol text
    pub text: String,
}

/// Represents explicit spacing elements in mathematical expressions.
///
/// This struct handles manually inserted spacing that affects the layout
/// and positioning of mathematical elements.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `text` - The spacing command or symbol
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX spacing commands:
/// ```latex
/// \,    % Thin space
/// \:    % Medium space
/// \;    % Thick space
/// \!    % Negative thin space
/// ```
///
/// # Usage
///
/// Spacing nodes allow fine control over the horizontal spacing between
/// mathematical elements for proper visual appearance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeSpacing {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The spacing command or symbol
    pub text: String,
}

/// Represents ordinary text symbols within mathematical expressions.
///
/// This struct handles text characters that appear in math mode but don't
/// have mathematical symbol properties.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `text` - The text content
///
/// # Usage
///
/// TextOrd nodes are used for text characters in mathematical contexts,
/// ensuring proper font and spacing treatment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeTextOrd {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The text content
    pub text: String,
}

/// Represents accent symbols used for diacritical marks.
///
/// This struct handles the symbol portion of accent commands,
/// which modify the appearance of base characters.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `text` - The accent symbol
///
/// # LaTeX Correspondence
///
/// Corresponds to accent symbols:
/// ```latex
/// \hat{x}     % ^
/// \bar{y}     % -
/// \tilde{z}   % ~
/// ```
///
/// # Usage
///
/// Accent tokens are combined with base characters to create accented symbols
/// in mathematical notation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeAccentToken {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The accent symbol
    pub text: String,
}

/// Represents operator symbols with special positioning rules.
///
/// This struct handles operator tokens that may have special rendering
/// requirements, such as limits positioning.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `text` - The operator symbol
///
///
/// # Usage
///
/// OpTokens represent operators that may need special handling for
/// subscripts/superscripts (limits vs. regular scripts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeOpToken {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The operator symbol
    pub text: String,
}

// Function parse nodes (from functions/*.js)

/// Represents accent marks placed above mathematical expressions.
///
/// This struct handles diacritical marks and accents that modify the appearance
/// of base mathematical expressions, such as hats, bars, and tildes.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `label` - The accent symbol (e.g., "^", "-", "~")
/// * `is_stretchy` - Whether the accent stretches to fit the base width
/// * `is_shifty` - Whether the accent is shifted for better positioning
/// * `base` - The expression being accented
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX accent commands:
/// ```latex
/// \hat{x}      % Hat
/// \bar{y}      % Bar
/// \tilde{z}    % Tilde
/// \widehat{abc} % Wide hat
/// ```
///
/// # Usage
///
/// Accent nodes modify the visual appearance of mathematical symbols,
/// commonly used for vectors, complex numbers, and special notation.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeAccent {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The accent symbol (e.g., "^", "-", "~")
    pub label: String,
    /// Whether the accent stretches to fit the base width
    pub is_stretchy: Option<bool>,
    /// Whether the accent is shifted for better positioning
    pub is_shifty: Option<bool>,
    /// The expression being accented
    pub base: AnyParseNode,
}

/// Represents accent marks placed below mathematical expressions.
///
/// This struct handles under-accents, which are diacritical marks positioned
/// beneath base expressions, such as underbars and undertildes.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `label` - The under-accent symbol
/// * `is_stretchy` - Whether the accent stretches to fit the base width
/// * `is_shifty` - Whether the accent is shifted for better positioning
/// * `base` - The expression being under-accented
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX under-accent commands:
/// ```latex
/// \underline{x}    % Underline
/// \underbar{y}     % Underbar
/// \utilde{z}       % Undertilde
/// ```
///
/// # Usage
///
/// Under-accent nodes provide visual modifications below expressions,
/// useful for emphasis and special mathematical notation.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeAccentUnder {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The under-accent symbol
    pub label: String,
    /// Whether the accent stretches to fit the base width
    pub is_stretchy: Option<bool>,
    /// Whether the accent is shifted for better positioning
    pub is_shifty: Option<bool>,
    /// The expression being under-accented
    pub base: Box<AnyParseNode>,
}

/// Represents carriage returns and line breaks in mathematical expressions.
///
/// This struct handles explicit line breaks and vertical spacing within
/// mathematical content, particularly in arrays and multiline constructs.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `new_line` - Whether this represents a new line
/// * `size` - Optional size of the break ([`MeasurementOwned`])
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX line break commands:
/// ```latex
/// \\          % Basic line break
/// \\[1em]     % Line break with spacing
/// \newline    % New line
/// ```
///
/// # Usage
///
/// CR nodes control line breaks in multiline mathematical expressions,
/// such as in arrays, cases, and aligned equations.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeCr {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// Whether this represents a new line
    pub new_line: bool,
    /// Optional size of the break ([`MeasurementOwned`])
    pub size: Option<MeasurementOwned>,
}

/// Represents delimiter sizing for mathematical symbols.
///
/// This struct handles the sizing of delimiters (parentheses, brackets, etc.)
/// to properly enclose mathematical expressions of varying heights.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `size` - The size level (1-4, where 4 is largest)
/// * `mclass` - The math class ("mopen", "mclose", "mrel", "mord")
/// * `delim` - The delimiter symbol
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX delimiter sizing:
/// ```latex
/// \big(       % Size 2
/// \Big[       % Size 3
/// \bigg\{     % Size 4
/// \Bigg\langle % Size 5
/// ```
///
/// # Usage
///
/// Delimsizing nodes ensure delimiters scale appropriately to contain
/// their enclosed mathematical content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeDelimsizing {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The size level (1-4, where 4 is largest)
    pub size: u8, // 1 | 2 | 3 | 4
    /// The math class ("mopen", "mclose", "mrel", "mord")
    pub mclass: DomType, // "mopen" | "mclose" | "mrel" | "mord"
    /// The delimiter symbol
    pub delim: String,
}

/// Represents enclosed mathematical expressions with background and border
/// styling.
///
/// This struct handles expressions that are enclosed with colored backgrounds
/// or borders, often used for highlighting or special notation.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `label` - The enclosure type/label
/// * `background_color` - Optional background color
/// * `border_color` - Optional border color
/// * `body` - The enclosed expression
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX enclosure commands:
/// ```latex
/// \boxed{x}              % Boxed
/// \colorbox{yellow}{x}   % Background color
/// \fcolorbox{red}{blue}{x} % Border and background
/// ```
///
/// # Usage
///
/// Enclose nodes provide visual emphasis and grouping for mathematical
/// expressions, useful for highlighting important terms or creating visual
/// distinctions.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeEnclose {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The enclosure type/label
    pub label: String,
    /// Optional background color
    pub background_color: Option<String>,
    /// Optional border color
    pub border_color: Option<String>,
    /// The enclosed expression
    pub body: Box<AnyParseNode>,
}

/// Represents custom mathematical environments.
///
/// This struct handles user-defined or custom mathematical environments
/// that extend beyond the standard KaTeX environments.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `name` - The environment name
/// * `name_group` - The parsed name as an expression
///
///
/// # Usage
///
/// Environment nodes support custom mathematical constructs and
/// user-defined environments in LaTeX documents.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeEnvironment {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The environment name
    pub name: String,
    /// The parsed name as an expression
    pub name_group: Box<AnyParseNode>,
}

impl From<ParseNodeEnvironment> for AnyParseNode {
    fn from(value: ParseNodeEnvironment) -> Self {
        Self::Environment(Box::new(value))
    }
}

/// Represents font changes applied to mathematical expressions.
///
/// This struct handles font family changes for mathematical content,
/// allowing different typefaces within expressions.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `font` - The font family name
/// * `body` - The expression in the specified font
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX font commands:
/// ```latex
/// \mathrm{dx}     % Roman/upright
/// \mathbf{v}     % Bold
/// \mathit{f}     % Italic
/// \mathsf{S}     % Sans-serif
/// ```
///
/// # Usage
///
/// Font nodes control the typeface of mathematical symbols, essential for
/// distinguishing different types of mathematical objects (scalars, vectors,
/// etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeFont {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The font family name
    pub font: String,
    /// The expression in the specified font
    pub body: Box<AnyParseNode>,
}

/// Represents generalized fractions and binomial coefficients.
///
/// This struct handles various types of fractions including regular fractions,
/// binomial coefficients, and other stacked expressions with optional
/// delimiters.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `continued` - Whether this is a continued fraction
/// * `numer` - The numerator expression
/// * `denom` - The denominator expression
/// * `has_bar_line` - Whether to draw a fraction bar
/// * `left_delim` - Optional left delimiter
/// * `right_delim` - Optional right delimiter
/// * `size` - The display size ([`Style`])
/// * `bar_size` - Optional custom bar thickness
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX fraction commands:
/// ```latex
/// \frac{1}{2}       % Regular fraction
/// \binom{n}{k}      % Binomial coefficient
/// \dfrac{a}{b}      % Display style fraction
/// \cfrac{1}{1+x}    % Continued fraction
/// ```
///
/// # Usage
///
/// Genfrac nodes are fundamental for representing ratios, probabilities,
/// and complex mathematical relationships in stacked form.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeGenfrac {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// Whether this is a continued fraction
    pub continued: bool,
    /// The numerator expression
    pub numer: Box<AnyParseNode>,
    /// The denominator expression
    pub denom: Box<AnyParseNode>,
    /// Whether to draw a fraction bar
    pub has_bar_line: bool,
    /// Optional left delimiter
    pub left_delim: Option<String>,
    /// Optional right delimiter
    pub right_delim: Option<String>,
    /// The display size ([`Style`])
    /// Corresponds to KaTeX's `\genfrac` size parameter:
    /// If this is None, the equivalent to "auto" is used.
    pub size: Option<&'static Style>,
    /// Optional custom bar thickness
    pub bar_size: Option<MeasurementOwned>,
}

/// Represents horizontal boxes for grouping mathematical content.
///
/// This struct handles horizontal grouping of mathematical expressions
/// that should be treated as a single unit for layout purposes.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The expressions contained in the horizontal box
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX horizontal box commands:
/// ```latex
/// \hbox{ab}     % Horizontal box
/// \mbox{xy}    % Math box
/// ```
///
/// # Usage
///
/// Hbox nodes control horizontal spacing and grouping of mathematical elements,
/// ensuring they are treated as a single layout unit.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeHbox {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The expressions contained in the horizontal box
    pub body: Vec<AnyParseNode>,
}

/// Represents horizontal braces above or below mathematical expressions.
///
/// This struct handles overbraces and underbraces that span multiple symbols
/// with optional labels for annotation.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `label` - The brace label/annotation
/// * `is_over` - Whether the brace is above (true) or below (false)
/// * `base` - The expression being braced
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX brace commands:
/// ```latex
/// \overbrace{abc}^{sum}    % Overbrace
/// \underbrace{xyz}_{total} % Underbrace
/// ```
///
/// # Usage
///
/// HorizBrace nodes provide visual grouping and annotation of related
/// mathematical terms or operations.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeHorizBrace {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The brace label/annotation
    pub label: String,
    /// Whether the brace is above (true) or below (false)
    pub is_over: bool,
    /// The expression being braced
    pub base: Box<AnyParseNode>,
}

/// Represents hyperlink references within mathematical expressions.
///
/// This struct handles clickable links in mathematical content,
/// allowing expressions to link to external resources or references.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `href` - The hyperlink URL
/// * `body` - The linked mathematical expression
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX hyperlink commands:
/// ```latex
/// \href{https://example.com}{x}
/// \url{https://example.com}
/// ```
///
/// # Usage
///
/// Href nodes enable interactive mathematical documents with embedded
/// navigation and reference links.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeHref {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The hyperlink URL
    pub href: String,
    /// The linked mathematical expression
    pub body: Vec<AnyParseNode>,
}

/// Represents embedded HTML content within mathematical expressions.
///
/// This struct handles raw HTML that needs to be included in the
/// rendered mathematical output.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `attributes` - HTML attributes as key-value pairs
/// * `body` - The HTML content as parse nodes
///
/// # Usage
///
/// HTML nodes allow embedding custom HTML elements within mathematical
/// expressions for advanced formatting or interactivity.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeHtml {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// HTML attributes as key-value pairs
    pub attributes: KeyMap<String, String>,
    /// The HTML content as parse nodes
    pub body: Vec<AnyParseNode>,
}

/// Represents content that can be rendered in both HTML and MathML formats.
///
/// This struct handles mathematical expressions that have different
/// representations for HTML and MathML output formats.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `html` - HTML representation
/// * `mathml` - MathML representation
///
/// # Usage
///
/// HtmlMathMl nodes support dual-format output for better compatibility
/// across different rendering engines and accessibility tools.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeHtmlMathMl {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// HTML representation
    pub html: Vec<AnyParseNode>,
    /// MathML representation
    pub mathml: Vec<AnyParseNode>,
}

/// Represents included graphics/images within mathematical expressions.
///
/// This struct handles the inclusion of external images or graphics
/// that are part of mathematical content.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `alt` - Alternative text for accessibility
/// * `width` - Image width ([`MeasurementOwned`])
/// * `height` - Image height ([`MeasurementOwned`])
/// * `total_height` - Total height including depth
/// * `src` - Image source path/URL
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX include graphics commands:
/// ```latex
/// \includegraphics[width=5em]{diagram.png}
/// \includegraphics[height=3em]{figure.jpg}
/// ```
///
/// # Usage
///
/// Includegraphics nodes allow embedding diagrams, plots, and other
/// visual elements within mathematical expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeIncludegraphics {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// Alternative text for accessibility
    pub alt: String,
    /// Image width ([`MeasurementOwned`])
    pub width: MeasurementOwned,
    /// Image height ([`MeasurementOwned`])
    pub height: MeasurementOwned,
    /// Total height including depth
    pub total_height: MeasurementOwned,
    /// Image source path/URL
    pub src: String,
}

/// Represents infix operators in mathematical expressions.
///
/// This struct handles operators that appear between operands,
/// such as custom or extensible operators.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `replace_with` - The replacement string for the operator
/// * `size` - Optional size specification
/// * `token` - Optional associated token
///
/// # Usage
///
/// Infix nodes handle custom operators and extensible operator syntax
/// in mathematical expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeInfix {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The replacement string for the operator
    pub replace_with: String,
    /// Optional size specification
    pub size: Option<MeasurementOwned>,
    /// Optional associated token
    pub token: Option<Token>,
}

/// Represents internal parser nodes for implementation details.
///
/// This struct handles nodes used internally by the parser for
/// processing and transformation purposes.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
///
/// # Usage
///
/// Internal nodes are used for parser implementation details and
/// intermediate processing steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeInternal {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
}

/// Represents explicit kerning/spacing adjustments in mathematical expressions.
///
/// This struct handles manual spacing adjustments between mathematical elements
/// for precise layout control.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `dimension` - The spacing amount ([`MeasurementOwned`])
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX kerning commands:
/// ```latex
/// \kern 0.5em    % Positive kerning
/// \mkern 3mu     % Math unit kerning
/// ```
///
/// # Usage
///
/// Kern nodes provide fine-grained control over spacing in mathematical
/// expressions for typographical precision.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeKern {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The spacing amount ([`MeasurementOwned`])
    pub dimension: MeasurementOwned,
}

/// Represents overlapping content (lap) in mathematical expressions.
///
/// This struct handles text or symbols that overlap other content,
/// commonly used for annotations or special positioning.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `alignment` - The alignment for overlapping ("l", "r", "c")
/// * `body` - The overlapping content
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX lap commands:
/// ```latex
/// \rlap{*}     % Right overlap
/// \llap{#}     % Left overlap
/// \clap{^}     % Center overlap
/// ```
///
/// # Usage
///
/// Lap nodes enable overlapping text for annotations, superscripts,
/// or special positioning requirements.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeLap {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The alignment for overlapping ("l", "r", "c")
    pub alignment: LapAlignment,
    /// The overlapping content
    pub body: Box<AnyParseNode>,
}

/// Alignment options for overlapping content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LapAlignment {
    /// Align left
    Left,
    /// Align center
    Center,
    /// Align right
    Right,
}

impl LapAlignment {
    /// Get as static string reference.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Left => "llap",
            Self::Center => "clap",
            Self::Right => "rlap",
        }
    }
}

/// Represents left-right delimiter pairs in mathematical expressions.
///
/// This struct handles matching delimiter pairs that enclose mathematical
/// content, such as parentheses, brackets, and braces with automatic sizing.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The enclosed mathematical expressions
/// * `left` - The left delimiter symbol
/// * `right` - The right delimiter symbol
/// * `right_color` - Optional color for the right delimiter
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX left-right delimiters:
/// ```latex
/// \left( \frac{a}{b} \right)
/// \left[ x + y \right]
/// \left\{ a, b, c \right\}
/// ```
///
/// # Usage
///
/// LeftRight nodes automatically size delimiters to fit their content,
/// ensuring proper visual grouping of mathematical expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeLeftRight {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The enclosed mathematical expressions
    pub body: Vec<AnyParseNode>,
    /// The left delimiter symbol
    pub left: String,
    /// The right delimiter symbol
    pub right: String,
    /// Optional color for the right delimiter
    pub right_color: Option<String>,
}

/// Represents right delimiters in left-right pairs.
///
/// This struct handles the right delimiter in a left-right delimiter pair,
/// allowing for separate styling or coloring.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `delim` - The delimiter symbol
/// * `color` - Optional color specification
///
/// # Usage
///
/// LeftRightRight nodes handle the right side of delimiter pairs,
/// supporting different styling from the left delimiter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeLeftRightRight {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The delimiter symbol
    pub delim: String,
    /// Optional color specification
    pub color: Option<String>,
}

/// Represents different renderings for various math styles.
///
/// This struct handles expressions that display differently depending on the
/// mathematical context (display, text, script, scriptscript sizes).
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `display` - Display style rendering
/// * `text` - Text style rendering
/// * `script` - Script style rendering
/// * `scriptscript` - Scriptscript style rendering
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX math choice command:
/// ```latex
/// \mathchoice{\sum}{\sum}{\sum}{\sum}
/// ```
///
/// # Usage
///
/// MathChoice nodes allow different visual representations based on the
/// mathematical context, ensuring optimal readability at different sizes.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeMathChoice {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// Display style rendering
    pub display: Vec<AnyParseNode>,
    /// Text style rendering
    pub text: Vec<AnyParseNode>,
    /// Script style rendering
    pub script: Vec<AnyParseNode>,
    /// Scriptscript style rendering
    pub scriptscript: Vec<AnyParseNode>,
}

/// Represents middle delimiters in mathematical expressions.
///
/// This struct handles delimiters that appear in the middle of expressions,
/// such as separators in continued fractions or special punctuation.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `delim` - The delimiter symbol
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX middle delimiters:
/// ```latex
/// \middle|
/// \middle.
/// ```
///
/// # Usage
///
/// Middle nodes handle delimiters that separate parts of mathematical
/// expressions without being part of left-right pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeMiddle {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The delimiter symbol
    pub delim: String,
}

/// Represents math class specifications for mathematical elements.
///
/// This struct handles the classification of mathematical symbols for
/// proper spacing and rendering behavior.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `mclass` - The math class ("mord", "mbin", "mrel", etc.)
/// * `body` - The mathematical expressions
/// * `is_character_box` - Whether this represents a single character box
///
///
/// # Usage
///
/// Mclass nodes control the spacing and positioning behavior of mathematical
/// elements according to their semantic classification.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeMclass {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The math class ("mord", "mbin", "mrel", etc.)
    pub mclass: DomType,
    /// The mathematical expressions
    pub body: Vec<AnyParseNode>,
    /// Whether this represents a single character box
    pub is_character_box: bool,
}

/// Represents operator names with special formatting.
///
/// This struct handles named operators like "lim", "max", "sin" that
/// have specific formatting rules for subscripts and superscripts.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The operator name content
/// * `always_handle_sup_sub` - Force special subscript/superscript handling
/// * `limits` - Whether to place limits above/below (true) or as scripts
///   (false)
/// * `parent_is_sup_sub` - Whether this is within a sup/sub context
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX operator name commands:
/// ```latex
/// \lim_{x \to 0} f(x)
/// \max_{a,b} f(a,b)
/// \sin^2 \theta
/// ```
///
/// # Usage
///
/// OperatorName nodes ensure proper formatting of mathematical operators
/// with their arguments and limits.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeOperatorName {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The operator name content
    pub body: Vec<AnyParseNode>,
    /// Force special subscript/superscript handling
    pub always_handle_sup_sub: bool,
    /// Whether to place limits above/below (true) or as scripts (false)
    pub limits: bool,
    /// Whether this is within a sup/sub context
    pub parent_is_sup_sub: bool,
}

/// Represents overlines above mathematical expressions.
///
/// This struct handles horizontal lines drawn above mathematical content
/// for emphasis or special notation.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The expression being overlined
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX overline command:
/// ```latex
/// \overline{ABC}
/// ```
///
/// # Usage
///
/// Overline nodes provide visual emphasis for mathematical expressions,
/// commonly used for repeating decimals or special notation.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeOverline {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The expression being overlined
    pub body: Box<AnyParseNode>,
}

/// Represents phantom content for spacing purposes.
///
/// This struct handles invisible content that affects layout and spacing
/// without being visible in the final output.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The phantom content (invisible but affects spacing)
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX phantom commands:
/// ```latex
/// \phantom{x}    % Invisible but takes space
/// \hphantom{y}   % Horizontal space only
/// \vphantom{z}   % Vertical space only
/// ```
///
/// # Usage
///
/// Phantom nodes allow precise control over spacing and alignment by
/// including invisible content that affects layout calculations.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodePhantom {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The phantom content (invisible but affects spacing)
    pub body: Vec<AnyParseNode>,
}

/// Represents horizontal phantom content.
///
/// This struct handles invisible content that only affects horizontal spacing
/// and width calculations.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The horizontal phantom content
///
/// # Usage
///
/// Hphantom nodes reserve horizontal space without vertical extent,
/// useful for alignment and spacing control.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeHphantom {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The horizontal phantom content
    pub body: Box<AnyParseNode>,
}

/// Represents vertical phantom content.
///
/// This struct handles invisible content that only affects vertical spacing
/// and height calculations.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The vertical phantom content
///
///
/// # Usage
///
/// Vphantom nodes reserve vertical space without horizontal extent,
/// useful for consistent baseline alignment.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeVphantom {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The vertical phantom content
    pub body: Box<AnyParseNode>,
}

/// Represents poor man's bold text formatting.
///
/// This struct handles bold formatting created by overprinting characters,
/// used when proper bold fonts are not available.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `mclass` - The math class of the content
/// * `body` - The content to be made bold
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX poor man's bold:
/// ```latex
/// \pmb{x}    % Poor man's bold
/// ```
///
/// # Usage
///
/// Pmb nodes create bold appearance by slight overprinting of characters,
/// useful for mathematical notation when bold fonts are unavailable.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodePmb {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The math class of the content
    pub mclass: DomType,
    /// The content to be made bold
    pub body: Vec<AnyParseNode>,
}

/// Represents raised or lowered content in mathematical expressions.
///
/// This struct handles vertical displacement of mathematical content
/// for special positioning requirements.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `dy` - The vertical displacement amount ([`MeasurementOwned`])
/// * `body` - The content to be displaced
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX raisebox command:
/// ```latex
/// \raisebox{0.5em}{x}
/// ```
///
/// # Usage
///
/// Raisebox nodes allow precise vertical positioning of mathematical elements
/// for special layout requirements.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeRaisebox {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The vertical displacement amount ([`MeasurementOwned`])
    pub dy: MeasurementOwned,
    /// The content to be displaced
    pub body: Box<AnyParseNode>,
}

/// Represents horizontal or vertical rules in mathematical expressions.
///
/// This struct handles the creation of straight lines for various
/// mathematical notation purposes.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `shift` - Optional vertical shift of the rule
/// * `width` - The width of the rule ([`MeasurementOwned`])
/// * `height` - The height/thickness of the rule ([`MeasurementOwned`])
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX rule command:
/// ```latex
/// \rule{2em}{0.1em}
/// ```
///
/// # Usage
///
/// Rule nodes create horizontal or vertical lines for mathematical
/// notation, such as fraction bars or special symbols.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeRule {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// Optional vertical shift of the rule
    pub shift: Option<MeasurementOwned>,
    /// The width of the rule ([`MeasurementOwned`])
    pub width: MeasurementOwned,
    /// The height/thickness of the rule ([`MeasurementOwned`])
    pub height: MeasurementOwned,
}

/// Represents size changes for mathematical expressions.
///
/// This struct handles scaling of mathematical content to different
/// display sizes for various contexts.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `size` - The size level/index
/// * `body` - The expressions to be sized
///
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX sizing commands:
/// ```latex
/// \scriptsize x
/// \large y
/// \Huge z
/// ```
///
/// # Usage
///
/// Sizing nodes control the scale of mathematical expressions for
/// different display contexts and emphasis.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeSizing {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The size level/index
    pub size: usize,
    /// The expressions to be sized
    pub body: Vec<AnyParseNode>,
}

/// Represents smashed content that ignores height or depth.
///
/// This struct handles content where the height or depth is ignored
/// for layout purposes, useful for alignment control.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The content to be smashed
/// * `smash_height` - Whether to ignore the height above baseline
/// * `smash_depth` - Whether to ignore the depth below baseline
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX smash commands:
/// ```latex
/// \smash{x}        % Smash both
/// \smash[t]{y}     % Smash top only
/// \smash[b]{z}     % Smash bottom only
/// ```
///
/// # Usage
///
/// Smash nodes allow content to be treated as having zero height or depth
/// for alignment purposes without affecting the actual rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeSmash {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The content to be smashed
    pub body: Box<AnyParseNode>,
    /// Whether to ignore the height above baseline
    pub smash_height: bool,
    /// Whether to ignore the depth below baseline
    pub smash_depth: bool,
}

/// Represents square roots and nth roots in mathematical expressions.
///
/// This struct handles radical expressions with optional indices
/// for nth roots.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The expression under the radical
/// * `index` - Optional index for nth roots
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX radical commands:
/// ```latex
/// \sqrt{x}      % Square root
/// \sqrt[3]{x}   % Cube root
/// \sqrt[n]{x}   % nth root
/// ```
///
/// # Usage
///
/// Sqrt nodes represent radical expressions, automatically sizing the
/// radical symbol to fit the content and handling nth root indices.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeSqrt {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The expression under the radical
    pub body: AnyParseNode,
    /// Optional index for nth roots
    pub index: Option<AnyParseNode>,
}

/// Represents underlines below mathematical expressions.
///
/// This struct handles horizontal lines drawn below mathematical content
/// for emphasis or special notation.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The expression being underlined
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX underline command:
/// ```latex
/// \underline{ABC}
/// ```
///
/// # Usage
///
/// Underline nodes provide visual emphasis for mathematical expressions,
/// commonly used for highlighting or special notation.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeUnderline {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The expression being underlined
    pub body: Box<AnyParseNode>,
}

/// Represents vertically centered content in mathematical expressions.
///
/// This struct handles content that should be centered vertically
/// relative to the current line.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `body` - The content to be vertically centered
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX vcenter command:
/// ```latex
/// \vcenter{x}
/// ```
///
/// # Usage
///
/// Vcenter nodes center content vertically, useful for aligning
/// elements in complex mathematical layouts.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeVcenter {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The content to be vertically centered
    pub body: Box<AnyParseNode>,
}

/// Represents extensible arrows in mathematical expressions.
///
/// This struct handles arrows that can stretch to fit their content,
/// with optional labels above or below.
///
/// # Fields
///
/// * `mode` - The parsing mode ([`Mode::Math`] or [`Mode::Text`])
/// * `loc` - Optional source location for error reporting
/// * `label` - The arrow type/label (e.g., "->", "<=>")
/// * `body` - The content above the arrow
/// * `below` - Optional content below the arrow
///
/// # LaTeX Correspondence
///
/// Corresponds to LaTeX extensible arrow commands:
/// ```latex
/// \xleftarrow{f}
/// \xrightarrow[g]{f}
/// \xleftrightarrow{h}
/// ```
///
/// # Usage
///
/// XArrow nodes create arrows that automatically size to fit their labels,
/// commonly used for function mappings and transformations.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseNodeXArrow {
    /// The parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    /// Optional source location for error reporting
    pub loc: Option<SourceLocation>,
    /// The arrow type/label (e.g., "->", "<=>")
    pub label: String,
    /// The content above the arrow
    pub body: Option<Box<AnyParseNode>>,
    /// Optional content below the arrow
    pub below: Option<Box<AnyParseNode>>,
}

/// Type alias for symbol parse nodes that represent individual mathematical
/// symbols.
///
/// This alias refers to the subset of [`AnyParseNode`] variants that represent
/// atomic mathematical symbols with specific spacing and rendering properties.
/// These include atoms, operators, ordinals, and other symbol-like elements.
///
/// # Symbol Types
///
/// The following [`AnyParseNode`] variants are considered symbol nodes:
/// * [`AnyParseNode::Atom`] - Atomic symbols with mathematical meaning
/// * [`AnyParseNode::MathOrd`] - Ordinary mathematical symbols
/// * [`AnyParseNode::Op`] - Mathematical operators
/// * [`AnyParseNode::Spacing`] - Explicit spacing elements
/// * [`AnyParseNode::TextOrd`] - Text symbols in math mode
/// * [`AnyParseNode::AccentToken`] - Accent symbol components
/// * [`AnyParseNode::OpToken`] - Operator symbol components
///
/// # Usage
///
/// Used for type checking and validation when a function expects only
/// symbol-type nodes. The utility functions [`check_symbol_node_type`] and
/// [`assert_symbol_node_type`] work specifically with this type of node.
///
/// # See Also
///
/// * [`check_symbol_node_type`] - Check if a node is a symbol node
/// * [`assert_symbol_node_type`] - Assert that a node is a symbol node
pub type SymbolParseNode = AnyParseNode;

/// Type alias for parse nodes representing unsupported or partially supported
/// commands.
///
/// This alias currently refers to color-related nodes that may have limited
/// functionality or require special handling in certain rendering contexts.
///
/// # Current Usage
///
/// Primarily used for [`ParseNodeColor`] nodes, which represent color changes
/// in mathematical expressions. These nodes may not be fully supported in all
/// output formats or rendering engines.
///
/// # Future Extensions
///
/// This type alias may be extended to include other command types that have
/// limited support or require special processing in the KaTeX rendering
/// pipeline.
///
/// # See Also
///
/// * [`ParseNodeColor`] - The color node type
pub type UnsupportedCmdParseNode = ParseNodeColor;

/// Primary type alias for all parse nodes in the KaTeX AST.
///
/// This is the main type used throughout the KaTeX parsing and rendering
/// pipeline to represent mathematical expressions. It is equivalent to
/// [`AnyParseNode`] and provides a more convenient name for the core AST node
/// type.
///
/// # Architecture
///
/// The parse node system uses an enum-based approach where each variant
/// represents a different type of mathematical construct, from simple symbols
/// to complex structures like fractions and arrays.
///
/// # Usage
///
/// This type is used extensively in:
/// * Parser output - the result of parsing LaTeX input
/// * AST transformations - modifying or analyzing the parse tree
/// * Rendering pipeline - converting to visual output
/// * Type checking and validation - ensuring correct node types
///
/// # See Also
///
/// * [`AnyParseNode`] - The underlying enum type
/// * [`NodeType`] - Discriminant type for runtime type checking
/// * [`assert_node_type`] - Type assertion utility
pub type ParseNode = AnyParseNode;

/// Errors that can occur during parse node validation and type checking.
///
/// This enum represents the various error conditions that can arise when
/// working with parse nodes, particularly during type assertions and
/// validation.
///
/// The error type implements [`std::error::Error`] and [`std::fmt::Display`]
/// for proper error handling and reporting throughout the KaTeX pipeline.
///
/// # Usage
///
/// These errors are typically returned by:
/// * [`assert_node_type`] - When type assertion fails
/// * [`assert_symbol_node_type`] - When symbol node assertion fails
///
/// They provide detailed information about what went wrong during node
/// validation, helping with debugging and error reporting in parsing and
/// rendering operations.
///
/// # Error Messages
///
/// Error messages are designed to be informative and include both the expected
/// and actual types to aid in troubleshooting type-related issues.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseNodeError {
    /// A node has an unexpected type during type checking
    #[error("Expected node of type {expected}, but got {actual}")]
    TypeMismatch {
        /// The expected [`NodeType`] that the node should have
        expected: NodeType,
        /// The actual type as a string representation
        actual: String,
    },
    /// A node is not one of the symbol node types
    #[error("Expected node of symbol group type, but got {actual}")]
    NotSymbolNode {
        /// The actual node type as a string
        actual: String,
    },
}

/// Asserts that a parse node has the expected type and returns the node type.
///
/// This function performs runtime type checking on parse nodes, ensuring that
/// a node matches the expected [`NodeType`] before allowing further processing.
/// It's particularly useful when working with the dynamically-typed
/// [`AnyParseNode`] enum.
///
/// # Parameters
///
/// * `node` - An optional reference to the parse node to check
/// * `expected_type` - The expected [`NodeType`] that the node should have
///
/// # Returns
///
/// * `Ok(NodeType)` - The node type if the type matches
/// * `Err(ParseNodeError::TypeMismatch)` - If the node has a different type
/// * `Err(ParseNodeError::TypeMismatch)` - If `node` is `None` (with "null" as
///   actual type)
///
/// # Error Handling
///
/// The function provides detailed error information through [`ParseNodeError`],
/// including both the expected and actual types to aid in debugging.
///
/// # Performance
///
/// This function performs a simple discriminant check and string conversion,
/// making it suitable for use in performance-critical code paths.
///
/// # See Also
///
/// * [`NodeType`] - The enum of possible node types
/// * [`ParseNodeError`] - Error type returned on failure
/// * [`check_symbol_node_type`] - Check for symbol node types specifically
pub fn assert_node_type(
    node: Option<&AnyParseNode>,
    expected_type: NodeType,
) -> Result<NodeType, ParseNodeError> {
    let node = node.ok_or_else(|| ParseNodeError::TypeMismatch {
        expected: expected_type,
        actual: "null".to_owned(),
    })?;

    let actual_type = NodeType::from(node);

    if actual_type == expected_type {
        Ok(actual_type)
    } else {
        Err(ParseNodeError::TypeMismatch {
            expected: expected_type,
            actual: actual_type.to_string(),
        })
    }
}

/// Checks if a parse node represents a symbol-type element and returns the node
/// type if so.
///
/// This function determines whether a given parse node belongs to the symbol
/// group, which includes atomic symbols, operators, ordinals, and other
/// fundamental mathematical elements that have specific spacing and rendering
/// properties.
///
/// # Symbol Node Types
///
/// The following [`AnyParseNode`] variants are considered symbol nodes:
/// * [`AnyParseNode::Atom`] - Atomic symbols with mathematical meaning
/// * [`AnyParseNode::MathOrd`] - Ordinary mathematical symbols
/// * [`AnyParseNode::Op`] - Mathematical operators
/// * [`AnyParseNode::Spacing`] - Explicit spacing elements
/// * [`AnyParseNode::TextOrd`] - Text symbols in math mode
/// * [`AnyParseNode::AccentToken`] - Accent symbol components
/// * [`AnyParseNode::OpToken`] - Operator symbol components
///
/// # Parameters
///
/// * `node` - An optional reference to the parse node to check
///
/// # Returns
///
/// * `Some(NodeType)` - The node type if it's a symbol node
/// * `None` - If the node is not a symbol node or if `node` is `None`
///
/// # Examples
///
/// Checking various node types:
/// ```rust
/// use katex::parser::parse_node::*;
/// use katex::symbols::Atom;
/// use katex::types::Mode;
///
/// // Symbol node - will return Some
/// let atom_node = AnyParseNode::Atom(ParseNodeAtom {
///     family: Atom::Bin,
///     mode: Mode::Math,
///     loc: None,
///     text: "+".to_string(),
/// });
/// assert!(check_symbol_node_type(Some(&atom_node)).is_some());
///
/// // Non-symbol node - will return None
/// let array_node = AnyParseNode::Array(ParseNodeArray {
///     mode: Mode::Math,
///     loc: None,
///     col_separation_type: None,
///     hskip_before_and_after: None,
///     add_jot: None,
///     cols: None,
///     arraystretch: 1.0,
///     body: vec![],
///     row_gaps: vec![],
///     h_lines_before_row: vec![],
///     tags: None,
///     leqno: None,
///     is_cd: None,
/// });
/// assert!(check_symbol_node_type(Some(&array_node)).is_none());
///
/// // None input - will return None
/// assert!(check_symbol_node_type(None).is_none());
/// ```
///
/// # Usage
///
/// This function is useful when you need to handle symbol nodes differently
/// from structural nodes (like arrays, fractions, etc.) in processing or
/// rendering logic.
///
/// # Performance
///
/// Uses a simple pattern match on the node variant, making it very efficient
/// for use in performance-critical code.
///
/// # See Also
///
/// * [`assert_symbol_node_type`] - Similar function that returns an error
///   instead of `None`
/// * [`SymbolParseNode`] - Type alias for symbol parse nodes
/// * [`assert_node_type`] - More general type checking function
#[must_use]
pub fn check_symbol_node_type(node: Option<&AnyParseNode>) -> Option<NodeType> {
    let node = node?;

    // Check if this node is one of the symbol types
    match node {
        AnyParseNode::Atom(_)
        | AnyParseNode::MathOrd(_)
        | AnyParseNode::Spacing(_)
        | AnyParseNode::TextOrd(_)
        | AnyParseNode::AccentToken(_)
        | AnyParseNode::OpToken(_) => Some(NodeType::from(node)),
        _ => None,
    }
}

/// Asserts that a parse node is a symbol-type element and returns the node
/// type, or returns an error.
///
/// This function is similar to [`check_symbol_node_type`] but returns a
/// [`ParseNodeError`] instead of `None` when the node is not a symbol type.
/// It's useful when you require a symbol node and want detailed error
/// information about why the assertion failed.
///
/// # Symbol Node Types
///
/// The following [`AnyParseNode`] variants are considered symbol nodes:
/// * [`AnyParseNode::Atom`] - Atomic symbols with mathematical meaning
/// * [`AnyParseNode::MathOrd`] - Ordinary mathematical symbols
/// * [`AnyParseNode::Op`] - Mathematical operators
/// * [`AnyParseNode::Spacing`] - Explicit spacing elements
/// * [`AnyParseNode::TextOrd`] - Text symbols in math mode
/// * [`AnyParseNode::AccentToken`] - Accent symbol components
/// * [`AnyParseNode::OpToken`] - Operator symbol components
///
/// # Parameters
///
/// * `node` - An optional reference to the parse node to check
///
/// # Returns
///
/// * `Ok(NodeType)` - The node type if it's a symbol node
/// * `Err(ParseNodeError::NotSymbolNode)` - If the node is not a symbol node
/// * `Err(ParseNodeError::NotSymbolNode)` - If `node` is `None` (with "null" as
///   the type)
///
/// # Error Handling
///
/// Provides detailed error information through
/// [`ParseNodeError::NotSymbolNode`], including the actual node type that was
/// encountered.
///
/// # Usage
///
/// Use this function when you need to ensure a node is a symbol type and want
/// to handle the error case explicitly, rather than just checking and
/// proceeding.
///
/// # Performance
///
/// Similar performance characteristics to [`check_symbol_node_type`], with the
/// added overhead of error construction on failure.
///
/// # See Also
///
/// * [`check_symbol_node_type`] - Non-panicking version that returns `Option`
/// * [`SymbolParseNode`] - Type alias for symbol parse nodes
/// * [`ParseNodeError`] - Error type returned on failure
pub fn assert_symbol_node_type(node: Option<&AnyParseNode>) -> Result<NodeType, ParseNodeError> {
    check_symbol_node_type(node).ok_or_else(|| {
        let actual = node.map_or_else(|| "null".to_owned(), |n| NodeType::from(n).to_string());

        ParseNodeError::NotSymbolNode { actual }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_node_type_success() {
        let node = AnyParseNode::Atom(ParseNodeAtom {
            family: Atom::Bin,
            mode: Mode::Math,
            loc: None,
            text: "+".to_owned(),
        });

        let result = assert_node_type(Some(&node), NodeType::Atom);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NodeType::Atom);
    }

    #[test]
    fn test_assert_node_type_failure() {
        let node = AnyParseNode::Atom(ParseNodeAtom {
            family: Atom::Bin,
            mode: Mode::Math,
            loc: None,
            text: "+".to_owned(),
        });

        let result = assert_node_type(Some(&node), NodeType::MathOrd);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseNodeError::TypeMismatch {
                expected: NodeType::MathOrd,
                actual: "atom".to_owned()
            }
        );
    }

    #[test]
    fn test_assert_node_type_none() {
        let result = assert_node_type(None, NodeType::Atom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseNodeError::TypeMismatch {
                expected: NodeType::Atom,
                actual: "null".to_owned()
            }
        );
    }

    #[test]
    fn test_check_symbol_node_type_success() {
        let node = AnyParseNode::Atom(ParseNodeAtom {
            family: Atom::Bin,
            mode: Mode::Math,
            loc: None,
            text: "+".to_owned(),
        });

        let result = check_symbol_node_type(Some(&node));
        assert!(result.is_some());
    }

    #[test]
    fn test_check_symbol_node_type_failure() {
        let node = AnyParseNode::Array(ParseNodeArray {
            mode: Mode::Math,
            loc: None,
            col_separation_type: None,
            hskip_before_and_after: None,
            add_jot: None,
            cols: None,
            arraystretch: 1.0,
            body: vec![],
            row_gaps: vec![],
            h_lines_before_row: vec![],
            tags: None,
            leqno: None,
            is_cd: None,
        });

        let result = check_symbol_node_type(Some(&node));
        assert!(result.is_none());
    }

    #[test]
    fn test_assert_symbol_node_type_success() {
        let node = AnyParseNode::Atom(ParseNodeAtom {
            family: Atom::Bin,
            mode: Mode::Math,
            loc: None,
            text: "+".to_owned(),
        });

        let result = assert_symbol_node_type(Some(&node));
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_symbol_node_type_failure() {
        let node = AnyParseNode::Array(ParseNodeArray {
            mode: Mode::Math,
            loc: None,
            col_separation_type: None,
            hskip_before_and_after: None,
            add_jot: None,
            cols: None,
            arraystretch: 1.0,
            body: vec![],
            row_gaps: vec![],
            h_lines_before_row: vec![],
            tags: None,
            leqno: None,
            is_cd: None,
        });

        let result = assert_symbol_node_type(Some(&node));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseNodeError::NotSymbolNode {
                actual: "array".to_owned()
            }
        );
    }
}
