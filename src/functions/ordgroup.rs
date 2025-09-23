//! Ordgroup function implementations for KaTeX Rust
//!
//! This module handles ordered groups of mathematical expressions,
//! migrated from KaTeX's ordgroup.js.

use crate::build_common::make_span;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::MathDomNode;
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNodeOrdGroup};
use crate::types::ParseError;
use crate::{KatexContext, build_html, build_mathml, make_fragment};

/// Registers ordgroup functions in the KaTeX context
pub fn define_ordgroup(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::OrdGroup),
        names: &["ordgroup"],
        props: FunctionPropSpec::default(),
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = args[0].clone();
            let semisimple = context.func_name == "ordgroup"; // For now, assume ordgroup is semisimple

            Ok(AnyParseNode::OrdGroup(ParseNodeOrdGroup {
                mode: context.parser.mode,
                loc: context.loc(),
                body: vec![body],
                semisimple: Some(semisimple),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for ordgroup nodes
fn html_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::OrdGroup(group) = node else {
        return Err(ParseError::new("Expected OrdGroup node"));
    };

    if group.semisimple.unwrap_or(false) {
        let body = build_html::build_expression(
            ctx,
            &group.body,
            options,
            build_html::GroupType::False,
            (None, None),
        )?;
        Ok(make_fragment(&body).into())
    } else {
        // Use makeSpan with "mord" class
        let body = build_html::build_expression(
            ctx,
            &group.body,
            options,
            build_html::GroupType::True,
            (None, None),
        )?;
        let span = make_span(vec!["mord".to_owned()], body, Some(options), None);
        Ok(span.into())
    }
}

/// MathML builder for ordgroup nodes
fn mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::OrdGroup(group) = node else {
        return Err(ParseError::new("Expected OrdGroup node"));
    };

    let body = build_mathml::build_expression_row(ctx, &group.body, options, Some(true))?;
    Ok(body)
}
