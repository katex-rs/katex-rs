//! Implementation of math mode switching functions in KaTeX
//!
//! This module provides functions for switching between text and math modes
//! using delimiters like \(, \), $, and for handling mismatched delimiters.

use crate::context::KatexContext;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeStyling};
use crate::style::TEXT;
use crate::types::{BreakToken, Mode, ParseError, ParseErrorKind};

/// Register all math mode switching functions
pub fn define_math(ctx: &mut KatexContext) {
    // Switching from text mode back to math mode
    define_math_open(ctx);
    // Check for extra closing math delimiters
    define_math_close(ctx);
}

/// Register the math mode opening functions (\(, $)
fn define_math_open(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Styling),
        names: &["\\(", "$"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            allowed_in_math: false,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            let outer_mode = context.parser.mode;
            context.parser.switch_mode(Mode::Math);
            let close_token = if context.func_name == "\\(" {
                BreakToken::RightParen
            } else {
                BreakToken::Dollar
            };
            let body = context.parser.parse_expression(false, Some(&close_token))?;
            // Expect the closing token
            let token = context.parser.fetch()?;
            if token.text != close_token.as_ref() {
                return Err(ParseError::new(ParseErrorKind::ExpectedToken {
                    expected: close_token.as_ref().to_owned(),
                    found: token.text.to_owned_string(),
                }));
            }
            context.parser.consume();
            context.parser.switch_mode(outer_mode);
            Ok(ParseNode::Styling(ParseNodeStyling {
                mode: context.parser.mode,
                loc: context.loc(),
                style: TEXT,
                body,
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}

/// Register the math mode closing functions (\), \])
fn define_math_close(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Text),
        names: &["\\)", "\\]"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            allowed_in_math: false,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            Err(ParseError::new(ParseErrorKind::Mismatched {
                what: context.func_name,
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}
