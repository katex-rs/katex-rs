//! HTML extension function implementations for KaTeX Rust
//!
//! This module handles HTML extension commands like \htmlClass, \htmlId,
//! \htmlStyle, \htmlData migrated from KaTeX's html.js.

use crate::namespace::KeyMap;

use crate::build_common::make_span;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::MathDomNode;
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeHtml};
use crate::types::{ArgType, ErrorLocationProvider, ParseError, ParseErrorKind};
use crate::{ClassList, KatexContext, TrustContext, build_html, build_mathml};

/// HTML extension command names
const HTML_COMMANDS: &[&str] = &["\\htmlClass", "\\htmlId", "\\htmlStyle", "\\htmlData"];

/// Registers HTML extension functions in the KaTeX context
pub fn define_html(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Html),
        names: HTML_COMMANDS,
        props: FunctionPropSpec {
            num_args: 2,
            arg_types: Some(vec![ArgType::Raw, ArgType::Original]),
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let value = match &args[0] {
                ParseNode::Raw(raw) => raw.string.clone(),
                _ => {
                    return Err(ParseError::new(
                        ParseErrorKind::ExpectedRawStringFirstArgument,
                    ));
                }
            };

            let body = args[1].clone();

            // Check strict mode
            context.parser.settings.report_nonstrict(
                "htmlExtension",
                "HTML extension is disabled on strict mode",
                context
                    .token
                    .as_ref()
                    .map(|t| *t as &dyn ErrorLocationProvider),
            )?;

            let mut attributes = KeyMap::default();
            let mut trust_context = match context.func_name {
                "\\htmlClass" => {
                    attributes.insert("class".to_owned(), value.clone());
                    TrustContext {
                        command: "\\htmlClass".to_owned(),
                        class: Some(value),
                        ..Default::default()
                    }
                }
                "\\htmlId" => {
                    attributes.insert("id".to_owned(), value.clone());
                    TrustContext {
                        command: "\\htmlId".to_owned(),
                        id: Some(value),
                        ..Default::default()
                    }
                }
                "\\htmlStyle" => {
                    attributes.insert("style".to_owned(), value.clone());
                    TrustContext {
                        command: "\\htmlStyle".to_owned(),
                        style: Some(value),
                        ..Default::default()
                    }
                }
                "\\htmlData" => {
                    let data_parts: Vec<&str> = value.split(',').collect();
                    for part in data_parts {
                        let key_val: Vec<&str> = part.split('=').collect();
                        if key_val.len() != 2 {
                            return Err(ParseError::new(
                                ParseErrorKind::HtmlDataKeyValueParseError,
                            ));
                        }
                        let key = format!("data-{}", key_val[0].trim());
                        let val = key_val[1].trim().to_owned();
                        attributes.insert(key, val);
                    }
                    TrustContext {
                        command: "\\htmlData".to_owned(),
                        attributes: Some(attributes.clone()),
                        ..Default::default()
                    }
                }
                _ => {
                    return Err(ParseError::new(ParseErrorKind::UnrecognizedHtmlCommand));
                }
            };

            if !context.parser.settings.is_trusted(&mut trust_context) {
                return Ok(context
                    .parser
                    .format_unsupported_cmd(context.func_name)
                    .into());
            }

            Ok(ParseNode::Html(ParseNodeHtml {
                mode: context.parser.mode,
                loc: context.loc(),
                attributes,
                body: ord_argument(&body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for HTML extension nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Html(html_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Html,
        }));
    };

    let elements = build_html::build_expression(
        ctx,
        &html_node.body,
        options,
        build_html::GroupType::False,
        (None, None),
    )?;

    let mut classes = ClassList::Static("enclosing");
    if let Some(class) = html_node.attributes.get("class") {
        classes.extend(class.split_whitespace().map(|s| s.to_owned().into()));
    }

    let mut span = make_span(classes, elements, Some(options), None);

    // Avoid overriding the computed class list. KaTeX.js deliberately skips
    // copying the `class` attribute from the original node because
    // `buildCommon.makeSpan` already encoded the classes (including the
    // `enclosing` helper class). Extending with the raw attribute here would
    // drop that helper class and change layout semantics.
    for (attr, value) in &html_node.attributes {
        if attr == "class" {
            continue;
        }
        span.attributes.insert(attr.clone(), value.clone());
    }

    Ok(span.into())
}

/// MathML builder for HTML extension nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Html(html_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Html,
        }));
    };

    let base_group = build_mathml::build_expression_row(ctx, &html_node.body, options, None)?;
    Ok(base_group)
}
