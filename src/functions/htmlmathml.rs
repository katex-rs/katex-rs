//! HTML/MathML function implementations for KaTeX Rust
//!
//! This module handles the `\html@mathml` function, which allows different
//! content to be rendered in HTML and MathML formats. This is migrated from
//! KaTeX's htmlmathml.js.

use crate::build_common::make_fragment;
use crate::build_html;
use crate::build_mathml;
use crate::define_function::ord_argument;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::MathDomNode;
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNodeHtmlMathMl};
use crate::types::ParseError;

/// Registers the htmlmathml function in the KaTeX context
pub fn define_htmlmathml(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::HtmlMathMl),
        names: &["\\html@mathml"],
        props: FunctionPropSpec {
            num_args: 2,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            Ok(AnyParseNode::HtmlMathMl(ParseNodeHtmlMathMl {
                mode: context.parser.mode,
                loc: context.loc(),
                html: ord_argument(&args[0]),
                mathml: ord_argument(&args[1]),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for htmlmathml nodes
fn html_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::HtmlMathMl(group) = node else {
        return Err(ParseError::new("Expected HtmlMathMl node"));
    };

    let elements = build_html::build_expression(
        ctx,
        &group.html,
        options,
        build_html::GroupType::False,
        (None, None),
    )?;

    Ok(make_fragment(&elements).into())
}

/// MathML builder for htmlmathml nodes
fn mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::HtmlMathMl(group) = node else {
        return Err(ParseError::new("Expected HtmlMathMl node"));
    };

    build_mathml::build_expression_row(ctx, &group.mathml, options, None)
}
