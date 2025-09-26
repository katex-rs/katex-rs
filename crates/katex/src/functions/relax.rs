//! Implementation of the \relax function in KaTeX
//!
//! This module implements the \relax LaTeX command, which is a no-op command
//! that can be used to stop expansion in TeX contexts.

use crate::context::KatexContext;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeInternal};

/// Register the \relax function
pub fn define_relax(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Internal),
        names: &["\\relax"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            allowed_in_argument: true,
            ..Default::default()
        },
        handler: Some(|context, _args, _opt_args| {
            // \relax is a no-op function that doesn't produce any visible output
            // but affects the parsing process by stopping expansion in certain contexts
            Ok(ParseNode::Internal(ParseNodeInternal {
                mode: context.parser.mode, // Use the current parser mode
                loc: context.loc(),
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}
