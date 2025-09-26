//! Math class function implementations for KaTeX Rust
//!
//! This module handles math class commands in mathematical expressions,
//! migrated from KaTeX's mclass.js.

use crate::namespace::KeyMap;

use crate::build_common::make_span;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNode, ParseNodeMclass, ParseNodeOp, ParseNodeSupSub,
};
use crate::symbols::Atom;
use crate::types::{ParseError, ParseErrorKind};
use crate::{KatexContext, build_html, build_mathml};

/// Determines the math class for binrel spacing based on the argument node.
///
/// This function analyzes the type of the argument node and returns the
/// appropriate math class ("mbin", "mrel", or "mord") for spacing purposes.
///
/// # Arguments
///
/// * `arg` - The parse node to analyze
///
/// # Returns
///
/// The math class string ("mbin", "mrel", or "mord")
pub fn binrel_class(arg: &AnyParseNode) -> String {
    // \binrel@ spacing varies with (bin|rel|ord) of the atom in the argument.
    // (by rendering separately and with {}s before and after, and measuring
    // the change in spacing).  We'll do roughly the same by detecting the
    // atom type directly.
    let atom = match arg {
        AnyParseNode::OrdGroup(ord) if !ord.body.is_empty() => &ord.body[0],
        _ => arg,
    };

    match atom {
        AnyParseNode::Atom(atom_node) => {
            if atom_node.family == Atom::Bin || atom_node.family == Atom::Rel {
                format!("m{}", atom_node.family.as_ref())
            } else {
                "mord".to_owned()
            }
        }
        _ => "mord".to_owned(),
    }
}

/// HTML builder for mclass nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Mclass(mclass_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Mclass,
        }));
    };

    let elements = build_html::build_expression(
        ctx,
        &mclass_node.body,
        options,
        build_html::GroupType::True,
        (None, None),
    )?;
    Ok(make_span(
        vec![mclass_node.mclass.clone()],
        elements,
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for mclass nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Mclass(mclass_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Mclass,
        }));
    };

    let inner = build_mathml::build_expression(ctx, &mclass_node.body, options, None)?;

    let node_result = if mclass_node.mclass == "minner" {
        MathNode::builder()
            .node_type(MathNodeType::Mpadded)
            .children(inner)
            .build()
            .into()
    } else if mclass_node.mclass == "mord" {
        if mclass_node.is_character_box {
            // Use the first inner element directly
            if inner.is_empty() {
                MathNode::builder()
                    .node_type(MathNodeType::Mi)
                    .children(inner)
                    .build()
                    .into()
            } else {
                inner[0].clone()
            }
        } else {
            MathNode::builder()
                .node_type(MathNodeType::Mi)
                .children(inner)
                .build()
                .into()
        }
    } else {
        let mut node = if mclass_node.is_character_box {
            // Use the first inner element directly
            if inner.is_empty() {
                MathNode::builder()
                    .node_type(MathNodeType::Mo)
                    .children(inner)
                    .build()
                    .into()
            } else {
                inner[0].clone()
            }
        } else {
            MathNode::builder()
                .node_type(MathNodeType::Mo)
                .children(inner)
                .build()
                .into()
        };

        // Set spacing based on what is the most likely adjacent atom type.
        // See TeXbook p170.
        if let MathDomNode::Math(math_node) = &mut node {
            let mut attributes = KeyMap::default();

            if mclass_node.mclass == "mbin" {
                attributes.insert("lspace".to_owned(), "0.22em".to_owned()); // medium space
                attributes.insert("rspace".to_owned(), "0.22em".to_owned());
            } else if mclass_node.mclass == "mpunct" {
                attributes.insert("lspace".to_owned(), "0em".to_owned());
                attributes.insert("rspace".to_owned(), "0.17em".to_owned()); // thinspace
            } else if mclass_node.mclass == "mopen" || mclass_node.mclass == "mclose" {
                attributes.insert("lspace".to_owned(), "0em".to_owned());
                attributes.insert("rspace".to_owned(), "0em".to_owned());
            } else if mclass_node.mclass == "minner" {
                attributes.insert("lspace".to_owned(), "0.0556em".to_owned()); // 1 mu is the most likely option
                attributes.insert("width".to_owned(), "+0.1111em".to_owned());
            }
            // MathML <mo> default space is 5/18 em, so <mrel> needs no action.
            // Ref: https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mo

            if !attributes.is_empty() {
                math_node.attributes = attributes;
            }
        }

        node
    };

    Ok(node_result)
}

/// Registers mclass functions in the KaTeX context
pub fn define_mclass(ctx: &mut crate::KatexContext) {
    // Math class commands except \mathop
    let math_class_names = [
        "\\mathord",
        "\\mathbin",
        "\\mathrel",
        "\\mathopen",
        "\\mathclose",
        "\\mathpunct",
        "\\mathinner",
    ];

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Mclass),
        names: &math_class_names,
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let body = &args[0];
            let func_name = context.func_name;

            // Extract mclass from function name: \mathord -> mord, etc.
            let mclass = if func_name == "\\mathord" {
                "mord"
            } else if func_name == "\\mathbin" {
                "mbin"
            } else if func_name == "\\mathrel" {
                "mrel"
            } else if func_name == "\\mathopen" {
                "mopen"
            } else if func_name == "\\mathclose" {
                "mclose"
            } else if func_name == "\\mathpunct" {
                "mpunct"
            } else if func_name == "\\mathinner" {
                "minner"
            } else {
                "mord" // fallback
            };

            Ok(ParseNode::Mclass(ParseNodeMclass {
                mode: context.parser.mode,
                loc: context.loc(),
                mclass: mclass.to_owned(),
                body: ord_argument(body),
                is_character_box: body.is_character_box()?,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \@binrel{x}{y} renders like y but as mbin/mrel/mord if x is mbin/mrel/mord.
    // This is equivalent to \binrel@{x}\binrel@@{y} in AMSTeX.
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Mclass),
        names: &["\\@binrel"],
        props: FunctionPropSpec {
            num_args: 2,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let mclass = binrel_class(&args[0]);

            Ok(ParseNode::Mclass(ParseNodeMclass {
                mode: context.parser.mode,
                loc: context.loc(),
                mclass,
                body: ord_argument(&args[1]),
                is_character_box: args[1].is_character_box()?,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // Build a relation or stacked op by placing one symbol on top of another
    let stacked_names = ["\\stackrel", "\\overset", "\\underset"];

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Mclass),
        names: &stacked_names,
        props: FunctionPropSpec {
            num_args: 2,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let base_arg = &args[1];
            let shifted_arg = &args[0];
            let func_name = context.func_name;

            let mclass = if func_name == "\\stackrel" {
                "mrel".to_owned() // for \stackrel
            } else {
                // LaTeX applies \binrel spacing to \overset and \underset.
                binrel_class(base_arg)
            };

            let base_op = ParseNodeOp::Body {
                mode: base_arg.mode(),
                loc: context.loc(),
                limits: true,
                always_handle_sup_sub: Some(true),
                suppress_base_shift: Some(func_name != "\\stackrel"),
                parent_is_sup_sub: false,
                body: ord_argument(base_arg),
            };

            let supsub = ParseNodeSupSub {
                mode: shifted_arg.mode(),
                loc: context.loc(),
                base: Some(Box::new(ParseNode::Op(base_op))),
                sup: if func_name == "\\underset" {
                    None
                } else {
                    Some(Box::new(shifted_arg.clone()))
                },
                sub: (func_name == "\\underset").then(|| Box::new(shifted_arg.clone())),
            };

            let supsub = ParseNode::SupSub(supsub);

            Ok(ParseNode::Mclass(ParseNodeMclass {
                mode: context.parser.mode,
                loc: context.loc(),
                mclass,
                is_character_box: supsub.is_character_box()?,
                body: vec![supsub],
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
