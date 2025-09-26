//! Implementation of the \@char function in KaTeX
//!
//! This module implements the \@char LaTeX command, which converts a decimal
//! number to the corresponding Unicode character. It is used internally by
//! the \char macro to create symbols from code points.

use crate::context::KatexContext;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeTextOrd};
use crate::types::{ParseError, ParseErrorKind};

/// Register the \@char function
pub fn define_char(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::TextOrd),
        names: &["\\@char"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(
            |context: FunctionContext, args: Vec<ParseNode>, _opt_args: Vec<Option<ParseNode>>| {
                // Extract the first argument, which should be an ordgroup
                let arg = &args[0];
                let ParseNode::OrdGroup(ordgroup) = arg else {
                    return Err(ParseError::new("\\@char argument must be an ordgroup"));
                };

                // Concatenate all text from textord and mathord nodes in the group
                let mut number_str = String::new();
                for node in &ordgroup.body {
                    match node {
                        AnyParseNode::TextOrd(textord) => {
                            number_str.push_str(&textord.text);
                        }
                        AnyParseNode::MathOrd(mathord) => {
                            number_str.push_str(&mathord.text);
                        }
                        _ => {
                            return Err(ParseError::new(
                                "\\@char ordgroup must contain only textord or mathord nodes",
                            ));
                        }
                    }
                }

                // Parse the number
                let code_point: u32 = match number_str.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        return Err(ParseError::new(ParseErrorKind::CharNonNumericArgument {
                            value: number_str.clone(),
                        }));
                    }
                };

                // Validate the code point range
                if code_point > 0x10FFFF {
                    return Err(ParseError::new(ParseErrorKind::InvalidCharCodePoint {
                        code: number_str.clone(),
                    }));
                }

                // Convert code point to character(s)
                let text = char::from_u32(code_point).ok_or_else(|| {
                    ParseError::new(ParseErrorKind::InvalidCharCodePoint {
                        code: number_str.clone(),
                    })
                })?;

                // Return a textord node with the character
                Ok(ParseNode::TextOrd(ParseNodeTextOrd {
                    mode: context.parser.mode,
                    loc: context.loc(),
                    text: text.to_string(),
                }))
            },
        ),
        html_builder: None,
        mathml_builder: None,
    });
}
