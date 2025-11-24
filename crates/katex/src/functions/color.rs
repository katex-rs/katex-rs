//! Color function implementations for KaTeX Rust
//!
//! This module handles color commands in mathematical expressions,
//! migrated from KaTeX's color.js.

use crate::build_common::make_fragment;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::macros::{MacroContextInterface as _, MacroDefinition};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeColor};
use crate::types::{ArgType, ParseError, ParseErrorKind};
use crate::{KatexContext, build_html, build_mathml};

/// Registers color functions in the KaTeX context
pub fn define_color(ctx: &mut KatexContext) {
    // \textcolor command
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Color),
        names: &["\\textcolor"],
        props: FunctionPropSpec {
            num_args: 2,
            allowed_in_text: true,
            arg_types: Some(vec![ArgType::Color, ArgType::Original]),
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let color_node = args[0].clone();
            let body = args[1].clone();

            // Extract color from color-token
            let color = match &color_node {
                ParseNode::ColorToken(token) => token.color.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ExpectedColorToken {
                        argument: "color argument",
                    }));
                }
            };

            // Extract body as AnyParseNode vector
            let body_nodes = match body {
                ParseNode::OrdGroup(group) => group.body,
                _ => vec![body],
            };

            Ok(ParseNode::Color(ParseNodeColor {
                mode: context.parser.mode,
                loc: context.loc(),
                color: color.to_string(),
                body: body_nodes,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \color command
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Color),
        names: &["\\color"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            arg_types: Some(vec![ArgType::Color]),
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let color_node = args[0].clone();

            // Extract color from color-token
            let color = match &color_node {
                ParseNode::ColorToken(token) => token.color.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ExpectedColorToken {
                        argument: "color argument",
                    }));
                }
            };

            // Set macro \current@color in current namespace
            // This mimics the behavior of color.sty
            context.parser.gullet.macros_mut().set(
                "\\current@color",
                Some(MacroDefinition::String(color.to_string())),
                false,
            );

            // Parse out the implicit body that should be colored
            let body = context
                .parser
                .parse_expression(true, context.break_on_token_text)?;

            Ok(ParseNode::Color(ParseNodeColor {
                mode: context.parser.mode,
                loc: context.loc(),
                color: color.to_string(),
                body,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for color nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Color(color_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Color,
        }));
    };

    // Build the expression with the specified color
    let colored_options = options.with_color(&color_node.color);
    let elements = build_html::build_expression(
        ctx,
        &color_node.body,
        &colored_options,
        build_html::GroupType::False,
        (None, None),
    )?;

    // \color isn't supposed to affect the type of the elements it contains.
    // To accomplish this, we wrap the results in a fragment, so the inner
    // elements will be able to directly interact with their neighbors.
    // For example, `\color{red}{2 +} 3` has the same spacing as `2 + 3`
    Ok(make_fragment(elements).into())
}

/// MathML builder for color nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Color(color_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Color,
        }));
    };

    let inner = build_mathml::build_expression(
        ctx,
        &color_node.body,
        &options.with_color(&color_node.color),
        None,
    )?;

    let mut node = MathNode::builder()
        .node_type(MathNodeType::Mstyle)
        .children(inner)
        .build();

    node.set_attribute("mathcolor", color_node.color.clone());

    Ok(MathDomNode::Math(node))
}
