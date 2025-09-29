//! Accent under function implementations for KaTeX Rust
//!
//! This module handles accent under symbols in mathematical expressions,
//! migrated from KaTeX's accentunder.js.

use crate::build_common::{VListChild, VListElem, VListKern, VListParam, make_span, make_v_list};
use crate::build_html::build_group;
use crate::build_mathml;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeAccentUnder};
use crate::stretchy::{math_ml_node, svg_span};
use crate::tree::MathDomNode;
use crate::types::ClassList;
use crate::types::{ParseError, ParseErrorKind};

/// Under-accent commands
const UNDER_ACCENTS: &[&str] = &[
    "\\underleftarrow",
    "\\underrightarrow",
    "\\underleftrightarrow",
    "\\undergroup",
    "\\underlinesegment",
    "\\utilde",
];

/// Registers accent under functions in the KaTeX context
pub fn define_accentunder(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::AccentUnder),
        names: UNDER_ACCENTS,
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let base = args[0].clone();

            Ok(ParseNode::AccentUnder(Box::new(ParseNodeAccentUnder {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                is_stretchy: Some(true), // Under accents are typically stretchy
                is_shifty: Some(false),  // Under accents don't shift
                base: Box::new(base),
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for accent under nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    if let ParseNode::AccentUnder(accent_under) = node {
        let inner_group = build_group(ctx, &accent_under.base, options, None)?;
        let accent_body = svg_span(node, options)?;
        let kern = if accent_under.label == "\\utilde" {
            0.12f64
        } else {
            0f64
        };
        let vlist = make_v_list(
            VListParam::Top {
                position_data: inner_group.height(),
                children: vec![
                    VListElem::builder()
                        .elem(accent_body)
                        .wrapper_classes(ClassList::Static("svg-align"))
                        .build()
                        .into(),
                    VListChild::Kern(VListKern { size: kern }),
                    VListElem::builder().elem(inner_group).build().into(),
                ],
            },
            options,
        )?;

        Ok(make_span(
            ClassList::Const(&["mord", "accentunder"]),
            vec![vlist.into()],
            Some(options),
            None,
        )
        .into())
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::AccentUnder,
        }))
    }
}

/// MathML builder for accent under nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    if let ParseNode::AccentUnder(accent_under) = node {
        let accent_node = math_ml_node(accent_under.label.as_str());
        let group = build_mathml::build_group(ctx, &accent_under.base, options)?;
        let node = MathNode::builder()
            .node_type(MathNodeType::Munder)
            .children(vec![group, accent_node.into()])
            .build();
        Ok(node.into())
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::AccentUnder,
        }))
    }
}
