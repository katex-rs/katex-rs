//! Phantom function implementations for KaTeX Rust
//!
//! This module handles phantom commands (\phantom, \hphantom, \vphantom) in
//! mathematical expressions, migrated from KaTeX's phantom.js.

use crate::build_common::{
    VListChild, VListElem, VListParam, make_fragment, make_span, make_v_list,
};
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNodeHphantom, ParseNodePhantom, ParseNodeVphantom,
};
use crate::types::ParseError;
use crate::{build_html, build_mathml};

/// Registers phantom functions in the KaTeX context
pub fn define_phantom(ctx: &mut crate::KatexContext) {
    // \phantom - Complete invisible placeholder
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Phantom),
        names: &["\\phantom"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = args[0].clone();
            Ok(AnyParseNode::Phantom(ParseNodePhantom {
                mode: context.parser.mode,
                loc: context.loc(),
                body: ord_argument(&body),
            }))
        }),
        html_builder: Some(html_builder_phantom),
        mathml_builder: Some(mathml_builder_phantom),
    });

    // \hphantom - Horizontal phantom (only width, no height)
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Hphantom),
        names: &["\\hphantom"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = args[0].clone();
            Ok(AnyParseNode::Hphantom(ParseNodeHphantom {
                mode: context.parser.mode,
                loc: context.loc(),
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder_hphantom),
        mathml_builder: Some(mathml_builder_hphantom),
    });

    // \vphantom - Vertical phantom (only height, no width)
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Vphantom),
        names: &["\\vphantom"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = args[0].clone();
            Ok(AnyParseNode::Vphantom(ParseNodeVphantom {
                mode: context.parser.mode,
                loc: context.loc(),
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder_vphantom),
        mathml_builder: Some(mathml_builder_vphantom),
    });
}

/// HTML builder for \phantom nodes
fn html_builder_phantom(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::Phantom(phantom_node) = node else {
        return Err(ParseError::new("Expected Phantom node"));
    };

    // Build the expression with phantom options
    let phantom_options = options.with_phantom();
    let elements = build_html::build_expression(
        ctx,
        &phantom_node.body,
        &phantom_options,
        build_html::GroupType::False,
        (None, None),
    )?;

    // \phantom isn't supposed to affect the elements it contains.
    // See "color" for more details.
    Ok(make_fragment(&elements).into())
}

/// HTML builder for \hphantom nodes
fn html_builder_hphantom(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::Hphantom(hphantom_node) = node else {
        return Err(ParseError::new("Expected Hphantom node"));
    };

    let phantom_options = options.with_phantom();
    let inner = build_html::build_group(ctx, &hphantom_node.body, &phantom_options, None)?;
    let mut node_span = make_span(vec![], vec![inner], None, None);
    node_span.height = 0.0;
    node_span.depth = 0.0;

    // Set height and depth to 0 for all children
    for child in &mut node_span.children {
        if let Some(h) = child.height_mut() {
            *h = 0.0;
        }
        if let Some(d) = child.depth_mut() {
            *d = 0.0;
        }
    }

    // Use makeVList for spacing consistency
    let vlist = make_v_list(
        VListParam::FirstBaseline {
            children: vec![VListChild::Elem(
                VListElem::builder().elem(node_span.into()).build().into(),
            )],
        },
        options,
    )?;

    // For spacing, TeX treats \hphantom as a math group (same spacing as ord).
    Ok(make_span(
        vec!["mord".to_owned()],
        vec![vlist.into()],
        Some(options),
        None,
    )
    .into())
}

/// HTML builder for \vphantom nodes
fn html_builder_vphantom(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::Vphantom(vphantom_node) = node else {
        return Err(ParseError::new("Expected Vphantom node"));
    };

    let phantom_options = options.with_phantom();
    let inner = build_html::build_group(ctx, &vphantom_node.body, &phantom_options, None)?;
    let inner_span = make_span(vec!["inner".to_owned()], vec![inner], None, None);
    let fix = make_span(vec!["fix".to_owned()], vec![], None, None);

    Ok(make_span(
        vec!["mord".to_owned(), "rlap".to_owned()],
        vec![inner_span.into(), fix.into()],
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for \phantom nodes
fn mathml_builder_phantom(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::Phantom(phantom_node) = node else {
        return Err(ParseError::new("Expected Phantom node"));
    };

    let inner = build_mathml::build_expression(ctx, &phantom_node.body, options, None)?;
    Ok(MathNode::builder()
        .node_type(MathNodeType::Mphantom)
        .children(inner)
        .build()
        .into())
}

/// MathML builder for \hphantom nodes
fn mathml_builder_hphantom(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::Hphantom(hphantom_node) = node else {
        return Err(ParseError::new("Expected Hphantom node"));
    };

    let inner =
        build_mathml::build_expression(ctx, &ord_argument(&hphantom_node.body), options, None)?;
    let phantom = MathNode::builder()
        .node_type(MathNodeType::Mphantom)
        .children(inner)
        .build();

    let mut node = MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .children(vec![MathDomNode::Math(phantom)])
        .build();

    node.attributes.extend([
        ("height".to_owned(), "0px".to_owned()),
        ("depth".to_owned(), "0px".to_owned()),
    ]);

    Ok(MathDomNode::Math(node))
}

/// MathML builder for \vphantom nodes
fn mathml_builder_vphantom(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::Vphantom(vphantom_node) = node else {
        return Err(ParseError::new("Expected Vphantom node"));
    };

    let inner =
        build_mathml::build_expression(ctx, &ord_argument(&vphantom_node.body), options, None)?;
    let phantom = MathNode::builder()
        .node_type(MathNodeType::Mphantom)
        .children(inner)
        .build();

    let mut node = MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .children(vec![MathDomNode::Math(phantom)])
        .build();

    node.attributes.insert("width".to_owned(), "0px".to_owned());

    Ok(MathDomNode::Math(node))
}
