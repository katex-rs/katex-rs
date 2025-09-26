//! Styling function implementations for KaTeX Rust
//!
//! This module handles style change commands in mathematical expressions,
//! migrated from KaTeX's styling.js.

use crate::build_mathml;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::functions::sizing::sizing_group;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeStyling};
use crate::style::{DISPLAY, SCRIPT, SCRIPTSCRIPT, Style, TEXT};
use crate::types::{ParseError, ParseErrorKind};

/// Style mapping from string names to Style references
fn style_map(style_name: &str) -> &'static Style {
    match style_name {
        "display" => DISPLAY,
        "text" => TEXT,
        "script" => SCRIPT,
        "scriptscript" => SCRIPTSCRIPT,
        _ => unreachable!(),
    }
}

/// HTML builder for styling nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Styling(styling_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Styling,
        }));
    };

    // Style changes are handled in the TeXbook on pg. 442, Rule 3.
    let new_options = options
        .having_style(styling_node.style)
        .with_font(String::new());
    sizing_group(ctx, &styling_node.body, &new_options, options)
}

const STYLE_ATTRIBUTES: [(&Style, (u8, bool)); 4] = [
    (DISPLAY, (0, true)),
    (TEXT, (0, false)),
    (SCRIPT, (1, false)),
    (SCRIPTSCRIPT, (2, false)),
];

/// MathML builder for styling nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Styling(styling_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Styling,
        }));
    };

    // Figure out what style we're changing to.
    let new_options = options.having_style(styling_node.style);

    let inner = build_mathml::build_expression(ctx, &styling_node.body, &new_options, None)?;

    let mut node = MathNode::builder()
        .node_type(MathNodeType::Mstyle)
        .children(inner)
        .build();

    // Find the matching style attributes
    for (style_ref, attrs) in STYLE_ATTRIBUTES {
        if styling_node.style == style_ref {
            node.attributes
                .insert("scriptlevel".to_owned(), attrs.0.to_string());
            node.attributes
                .insert("displaystyle".to_owned(), attrs.1.to_string());
            break;
        }
    }

    Ok(MathDomNode::Math(node))
}

/// Registers styling functions in the KaTeX context
pub fn define_styling(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Styling),
        names: &[
            "\\displaystyle",
            "\\textstyle",
            "\\scriptstyle",
            "\\scriptscriptstyle",
        ],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            if !args.is_empty() {
                return Err(ParseError::new(ParseErrorKind::StylingTakesNoArguments));
            }

            // Parse out the implicit body
            let body = context
                .parser
                .parse_expression(true, context.break_on_token_text)?;

            // Figure out what style to use by pulling out the style from
            // the function name
            let style_name = &context.func_name[1..context.func_name.len() - 5]; // Remove \ and style
            let style = style_map(style_name);

            Ok(ParseNode::Styling(ParseNodeStyling {
                mode: context.parser.mode,
                loc: context.loc(),
                style,
                body,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
