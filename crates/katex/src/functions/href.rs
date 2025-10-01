//! Href function implementations for KaTeX Rust
//!
//! This module handles hyperlink functions in mathematical expressions,
//! migrated from KaTeX's href.js.

use crate::namespace::KeyMap;

use crate::build_common::make_anchor;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNodeHref, ParseNodeText, ParseNodeTextOrd,
};
use crate::types::{ArgType, ParseError, ParseErrorKind, TrustContext};
use crate::{build_html, build_mathml};

/// Registers href functions in the KaTeX context
pub fn define_href(ctx: &mut crate::KatexContext) {
    // \href command - creates hyperlink with custom text
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Href),
        names: &["\\href"],
        props: FunctionPropSpec {
            num_args: 2,
            arg_types: Some(vec![ArgType::Url, ArgType::Original]),
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let url_node = &args[0];
            let body = args[1].clone();

            // Extract URL from the url node
            let href = match url_node {
                AnyParseNode::Url(url_node) => url_node.url.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ArgumentMustBeUrl {
                        context: "First argument",
                    }));
                }
            };

            let mut trust_ctx = TrustContext {
                command: "\\href".to_owned(),
                url: Some(href.clone()),
                ..Default::default()
            };

            // Check trust settings
            if !context.parser.settings.is_trusted(&mut trust_ctx) {
                return Ok(context.parser.format_unsupported_cmd("\\href").into());
            }

            Ok(AnyParseNode::Href(ParseNodeHref {
                mode: context.parser.mode,
                loc: context.loc(),
                href,
                body: vec![body],
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \url command - creates hyperlink from URL text
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Href),
        names: &["\\url"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Url]),
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let url_node = &args[0];

            // Extract URL from the url node
            let href = match url_node {
                AnyParseNode::Url(url_node) => url_node.url.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ArgumentMustBeUrl {
                        context: "Argument",
                    }));
                }
            };

            let mut trust_ctx = TrustContext {
                command: "\\url".to_owned(),
                url: Some(href.clone()),
                ..Default::default()
            };

            // Check trust settings
            if !context.parser.settings.is_trusted(&mut trust_ctx) {
                return Ok(context.parser.format_unsupported_cmd("\\url").into());
            }

            // Process URL characters, replacing ~ with \textasciitilde
            let mut chars = Vec::new();
            for ch in href.chars() {
                if ch == '~' {
                    // Create \textasciitilde node
                    chars.push(AnyParseNode::Text(ParseNodeText {
                        mode: context.parser.mode,
                        loc: context.loc(),
                        body: vec![AnyParseNode::TextOrd(ParseNodeTextOrd {
                            mode: context.parser.mode,
                            loc: context.loc(),
                            text: "\\textasciitilde".to_owned(),
                        })],
                        font: None,
                    }));
                } else {
                    chars.push(AnyParseNode::TextOrd(ParseNodeTextOrd {
                        mode: context.parser.mode,
                        loc: context.loc(),
                        text: ch.to_string(),
                    }));
                }
            }

            // Create text node with monospace font
            let body = AnyParseNode::Text(ParseNodeText {
                mode: context.parser.mode,
                loc: context.loc(),
                body: chars,
                font: Some("\\texttt".to_owned()),
            });

            Ok(AnyParseNode::Href(ParseNodeHref {
                mode: context.parser.mode,
                loc: context.loc(),
                href,
                body: vec![body],
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for href nodes
fn html_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let AnyParseNode::Href(href_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Href,
        }));
    };

    // Build the body content
    let mut body_elements = Vec::new();
    for child in &href_node.body {
        body_elements.push(build_html::build_group(ctx, child, options, None)?);
    }

    // Create anchor element
    Ok(make_anchor(&href_node.href, vec![], body_elements, options).into())
}

/// MathML builder for href nodes
fn mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let AnyParseNode::Href(href_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Href,
        }));
    };

    // Build the body content
    let mut body_elements = Vec::new();
    for child in &href_node.body {
        body_elements.push(build_mathml::build_group(ctx, child, options)?);
    }

    // If there's only one element and it's already a MathNode, use it directly
    let math_node = if body_elements.len() == 1 {
        match &body_elements[0] {
            MathDomNode::Math(math) => math.clone(),
            _ => MathNode::builder()
                .node_type(MathNodeType::Mrow)
                .children(body_elements)
                .build(),
        }
    } else {
        MathNode::builder()
            .node_type(MathNodeType::Mrow)
            .children(body_elements)
            .build()
    };

    // Set href attribute
    let mut attributes = KeyMap::default();
    attributes.insert("href".to_owned(), href_node.href.clone());

    Ok(MathDomNode::Math(
        MathNode::builder()
            .node_type(math_node.node_type)
            .children(math_node.children)
            .attributes(attributes)
            .build(),
    ))
}
