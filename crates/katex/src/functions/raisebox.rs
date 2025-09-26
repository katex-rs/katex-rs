//! Raisebox function implementation for KaTeX Rust
//!
//! This module handles the \raisebox command, which vertically displaces
//! mathematical content by a specified amount.

use crate::build_common::{VListElem, VListParam, make_v_list};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeRaisebox};
use crate::types::{ArgType, ParseError, ParseErrorKind};
use crate::{build_html, build_mathml};

/// Registers the \raisebox function in the KaTeX context
pub fn define_raisebox(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Raisebox),
        names: &["\\raisebox"],
        props: FunctionPropSpec {
            num_args: 2,
            arg_types: Some(vec![ArgType::Size, ArgType::Hbox]),
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let amount = match &args[0] {
                ParseNode::Size(size_node) => size_node.value.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ArgumentMustBeSize {
                        position: "First",
                    }));
                }
            };

            let body = args[1].clone();

            Ok(ParseNode::Raisebox(ParseNodeRaisebox {
                mode: context.parser.mode,
                loc: context.loc(),
                dy: amount,
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for raisebox nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Raisebox(raisebox_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Raisebox,
        }));
    };

    // Build the body content
    let body = build_html::build_group(ctx, &raisebox_node.body, options, None)?;

    // Calculate the displacement amount
    let dy = ctx.calculate_size(&raisebox_node.dy, options)?;

    // Create a vertical list with shift positioning
    let children = vec![VListElem::builder().elem(body).build().into()];

    let vlist = make_v_list(
        VListParam::Shift {
            children,
            position_data: -dy,
        },
        options,
    )?;

    Ok(vlist.into())
}

/// MathML builder for raisebox nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Raisebox(raisebox_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Raisebox,
        }));
    };

    // Build the body content
    let body_group = build_mathml::build_group(ctx, &raisebox_node.body, options)?;

    // Create mpadded element with voffset
    let dy_string = format!("{}{}", raisebox_node.dy.number, raisebox_node.dy.unit);

    let mut mpadded = MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .children(vec![body_group])
        .build();

    mpadded.attributes.insert("voffset".to_owned(), dy_string);

    Ok(MathDomNode::Math(mpadded))
}
