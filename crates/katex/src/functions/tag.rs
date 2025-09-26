//! Tag function implementations for KaTeX Rust
//!
//! This module handles tag commands in mathematical expressions,
//! migrated from KaTeX's tag.js.

use crate::build_mathml;
use crate::context::KatexContext;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType};
use crate::types::ParseError;

/// Registers tag functions in the KaTeX context
pub fn define_tag(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Tag),
        names: &["\\tag"],
        props: FunctionPropSpec::default(),
        handler: None,
        html_builder: None, // Tag only has MathML builder
        mathml_builder: Some(mathml_builder),
    });
}

/// MathML builder for tag nodes
fn mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::Tag(group) = node else {
        return Err(ParseError::new("Expected Tag node"));
    };

    // Create padding cell
    let mut pad = MathNode::builder().node_type(MathNodeType::Mtd).build();

    pad.attributes.insert("width".to_owned(), "50%".to_owned());

    // Build body expression
    let body = build_mathml::build_expression_row(ctx, &group.body, options, None)?;

    // Build tag expression
    let tag = build_mathml::build_expression_row(ctx, &group.tag, options, None)?;

    // Create body cell
    let body_cell = MathNode::builder()
        .node_type(MathNodeType::Mtd)
        .children(vec![body])
        .build();

    // Create tag cell
    let tag_cell = MathNode::builder()
        .node_type(MathNodeType::Mtd)
        .children(vec![tag])
        .build();

    // Create table row
    let row = MathNode::builder()
        .node_type(MathNodeType::Mtr)
        .children(vec![
            MathDomNode::Math(pad.clone()),
            MathDomNode::Math(body_cell),
            MathDomNode::Math(pad),
            MathDomNode::Math(tag_cell),
        ])
        .build();

    // Create table
    let mut table = MathNode::builder()
        .node_type(MathNodeType::Mtable)
        .children(vec![MathDomNode::Math(row)])
        .build();

    table
        .attributes
        .insert("width".to_owned(), "100%".to_owned());

    Ok(MathDomNode::Math(table))
}
