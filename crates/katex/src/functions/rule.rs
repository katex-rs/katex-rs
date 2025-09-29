//! Implementation of the \rule command in KaTeX
//!
//! This module implements the LaTeX `\rule` command, which creates horizontal
//! or vertical rules (lines) with specified width and height.

use crate::ClassList;
use crate::context::KatexContext;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::{HtmlDomNode, Span};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeRule};
use crate::types::{ArgType, CssProperty, CssStyle, ParseError, ParseErrorKind};
use crate::units::make_em;

/// Register the \rule function in the KaTeX context.
pub fn define_rule(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Rule),
        names: &["\\rule"],
        props: FunctionPropSpec {
            num_args: 2,
            arg_types: Some(vec![ArgType::Size, ArgType::Size, ArgType::Size]),
            num_optional_args: 1,
            allowed_in_text: true,
            allowed_in_math: true,
            ..Default::default()
        },
        handler: Some(
            |context, args: Vec<ParseNode>, opt_args: Vec<Option<ParseNode>>| {
                // Extract the width argument
                let AnyParseNode::Size(width_node) = &args[0] else {
                    return Err(ParseError::new(ParseErrorKind::ExpectedSizeArgumentFor {
                        context: "width",
                    }));
                };

                // Extract the height argument
                let AnyParseNode::Size(height_node) = &args[1] else {
                    return Err(ParseError::new(ParseErrorKind::ExpectedSizeArgumentFor {
                        context: "height",
                    }));
                };

                // Extract optional shift argument
                let shift = if let Some(shift_arg) = &opt_args[0] {
                    match shift_arg {
                        AnyParseNode::Size(s) => Some(s.value.clone()),
                        _ => {
                            return Err(ParseError::new(ParseErrorKind::ExpectedSizeArgumentFor {
                                context: "shift",
                            }));
                        }
                    }
                } else {
                    None
                };

                Ok(ParseNode::Rule(ParseNodeRule {
                    mode: context.parser.mode,
                    loc: context.loc(),
                    shift,
                    width: width_node.value.clone(),
                    height: height_node.value.clone(),
                }))
            },
        ),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for the \rule function
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    if let ParseNode::Rule(rule_node) = node {
        // Use calculate_size to properly convert measurements to ems
        let width = ctx.calculate_size(&rule_node.width, options)?;
        let height = ctx.calculate_size(&rule_node.height, options)?;
        let shift = if let Some(shift_measurement) = &rule_node.shift {
            ctx.calculate_size(shift_measurement, options)?
        } else {
            0.0
        };

        // Create style for the rule
        let mut style = CssStyle::default();
        style.insert(CssProperty::BorderRightWidth, make_em(width));
        style.insert(CssProperty::BorderTopWidth, make_em(height));
        style.insert(CssProperty::Bottom, make_em(shift));

        // Create the span for the rule
        Ok(HtmlDomNode::DomSpan(
            Span::builder()
                .children(vec![]) // Empty content for the rule
                .classes(ClassList::Const(&["mord", "rule"]))
                .height(height + shift)
                .depth(-shift)
                .width(Some(width))
                .max_font_size(height * 1.125 * options.size_multiplier)
                .style(style)
                .build(Some(options)),
        ))
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Rule,
        }))
    }
}

/// MathML builder for the \rule function
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    if let ParseNode::Rule(rule_node) = node {
        // Use calculate_size to properly convert measurements to ems
        let width = ctx.calculate_size(&rule_node.width, options)?;
        let height = ctx.calculate_size(&rule_node.height, options)?;
        let shift = if let Some(shift_measurement) = &rule_node.shift {
            ctx.calculate_size(shift_measurement, options)?
        } else {
            0.0
        };

        let color = options
            .color
            .as_ref()
            .map_or_else(|| "black".to_owned(), ToString::to_string);

        let mut rule = MathNode::builder().node_type(MathNodeType::Mspace).build();

        rule.attributes.extend([
            ("width".to_owned(), make_em(width)),
            ("height".to_owned(), make_em(height)),
            ("mathbackground".to_owned(), color),
        ]);

        let mut wrapper = MathNode::builder()
            .node_type(MathNodeType::Mpadded)
            .children(vec![MathDomNode::Math(rule)])
            .build();

        wrapper.attributes.extend([
            ("height".to_owned(), make_em(shift)),
            ("voffset".to_owned(), make_em(shift)),
        ]);

        if shift < 0.0 {
            wrapper.set_attribute("depth", make_em(-shift));
        }

        Ok(MathDomNode::Math(wrapper))
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Rule,
        }))
    }
}
