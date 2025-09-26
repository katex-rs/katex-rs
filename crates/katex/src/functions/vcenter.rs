//! Vcenter function implementation for KaTeX Rust
//!
//! This module handles the \vcenter command, which vertically centers
//! mathematical expressions on the math axis.
//!
//! Migrated from KaTeX's vcenter.js.

use crate::build_common::{VListChild, VListElem, VListParam, make_v_list};
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeVcenter};
use crate::types::{ArgType, ParseError};
use crate::{KatexContext, build_html, build_mathml};

/// Registers the \vcenter function in the KaTeX context
pub fn define_vcenter(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Vcenter),
        names: &["\\vcenter"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Original]),
            allowed_in_text: false,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = args[0].clone();

            Ok(ParseNode::Vcenter(ParseNodeVcenter {
                mode: context.parser.mode,
                loc: context.loc(),
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for vcenter nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Vcenter(vcenter_node) = node else {
        return Err(ParseError::new("Expected Vcenter node"));
    };

    // Build the body group
    let body = build_html::build_group(ctx, &vcenter_node.body, options, None)?;

    // Calculate the vertical shift to center on the math axis
    let axis_height = options.font_metrics().axis_height;
    let dy = 0.5 * ((body.height() - axis_height) - (body.depth() + axis_height));

    // Create a vertical list with the shifted body
    let children = vec![VListChild::Elem(
        VListElem::builder().elem(body).build().into(),
    )];

    let vlist = make_v_list(
        VListParam::Shift {
            children,
            position_data: dy,
        },
        options,
    )?;

    Ok(vlist.into())
}

/// MathML builder for vcenter nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Vcenter(vcenter_node) = node else {
        return Err(ParseError::new("Expected Vcenter node"));
    };

    // Build the base group
    let base_group = build_mathml::build_group(ctx, &vcenter_node.body, options)?;

    // Use mpadded element with vcenter class as a breadcrumb
    // MathML doesn't have direct support for vcenter
    let mut mpadded = MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .children(vec![base_group])
        .build();

    mpadded
        .attributes
        .insert("class".to_owned(), "vcenter".to_owned());

    Ok(MathDomNode::Math(mpadded))
}
