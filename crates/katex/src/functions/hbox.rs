//! Hbox function implementation for KaTeX Rust
//!
//! This module handles the \hbox command, migrated from KaTeX's hbox.js.
//! \hbox is provided for compatibility with LaTeX \vcenter.
//! In LaTeX, \vcenter can act only on a box, as in
//! \vcenter{\hbox{$\frac{a+b}{\dfrac{c}{d}}$}}
//! This function by itself doesn't do anything but prevent a soft line break.

use crate::build_common::make_fragment;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNodeHbox};
use crate::types::{ArgType, Mode, ParseError};
use crate::{KatexContext, build_html, build_mathml};

/// Registers hbox function in the KaTeX context
pub fn define_hbox(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Hbox),
        names: &["\\hbox"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Mode(Mode::Text)]),
            allowed_in_text: true,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = args[0].clone();

            Ok(AnyParseNode::Hbox(ParseNodeHbox {
                mode: context.parser.mode,
                loc: context.loc(),
                body: vec![body],
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for hbox nodes
fn html_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::Hbox(hbox_node) = node else {
        return Err(ParseError::new("Expected Hbox node"));
    };

    let elements = build_html::build_expression(
        ctx,
        &hbox_node.body,
        options,
        build_html::GroupType::False,
        (None, None),
    )?;
    Ok(make_fragment(&elements).into())
}

/// MathML builder for hbox nodes
fn mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::Hbox(hbox_node) = node else {
        return Err(ParseError::new("Expected Hbox node"));
    };

    let children = build_mathml::build_expression(ctx, &hbox_node.body, options, None)?;
    let mrow = MathNode::builder()
        .node_type(MathNodeType::Mrow)
        .children(children)
        .build();

    Ok(MathDomNode::Math(mrow))
}
