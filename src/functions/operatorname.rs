//! Operator name function implementation for KaTeX Rust
//!
//! This module handles the \operatorname command, which creates operator names
//! with special formatting and spacing rules for mathematical operators.
//!
//! Migrated from KaTeX's operatorname.js.

use crate::build_common::make_span;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::functions::utils::assemble_sup_sub;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeOperatorName};
use crate::types::ParseError;
use crate::{KatexContext, build_html, build_mathml};

/// HTML builder for operatorname nodes
/// NOTE: Unlike most `htmlBuilder`s, this one handles not only
/// "operatorname", but also "supsub" since \operatorname* can
/// affect super/subscripting.
pub fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    // Operators are handled in the TeXbook pg. 443-444, rule 13(a).
    let (operatorname_node, super_group, sub_group, has_limits) = match node {
        ParseNode::SupSub(supsub) => {
            // If we have limits, supsub will pass us its group to handle. Pull
            // out the superscript and subscript and set the group to the op in
            // its base.
            let sub_group = supsub.sub.as_deref();
            let super_group = supsub.sup.as_deref();
            if let Some(base) = supsub.base.as_ref()
                && let ParseNode::OperatorName(op_node) = &**base
            {
                (op_node, super_group, sub_group, true)
            } else {
                return Err(ParseError::new("Expected OperatorName node in SupSub base"));
            }
        }
        ParseNode::OperatorName(op_node) => (op_node, None, None, false),
        _ => return Err(ParseError::new("Expected OperatorName or SupSub node")),
    };

    // Build expression with mathrm font
    let expression = build_html::build_expression(
        ctx,
        &operatorname_node.body,
        &options.with_text_font_family("mathrm".to_owned()),
        build_html::GroupType::True,
        (None, None),
    )?;

    let base = make_span(vec!["mop".to_owned()], expression, Some(options), None);

    if has_limits {
        assemble_sup_sub(
            ctx,
            base.into(),
            super_group,
            sub_group,
            options,
            options.style,
            0.0,
            0.0,
        )
    } else {
        Ok(base.into())
    }
}

/// MathML builder for operatorname nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::OperatorName(operatorname_node) = node else {
        return Err(ParseError::new("Expected OperatorName node"));
    };

    let expression = build_mathml::build_expression_row(
        ctx,
        &operatorname_node.body,
        &options.with_text_font_family("mathrm".to_owned()),
        None,
    )?;

    let mut identifier = MathNode::builder()
        .node_type(MathNodeType::Mi)
        .children(vec![expression])
        .build();

    identifier
        .attributes
        .insert("mathvariant".to_owned(), "normal".to_owned());

    // \u2061 is the same as &ApplyFunction;
    let operator = MathNode::builder()
        .node_type(MathNodeType::Mo)
        .children(vec![MathDomNode::Text(TextNode {
            text: "\u{2061}".to_owned(),
        })])
        .build();

    Ok(MathDomNode::Math(
        MathNode::builder()
            .node_type(MathNodeType::Mrow)
            .children(vec![
                MathDomNode::Math(identifier),
                MathDomNode::Math(operator),
            ])
            .build(),
    ))
}

/// Registers the \operatorname function in the KaTeX context
pub fn define_operatorname(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::OperatorName),
        names: &["\\operatorname@", "\\operatornamewithlimits"],
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = ord_argument(&args[0]);
            let always_handle_sup_sub = context.func_name == "\\operatornamewithlimits";

            Ok(ParseNode::OperatorName(ParseNodeOperatorName {
                mode: context.parser.mode,
                loc: context.loc(),
                body,
                always_handle_sup_sub,
                limits: false,
                parent_is_sup_sub: false,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
