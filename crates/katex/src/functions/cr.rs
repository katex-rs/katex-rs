//! Implementation of the \\ (line break) function in KaTeX
//!
//! This module implements the LaTeX line break command `\\`, which creates
//! line breaks in mathematical expressions. It handles both tabular
//! environments and top-level line breaks, with optional size specifications.

use crate::macros::MacroContextInterface as _;
use crate::namespace::KeyMap;

use crate::ParseError;
use crate::build_common::make_span;
use crate::context::KatexContext;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeCr};
use crate::types::{CssProperty, ParseErrorKind};
use crate::units::make_em;

/// Register the \\ (line break) function
pub fn define_cr(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Cr),
        names: &["\\\\"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(
            |context, _args: Vec<ParseNode>, _opt_args: Vec<Option<ParseNode>>| {
                // Check if the next token is "[" to parse optional size
                let size = if context.parser.gullet.future_mut()?.text == "[" {
                    context
                        .parser
                        .parse_size_group(true)?
                        .as_ref()
                        .map(|s| s.value.clone())
                } else {
                    None
                };

                // Determine if this should create a new line
                let new_line = !context.parser.settings.display_mode
                    || !context.parser.settings.use_strict_behavior(
                        "newLineInDisplayMode",
                        "In LaTeX, \\\\ or \\newline does nothing in display mode",
                        None,
                    );

                Ok(ParseNode::Cr(ParseNodeCr {
                    mode: context.parser.mode,
                    loc: context.loc(),
                    new_line,
                    size,
                }))
            },
        ),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for the \\ function
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    if let ParseNode::Cr(cr) = node {
        let mut span = make_span("mspace", vec![], Some(options), None);
        if cr.new_line {
            span.classes.push("newline");
            if let Some(size) = &cr.size {
                span.style.insert(
                    CssProperty::MarginTop,
                    make_em(ctx.calculate_size(size, options)?),
                );
            }
        }
        Ok(span.into())
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Cr,
        }))
    }
}

/// MathML builder for the \\ function
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    if let ParseNode::Cr(cr_node) = node {
        let mut attributes = KeyMap::default();

        if cr_node.new_line {
            attributes.insert("linebreak".to_owned(), "newline".to_owned());

            if let Some(size) = &cr_node.size {
                // Use calculate_size to properly convert the measurement to ems
                if let Ok(size_value) = ctx.calculate_size(size, options) {
                    let height = make_em(size_value);
                    attributes.insert("height".to_owned(), height);
                } else {
                    // If calculate_size fails, fall back to a simple calculation
                    attributes.insert("height".to_owned(), make_em(size.number));
                }
            }
        }

        let math_node = MathNode::builder()
            .node_type(MathNodeType::Mspace)
            .attributes(attributes)
            .build();

        Ok(MathDomNode::Math(math_node))
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Cr,
        }))
    }
}
