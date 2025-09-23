//! Environment definition system for KaTeX
//!
//! This module provides functionality for defining LaTeX environments
//! and their corresponding HTML/MathML builders.

mod array;
mod cd;
mod types;

use crate::{
    define_function::{HtmlBuilder, MathMLBuilder},
    parser::parse_node::NodeType,
};

pub use array::{define_array, parse_array};
pub use cd::{define_cd, parse_cd};
pub use types::*;

/// Specification for defining LaTeX environments in KaTeX's parsing system.
///
/// This struct encapsulates all the information needed to define how a LaTeX
/// environment should be parsed, processed, and rendered. It serves as the
/// blueprint for environment definitions, mapping LaTeX syntax to KaTeX's
/// internal representation and output formats.
///
/// Environment definitions typically correspond to LaTeX's
/// `\begin{env}...\end{env}` syntax and are used for structured content like
/// matrices, alignments, and other mathematical constructs.
///
/// # See Also
///
/// - [`EnvProps`]: Properties controlling parsing behavior
/// - [`EnvHandler`]: Function type for processing environments
/// - [`HtmlBuilder`]: Function type for HTML generation
/// - [`MathMLBuilder`]: Function type for MathML generation
/// - [`crate::define_function`]: Related function definition system
#[derive(Debug, Clone)]
pub struct EnvDefSpec {
    /// Unique node type identifier for this environment.
    ///
    /// Specifies the type of parse node that this environment produces.
    /// This is used internally by the parser to categorize and process
    /// the environment appropriately.
    ///
    /// # Examples
    ///
    /// - `NodeType::Environment`: Generic environment node
    /// - `NodeType::Matrix`: Specific matrix environment type
    pub node_type: NodeType,

    /// List of LaTeX environment names that use this specification.
    ///
    /// Each name corresponds to a LaTeX environment like
    /// `\begin{matrix}...\end{matrix}`. Multiple names can share the same
    /// specification for environments with similar parsing rules (e.g.,
    /// "matrix" and "pmatrix").
    pub names: Vec<String>,

    /// Properties that control parsing behavior for this environment.
    ///
    /// Defines argument requirements, mode restrictions, and other parsing
    /// constraints that apply to this environment type.
    pub props: EnvProps,

    /// Core function that processes the environment's arguments and produces a
    /// parse node.
    ///
    /// This handler is responsible for validating arguments, processing the
    /// environment content, and creating the appropriate internal
    /// representation. It receives the parsing context and arguments, then
    /// returns a parse node that can be rendered to HTML/MathML.
    pub handler: EnvHandler,

    /// Optional function for generating HTML output from the parse node.
    ///
    /// If provided, this function will be called during HTML rendering to
    /// produce the final HTML representation of the environment. If `None`,
    /// a default HTML builder may be used or rendering may fail.
    ///
    /// # See Also
    ///
    /// - [`HtmlBuilder`]: The function signature for HTML builders
    pub html_builder: Option<HtmlBuilder>,

    /// Optional function for generating MathML output from the parse node.
    ///
    /// If provided, this function will be called during MathML rendering to
    /// produce the final MathML representation of the environment. If `None`,
    /// a default MathML builder may be used or rendering may fail.
    ///
    /// # See Also
    ///
    /// - [`MathMLBuilder`]: The function signature for MathML builders
    pub mathml_builder: Option<MathMLBuilder>,
}
