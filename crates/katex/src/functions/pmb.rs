//! Poor man's bold function implementation for KaTeX Rust
//!
//! This module handles the \pmb command, which simulates bold text by
//! overprinting characters with slight offsets using CSS text-shadow.
//!
//! Migrated from KaTeX's pmb.js.

use crate::build_common::make_span;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::functions::mclass::binrel_class;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodePmb};
use crate::types::{CssProperty, ParseError};
use crate::{KatexContext, build_html, build_mathml};

/// HTML builder for pmb nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Pmb(pmb_node) = node else {
        return Err(ParseError::new("Expected Pmb node"));
    };

    let elements = build_html::build_expression(
        ctx,
        &pmb_node.body,
        options,
        build_html::GroupType::True,
        (None, None),
    )?;
    let mut span = make_span(vec![pmb_node.mclass.clone()], elements, Some(options), None);

    // Apply text-shadow to simulate bold
    span.style
        .insert(CssProperty::TextShadow, "0.02em 0.01em 0.04px".to_owned());

    Ok(span.into())
}

/// MathML builder for pmb nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Pmb(pmb_node) = node else {
        return Err(ParseError::new("Expected Pmb node"));
    };

    let inner = build_mathml::build_expression(ctx, &pmb_node.body, options, None)?;

    // Wrap with an <mstyle> element
    let mut mstyle = MathNode::builder()
        .node_type(MathNodeType::Mstyle)
        .children(inner)
        .build();

    mstyle.attributes.insert(
        "style".to_owned(),
        "text-shadow: 0.02em 0.01em 0.04px".to_owned(),
    );

    Ok(MathDomNode::Math(mstyle))
}

/// Registers the \pmb function in the KaTeX context
pub fn define_pmb(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Pmb),
        names: &["\\pmb"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let mclass = binrel_class(&args[0]);

            Ok(ParseNode::Pmb(ParseNodePmb {
                mode: context.parser.mode,
                loc: context.loc(),
                mclass,
                body: ord_argument(&args[0]),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
