//! Implementation of spacing symbols in KaTeX
//!
//! This module handles the rendering of spacing commands like `\ `, `~`,
//! `\nobreak`, `\allowbreak`, etc. It provides both HTML and MathML builders
//! for spacing elements.

use crate::build_common::{make_ord, make_span, mathsym};
use crate::context::KatexContext;
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode};
use crate::types::{Mode, ParseError, ParseErrorKind};
use alloc::borrow::Cow;
use phf::phf_map;

/// CSS-based spacing functions mapped to their CSS class names
static CSS_SPACE: phf::Map<&'static str, &'static str> = phf_map! {
    "\\nobreak" => "nobreak",
    "\\allowbreak" => "allowbreak",
};

/// Regular space functions with optional CSS class names
static REGULAR_SPACE: phf::Map<&'static str, Option<&'static str>> = phf_map! {
    " " => None,
    "\\ " => None,
    "~" => Some("nobreak"),
    "\\space" => None,
    "\\nobreakspace" => Some("nobreak"),
};

/// HTML builder for spacing elements
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Spacing(spacing_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Spacing,
        }));
    };
    REGULAR_SPACE.get(spacing_node.text.as_str()).map_or_else(
        || {
            CSS_SPACE.get(spacing_node.text.as_str()).map_or_else(
                || {
                    Err(ParseError::new(ParseErrorKind::UnknownSpaceType {
                        name: spacing_node.text.to_string(),
                    }))
                },
                |&class_name| {
                    // CSS-based spacing
                    let classes = vec![Cow::Borrowed("mspace"), Cow::Owned(class_name.to_owned())];
                    Ok(make_span(classes, vec![], Some(options), None).into())
                },
            )
        },
        |class_name_opt| {
            let class_name = class_name_opt;
            let mut mspace_classes = vec![Cow::Borrowed("mspace")];
            if let Some(cn) = &class_name {
                mspace_classes.push(Cow::Owned((*cn).to_owned()));
            }

            if spacing_node.mode == Mode::Text {
                let mut ord = make_ord(ctx, &ParseNode::Spacing(spacing_node.clone()), options)?;
                if let Some(classes) = ord.classes_mut() {
                    if let Some(cn) = class_name {
                        classes.push(Cow::Owned((*cn).to_owned()));
                    }
                } else {
                    return Err(ParseError::new(ParseErrorKind::GeneratedOrdMissingClasses));
                }
                Ok(ord)
            } else {
                // In math mode, create a span with the symbol
                let symbol = mathsym(
                    ctx,
                    &spacing_node.text,
                    Mode::Math,
                    options,
                    crate::ClassList::Empty,
                )?;
                Ok(make_span(mspace_classes, vec![symbol.into()], Some(options), None).into())
            }
        },
    )
}

/// MathML builder for spacing elements
fn mathml_builder(
    node: &ParseNode,
    _options: &Options,
    _ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Spacing(spacing_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Spacing,
        }));
    };

    if REGULAR_SPACE.contains_key(&spacing_node.text) {
        // Regular spaces use mtext with non-breaking space
        Ok(MathNode::builder()
            .node_type(MathNodeType::Mtext)
            .children(vec![MathDomNode::Text(TextNode {
                text: "\u{00a0}".to_owned(), // Non-breaking space
            })])
            .build()
            .into())
    } else if CSS_SPACE.contains_key(&spacing_node.text) {
        // CSS-based spaces use mspace (ignored in MathML)
        Ok(MathNode::builder()
            .node_type(MathNodeType::Mspace)
            .build()
            .into())
    } else {
        Err(ParseError::new(ParseErrorKind::UnknownSpaceType {
            name: spacing_node.text.to_string(),
        }))
    }
}

/// Register the spacing builders in the KaTeX context
pub fn define_spacing(ctx: &mut KatexContext) {
    ctx.define_function_builders(NodeType::Spacing, Some(html_builder), Some(mathml_builder));
}
