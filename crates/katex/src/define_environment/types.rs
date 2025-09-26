//! Environment-related type definitions for KaTeX
//!
//! This module contains Rust equivalents of the JavaScript Flow type
//! definitions for environment specifications and handlers.

use crate::parser::Parser;
use crate::parser::parse_node::AnyParseNode;
use crate::parser::parse_node::NodeType;
use crate::types::{ArgType, Mode, ParseError};

/// Context information passed to environment handlers during LaTeX environment
/// parsing.
///
/// This struct encapsulates the current parsing state and environment details
/// necessary for processing LaTeX environments in KaTeX's mathematical
/// typesetting system. It provides essential information about the parsing
/// context, including the current mode and the specific environment name being
/// processed.
///
/// # Error Handling
///
/// Environment handlers should validate the context parameters and return
/// appropriate `ParseError` if the environment is not supported in the
/// current mode.
///
/// # See Also
///
/// - [`EnvHandler`]: The function type that receives this context
/// - [`Mode`]: Enumeration of parsing modes
/// - [`crate::parser`]: Parser module for LaTeX processing
pub struct EnvContext<'a, 'b> {
    /// The current parsing mode (e.g., math, text) as defined in [`Mode`].
    ///
    /// This determines how the environment content should be interpreted and
    /// rendered. For example, some environments may only be valid in math
    /// mode, while others can appear in text mode as well. The mode
    /// influences spacing, font selection, and other typesetting behaviors
    /// in LaTeX/KaTeX.
    ///
    /// # Examples
    ///
    /// - `Mode::Math`: For mathematical expressions and equations
    /// - `Mode::Text`: For regular text with embedded math
    pub mode: Mode,

    /// The name of the LaTeX environment being processed (e.g., "matrix",
    /// "align").
    ///
    /// Used to identify the specific environment type and apply appropriate
    /// parsing rules. Environment names correspond to LaTeX's
    /// `\begin{env_name}...\end{env_name}` syntax and determine the
    /// structural layout and rendering of the content within.
    ///
    /// # Examples
    ///
    /// - `"matrix"`: For matrix environments in mathematical expressions
    /// - `"align"`: For aligned equation environments
    /// - `"cases"`: For piecewise function definitions
    pub env_name: String,
    /// The parser instance used for processing the environment content.
    pub parser: &'a mut Parser<'b>,
}

/// Function type for handling LaTeX environment parsing and processing.
///
/// Environment handlers are responsible for transforming parsed LaTeX
/// environment arguments into appropriate parse nodes that can be rendered in
/// KaTeX's mathematical typesetting system. They encapsulate the logic for
/// specific environment types like matrices, alignments, and other structured
/// content.
///
/// # Parameters
///
/// * `context` - The [`EnvContext`] containing parsing mode and environment
///   name
/// * `args` - Vector of required arguments parsed from the environment content
/// * `opt_args` - Vector of optional arguments (may contain `None` for missing
///   optionals)
///
/// # Returns
///
/// Returns a [`Result`] containing either:
/// - [`AnyParseNode`]: The successfully parsed and processed environment node
/// - `ParseError`: If parsing or processing fails
///
/// # Error Handling
///
/// Handlers should return `ParseError` for:
/// - Invalid arguments or argument counts
/// - Unsupported parsing modes for the environment
/// - Malformed environment content
/// - Missing required arguments
///
/// # See Also
///
/// - [`EnvContext`]: Context information provided to handlers
/// - [`AnyParseNode`]: The parse node types that handlers produce
/// - `ParseError`: Error type for parsing failures
/// - [`crate::parser`]: Parser module that invokes these handlers
pub type EnvHandler = fn(
    context: EnvContext,
    args: Vec<AnyParseNode>,
    opt_args: Vec<Option<AnyParseNode>>,
) -> Result<AnyParseNode, ParseError>;

/// Configuration properties that control LaTeX environment parsing behavior.
///
/// These properties define the parsing rules and constraints for environment
/// definitions, including argument requirements, mode restrictions, and
/// optional parameter handling. They correspond to the parsing specifications
/// in KaTeX's JavaScript implementation and determine how the parser processes
/// environment arguments and content.
///
/// # See Also
///
/// - [`ArgType`]: Types of arguments that environments can accept
/// - [`crate::parser`]: Parser that applies these properties
#[derive(Debug, Clone)]
pub struct EnvProps {
    /// The number of required arguments the environment takes.
    ///
    /// This specifies how many mandatory arguments must be provided
    /// when the environment is used in LaTeX. For example, a matrix
    /// environment might require 0 arguments (content parsed internally),
    /// while a custom environment might require specific counts.
    ///
    /// # Examples
    ///
    /// - `Some(0)`: No required arguments (e.g., simple environments)
    /// - `Some(2)`: Two required arguments
    /// - `None`: Use default (typically 0)
    pub num_args: Option<usize>,

    /// Argument types for each required argument.
    ///
    /// Specifies the expected type of each argument in order. The length
    /// should match `num_args`. This ensures proper parsing and validation
    /// of environment arguments according to LaTeX/KaTeX conventions.
    ///
    /// # See Also
    ///
    /// - [`ArgType`]: Available argument types
    pub arg_types: Option<Vec<ArgType>>,

    /// Whether the environment is allowed inside text mode.
    ///
    /// Controls whether the environment can be used in regular text
    /// contexts or only within mathematical expressions. Most mathematical
    /// environments (matrices, equations) are restricted to math mode.
    ///
    /// # Examples
    ///
    /// - `Some(true)`: Can be used in both text and math modes
    /// - `Some(false)`: Math mode only (most mathematical environments)
    /// - `None`: Use default (typically false)
    pub allowed_in_text: Option<bool>,

    /// Number of optional arguments the environment accepts.
    ///
    /// Specifies how many optional arguments (enclosed in square brackets `[]`)
    /// the environment can take. These are processed separately from required
    /// arguments and may be `None` if not provided.
    ///
    /// # Examples
    ///
    /// - `Some(0)`: No optional arguments
    /// - `Some(1)`: One optional argument allowed
    /// - `None`: Use default (typically 0)
    pub num_optional_args: Option<usize>,
}

/// Provides sensible defaults for environment properties.
///
/// The defaults are chosen to represent a basic mathematical environment
/// that takes no arguments and is only allowed in math mode, which covers
/// most mathematical environments in LaTeX/KaTeX.
impl Default for EnvProps {
    fn default() -> Self {
        Self {
            num_args: Some(0),
            arg_types: None,
            allowed_in_text: Some(false),
            num_optional_args: Some(0),
        }
    }
}

/// Processed environment specification used during LaTeX parsing.
///
/// This struct represents the final, resolved configuration for an environment
/// after processing the definition specification. It contains
/// all necessary information for the parser to handle the environment
/// correctly, with defaults applied and validation performed.
///
/// # See Also
///
/// - [`EnvHandler`]: The handler function for processing the environment
/// - [`crate::parser`]: Parser that uses these specifications
#[derive(Debug, Clone)]
pub struct EnvSpec {
    /// Node type that this environment produces.
    ///
    /// Specifies the type of parse node that will be created when
    /// this environment is successfully parsed. This determines how
    /// the environment is represented internally and rendered.
    ///
    /// # Examples
    ///
    /// - `NodeType::Environment`: Generic environment
    /// - `NodeType::Matrix`: Matrix-specific environment
    pub node_type: NodeType,

    /// Number of required arguments for this environment.
    ///
    /// The exact count of mandatory arguments that must be provided.
    /// This is resolved from the optional `num_args` in [`EnvProps`].
    ///
    /// # Examples
    ///
    /// - `0`: No required arguments (content parsed internally)
    /// - `2`: Exactly two required arguments
    pub num_args: usize,

    /// Argument types for each required argument.
    ///
    /// Specifies the expected type of each argument. The vector length
    /// must match `num_args`. This is resolved from the optional
    /// `arg_types` in [`EnvProps`].
    ///
    /// # See Also
    ///
    /// - [`ArgType`]: Available argument types
    pub arg_types: Option<Vec<ArgType>>,

    /// Whether this environment is allowed in text mode.
    ///
    /// Controls whether the environment can be used in regular text
    /// contexts or only within mathematical expressions. This is
    /// resolved from the optional `allowed_in_text` in [`EnvProps`].
    ///
    /// # Examples
    ///
    /// - `true`: Can be used in both text and math modes
    /// - `false`: Math mode only (most mathematical environments)
    pub allowed_in_text: bool,

    /// Number of optional arguments this environment accepts.
    ///
    /// The exact count of optional arguments that can be provided.
    /// This is resolved from the optional `num_optional_args` in [`EnvProps`].
    ///
    /// # Examples
    ///
    /// - `0`: No optional arguments
    /// - `1`: One optional argument allowed
    pub num_optional_args: usize,

    /// The handler function that processes this environment.
    ///
    /// This function is called during parsing to validate arguments
    /// and create the appropriate parse node. It encapsulates the
    /// core logic for the environment type.
    ///
    /// # See Also
    ///
    /// - [`EnvHandler`]: The function signature
    pub handler: EnvHandler,
}

/// Provides a basic default environment specification.
///
/// The default implementation creates a minimal environment spec that can be
/// used as a starting point. The default handler returns an error, so this
/// should be customized with a proper handler for actual use.
///
/// # Warning
///
/// The default handler will always return an error. You must provide a
/// proper [`EnvHandler`] implementation for the environment to function.
impl Default for EnvSpec {
    fn default() -> Self {
        Self {
            node_type: NodeType::Environment, // Default to Environment node type
            num_args: 0,
            arg_types: None,
            allowed_in_text: false,
            num_optional_args: 0,
            handler: |_, _, _| Err(ParseError::new("Environment handler not implemented")),
        }
    }
}
