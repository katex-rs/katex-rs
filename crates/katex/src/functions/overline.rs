//! Overline function implementation for KaTeX Rust
//!
//! This module handles the \overline command, which draws a horizontal line
//! above mathematical expressions for emphasis or special notation.
//!
//! Migrated from KaTeX's overline.js.

use crate::build_common::{
    VListChild, VListElem, VListKern, VListParam, make_line_span, make_span, make_v_list,
};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeOverline};
use crate::types::{ParseError, ParseErrorKind};
use crate::{KatexContext, build_html, build_mathml};

/// Registers the \overline function in the KaTeX context
pub fn define_overline(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Overline),
        names: &["\\overline"],
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let body = args[0].clone();

            Ok(ParseNode::Overline(ParseNodeOverline {
                mode: context.parser.mode,
                loc: context.loc(),
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for overline nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Overline(overline_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Overline,
        }));
    };

    // Build the inner group in the cramped style
    let inner_group = build_html::build_group(
        ctx,
        &overline_node.body,
        &options.having_cramped_style(),
        None,
    )?;

    // Create the line above the body
    let line = make_line_span("overline-line", options, None);

    // Generate the vlist, with the appropriate kerns
    let default_rule_thickness = options.font_metrics().default_rule_thickness;
    let vlist = make_v_list(
        VListParam::FirstBaseline {
            children: vec![
                VListElem::builder().elem(inner_group).build().into(),
                VListChild::Kern(VListKern {
                    size: 3.0 * default_rule_thickness,
                }),
                VListElem::builder().elem(line.into()).build().into(),
                VListChild::Kern(VListKern {
                    size: default_rule_thickness,
                }),
            ],
        },
        options,
    )?;

    Ok(make_span(
        vec!["mord".to_owned(), "overline".to_owned()],
        vec![vlist.into()],
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for overline nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Overline(overline_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Overline,
        }));
    };

    let mut operator = MathNode::builder()
        .node_type(MathNodeType::Mo)
        .children(vec![MathDomNode::Text(TextNode {
            text: "\u{203e}".to_owned(),
        })]) // Unicode overline character
        .build();

    operator
        .attributes
        .insert("stretchy".to_owned(), "true".to_owned());

    let base_group = build_mathml::build_group(ctx, &overline_node.body, options)?;

    let mut mover = MathNode::builder()
        .node_type(MathNodeType::Mover)
        .children(vec![base_group, MathDomNode::Math(operator)])
        .build();

    mover
        .attributes
        .insert("accent".to_owned(), "true".to_owned());

    Ok(MathDomNode::Math(mover))
}
