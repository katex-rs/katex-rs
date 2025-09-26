//! Operator name function implementation for KaTeX Rust
//!
//! This module handles the \operatorname command, which creates operator names
//! with special formatting and spacing rules for mathematical operators.
//!
//! Migrated from KaTeX's operatorname.js.

use crate::build_common::make_span;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::functions::utils::assemble_sup_sub;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode, make_fragment};
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNode, ParseNodeOperatorName, ParseNodeTextOrd,
};
use crate::types::{ErrorLocationProvider as _, ParseError, ParseErrorKind};
use crate::{KatexContext, build_html, build_mathml};

fn normalize_symbol_text(node: &mut HtmlDomNode) {
    match node {
        HtmlDomNode::Symbol(symbol) => {
            let replaced = symbol
                .text
                .replace('\u{2212}', "-")
                .replace('\u{2217}', "*");
            if symbol.text != replaced {
                symbol.text = replaced;
            }
        }
        HtmlDomNode::DomSpan(span) => {
            for child in &mut span.children {
                normalize_symbol_text(child);
            }
        }
        HtmlDomNode::Anchor(anchor) => {
            for child in &mut anchor.children {
                normalize_symbol_text(child);
            }
        }
        HtmlDomNode::Fragment(fragment) => {
            for child in &mut fragment.children {
                normalize_symbol_text(child);
            }
        }
        _ => {}
    }
}

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
                return Err(ParseError::new(ParseErrorKind::ExpectedSupSubBaseNode {
                    node: NodeType::OperatorName,
                }));
            }
        }
        ParseNode::OperatorName(op_node) => (op_node, None, None, false),
        _ => {
            return Err(ParseError::new(ParseErrorKind::ExpectedNodeOrSupSub {
                node: NodeType::OperatorName,
            }));
        }
    };

    let body: Vec<AnyParseNode> = operatorname_node
        .body
        .iter()
        .map(|child| {
            child.text().map_or_else(
                || child.clone(),
                |text| {
                    AnyParseNode::TextOrd(ParseNodeTextOrd {
                        mode: child.mode(),
                        loc: child.loc().cloned(),
                        text: text.to_owned(),
                    })
                },
            )
        })
        .collect();

    let base = if body.is_empty() {
        make_span(vec!["mop".to_owned()], vec![], Some(options), None)
    } else {
        let mut expression = build_html::build_expression(
            ctx,
            &body,
            &options.with_font("mathrm".to_owned()),
            build_html::GroupType::True,
            (None, None),
        )?;

        for node in &mut expression {
            normalize_symbol_text(node);
        }

        make_span(vec!["mop".to_owned()], expression, Some(options), None)
    };

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
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::OperatorName,
        }));
    };

    let mut expression = build_mathml::build_expression(
        ctx,
        &operatorname_node.body,
        &options.with_font("mathrm".to_owned()),
        None,
    )?;

    let mut is_all_string = true;

    for node in &mut expression {
        match node {
            MathDomNode::Space(_) => {}
            MathDomNode::Math(math_node) => match math_node.node_type {
                MathNodeType::Mi
                | MathNodeType::Mn
                | MathNodeType::Mspace
                | MathNodeType::Mtext => {}
                MathNodeType::Mo => {
                    if math_node.children.len() == 1 {
                        if let MathDomNode::Text(text_node) = &mut math_node.children[0] {
                            let replaced = text_node
                                .text
                                .replace('\u{2212}', "-")
                                .replace('\u{2217}', "*");
                            if text_node.text != replaced {
                                text_node.text = replaced;
                            }
                        } else {
                            is_all_string = false;
                        }
                    } else {
                        is_all_string = false;
                    }
                }
                _ => {
                    is_all_string = false;
                }
            },
            _ => {
                is_all_string = false;
            }
        }
    }

    if is_all_string {
        let word: String = expression.iter().map(MathDomNode::to_text).collect();
        expression = vec![MathDomNode::Text(TextNode { text: word })];
    }

    let mut identifier = MathNode::builder()
        .node_type(MathNodeType::Mi)
        .children(expression)
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

    let identifier_dom = MathDomNode::Math(identifier);
    let operator_dom = MathDomNode::Math(operator);

    if operatorname_node.parent_is_sup_sub {
        Ok(MathDomNode::Math(
            MathNode::builder()
                .node_type(MathNodeType::Mrow)
                .children(vec![identifier_dom, operator_dom])
                .build(),
        ))
    } else {
        Ok(MathDomNode::Fragment(Box::new(make_fragment(vec![
            identifier_dom,
            operator_dom,
        ]))))
    }
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
        handler: Some(|context, args, _opt_args| {
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
