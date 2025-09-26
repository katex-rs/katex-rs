//! Math choice function implementation for KaTeX Rust
//!
//! This module handles the \mathchoice command, which allows different
//! visual representations based on the mathematical context
//! (display/text/script/scriptscript). Migrated from KaTeX's mathchoice.js.

use crate::build_common::make_fragment;
use crate::build_html::GroupType;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::MathDomNode;
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeMathChoice};
use crate::types::ParseError;
use crate::{build_html, build_mathml};

/// Choose the appropriate math style based on the current style size
///
/// This function selects one of the four arguments (display, text, script,
/// scriptscript) based on the current style size, similar to the JavaScript
/// chooseMathStyle function.
const fn choose_math_style<'a>(
    group: &'a ParseNodeMathChoice,
    options: &Options,
) -> &'a Vec<AnyParseNode> {
    match options.style.size {
        0 => &group.display, // DISPLAY
        // Same as fallback: 1 => &group.text,         // TEXT
        2 => &group.script,       // SCRIPT
        3 => &group.scriptscript, // SCRIPTSCRIPT
        _ => &group.text,         // Default fallback
    }
}

/// Registers the mathchoice function in the KaTeX context
pub fn define_mathchoice(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::MathChoice),
        names: &["\\mathchoice"],
        props: FunctionPropSpec {
            num_args: 4,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            Ok(ParseNode::MathChoice(ParseNodeMathChoice {
                mode: context.parser.mode,
                loc: context.loc(),
                display: ord_argument(&args[0]),
                text: ord_argument(&args[1]),
                script: ord_argument(&args[2]),
                scriptscript: ord_argument(&args[3]),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for mathchoice nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::MathChoice(group) = node else {
        return Err(ParseError::new("Expected MathChoice node"));
    };

    let body = choose_math_style(group, options);
    let elements =
        build_html::build_expression(ctx, body, options, GroupType::False, (None, None))?;
    Ok(make_fragment(&elements).into())
}

/// MathML builder for mathchoice nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::MathChoice(group) = node else {
        return Err(ParseError::new("Expected MathChoice node"));
    };

    let body = choose_math_style(group, options);
    build_mathml::build_expression_row(ctx, body, options, None)
}
