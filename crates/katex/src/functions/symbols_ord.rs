//! Symbol ordinal function implementations for KaTeX Rust
//!
//! This module handles mathord and textord symbols, migrated from KaTeX's
//! symbolsOrd.js.

use crate::ParseError;
use crate::build_common::make_ord;
use crate::build_mathml::{get_variant, make_text};
use crate::context::KatexContext;
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType};
use crate::types::Mode;
use phf::phf_map;

/// Default variant mapping for MathML elements
/// Equivalent to defaultVariant in the original JavaScript
static DEFAULT_VARIANT: phf::Map<&'static str, &'static str> = phf_map! {
    "mi" => "italic",
    "mn" => "normal",
    "mtext" => "normal",
};

/// Registers mathord and textord functions in the KaTeX context
pub fn define_symbols_ord(ctx: &mut KatexContext) {
    // Register mathord
    ctx.define_function_builders(
        NodeType::MathOrd,
        Some(ord_html_builder),
        Some(mathord_mathml_builder),
    );

    // Register textord
    ctx.define_function_builders(
        NodeType::TextOrd,
        Some(ord_html_builder),
        Some(textord_mathml_builder),
    );
}

/// HTML builder for textord nodes
fn ord_html_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    make_ord(ctx, node, options)
}

/// MathML builder for mathord nodes
fn mathord_mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::MathOrd(group) = node else {
        return Err(ParseError::new("Expected MathOrd node"));
    };
    let mut mi_node = MathNode::builder()
        .node_type(MathNodeType::Mi)
        .children(vec![MathDomNode::Text(make_text(
            &group.text,
            group.mode,
            Some(options),
            &ctx.symbols,
        ))])
        .build();

    let variant = get_variant(ctx, node, options)?.unwrap_or("italic");
    if Some(&variant) != DEFAULT_VARIANT.get("mi") {
        mi_node.set_attribute("mathvariant".to_owned(), variant.to_owned());
    }

    Ok(MathDomNode::Math(mi_node))
}

/// MathML builder for textord nodes
fn textord_mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::TextOrd(group) = node else {
        return Err(ParseError::new("Expected TextOrd node"));
    };

    let text = make_text(&group.text, group.mode, Some(options), &ctx.symbols);
    let variant = get_variant(ctx, node, options)?.unwrap_or("normal");

    let element_type = if group.mode == Mode::Text {
        MathNodeType::Mtext
    } else if group.text.chars().any(|c| c.is_ascii_digit()) {
        MathNodeType::Mn
    } else if group.text == "\\prime" {
        MathNodeType::Mo
    } else {
        MathNodeType::Mi
    };

    let mut node = MathNode::builder()
        .node_type(element_type)
        .children(vec![MathDomNode::Text(text)])
        .build();

    if Some(&variant) != DEFAULT_VARIANT.get(element_type.as_ref()) {
        node.set_attribute("mathvariant".to_owned(), variant.to_owned());
    }

    Ok(MathDomNode::Math(node))
}
