//! Function definition utilities for KaTeX Rust implementation
//!
//! This module provides utilities for defining mathematical functions and their
//! properties, similar to the JavaScript defineFunction.js module.

use crate::KatexContext;
use crate::dom_tree::HtmlDomNode;
use crate::options::Options;
use crate::parser::Parser;
use crate::parser::parse_node::NodeType;
use crate::parser::parse_node::ParseNode;
use crate::tree::MathDomNode;
use crate::types::{ArgType, BreakToken, ErrorLocationProvider as _, SourceLocation};
use crate::types::{ParseError, Token};

/// Context structure passed to function handlers during LaTeX function parsing
/// and rendering.
pub struct FunctionContext<'name, 'parser, 'token> {
    /// Function name
    pub func_name: &'name str,
    /// Parser instance
    pub parser: &'parser mut Parser<'token>,
    /// Optional token
    pub token: Option<&'parser Token>,
    /// Optional break token
    pub break_on_token_text: Option<&'parser BreakToken>,
}

impl FunctionContext<'_, '_, '_> {
    /// Get the SourceLocation of the current token, if available.
    #[must_use]
    pub fn loc(&self) -> Option<SourceLocation> {
        let t = self.token?;
        t.loc().cloned()
    }
}

/// Type alias for LaTeX function handlers that process mathematical
/// expressions.
pub type FunctionHandler = for<'name, 'parser, 'token> fn(
    FunctionContext<'name, 'parser, 'token>,
    args: Vec<ParseNode>,
    opt_args: Vec<Option<ParseNode>>,
) -> Result<ParseNode, ParseError>;

/// Type alias for functions that build HTML DOM nodes from mathematical parse
/// nodes.
pub type HtmlBuilder = fn(&ParseNode, &Options, &KatexContext) -> Result<HtmlDomNode, ParseError>;

/// Type alias for functions that build MathML DOM nodes from mathematical parse
/// nodes.
pub type MathMLBuilder = fn(&ParseNode, &Options, &KatexContext) -> Result<MathDomNode, ParseError>;

/// Configuration structure defining properties and parsing behavior for LaTeX
/// mathematical functions.
#[derive(Debug, Clone)]
pub struct FunctionPropSpec {
    /// The number of arguments the function takes
    pub num_args: usize,

    /// An array corresponding to each argument of the function, giving the
    /// type of argument that should be parsed
    pub arg_types: Option<Vec<ArgType>>,

    /// Whether it expands to a single token or a braced group of tokens
    pub allowed_in_argument: bool,

    /// Whether the function is allowed inside text mode
    pub allowed_in_text: bool,

    /// Whether the function is allowed inside math mode
    pub allowed_in_math: bool,

    /// The number of optional arguments the function should parse
    pub num_optional_args: usize,

    /// Whether the function is an infix operator
    pub infix: bool,

    /// Whether the function is a TeX primitive
    pub primitive: bool,
}

/// Default implementation for [`FunctionPropSpec`].
impl Default for FunctionPropSpec {
    fn default() -> Self {
        Self {
            num_args: 0,
            arg_types: None,
            allowed_in_argument: false,
            allowed_in_text: false,
            allowed_in_math: true,
            num_optional_args: 0,
            infix: false,
            primitive: false,
        }
    }
}

/// Complete specification for defining LaTeX mathematical functions in KaTeX.
pub struct FunctionDefSpec<'b> {
    /// Unique string to differentiate parse nodes
    pub node_type: Option<NodeType>,

    /// Function names (single name or list of names)
    pub names: &'b [&'b str],

    /// Properties that control how functions are parsed
    pub props: FunctionPropSpec,

    /// Handler function
    pub handler: Option<FunctionHandler>,

    /// HTML builder function
    pub html_builder: Option<HtmlBuilder>,

    /// MathML builder function
    pub mathml_builder: Option<MathMLBuilder>,
}

/// Runtime function specification used during LaTeX parsing and processing.
#[derive(Debug, Clone)]
pub struct FunctionSpec {
    /// Function type name (equivalent to `type` in KaTeX)
    pub node_type: Option<NodeType>,

    /// Number of arguments
    pub num_args: usize,

    /// Argument types
    pub arg_types: Option<Vec<ArgType>>,

    /// Allowed in argument position
    pub allowed_in_argument: bool,

    /// Allowed in text mode
    pub allowed_in_text: bool,

    /// Allowed in math mode
    pub allowed_in_math: bool,

    /// Number of optional arguments
    pub num_optional_args: usize,

    /// Infix operator
    pub infix: bool,

    /// TeX primitive
    pub primitive: bool,

    /// Handler function
    pub handler: Option<FunctionHandler>,
}

/// Normalizes a function argument by unwrapping single-element ordered groups.
#[must_use]
pub fn normalize_argument(arg: &ParseNode) -> &ParseNode {
    if let ParseNode::OrdGroup(ord) = arg
        && ord.body.len() == 1
    {
        return &ord.body[0];
    }
    arg
}

/// Normalizes a function argument into a list of elements for HTML/MathML
/// rendering.
#[must_use]
pub fn ord_argument(arg: &ParseNode) -> Vec<ParseNode> {
    if let ParseNode::OrdGroup(ord) = arg {
        return ord.body.clone();
    }
    vec![arg.clone()]
}
