//! Implementation of kerning and spacing commands in KaTeX
//!
//! This module implements the LaTeX kerning commands `\kern`, `\mkern`,
//! `\hskip`, and `\mskip`, which provide explicit horizontal spacing in
//! mathematical expressions.

use crate::context::KatexContext;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::{HtmlDomNode, Span};
use crate::mathml_tree::{MathDomNode, SpaceNode};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeKern};
use crate::spacing_data::MeasurementStatic;
use crate::types::{
    ArgType, CssProperty, CssStyle, ErrorLocationProvider, Mode, ParseError, ParseErrorKind,
};
use crate::units::make_em;

/// Register the kerning functions (\kern, \mkern, \hskip, \mskip)
pub fn define_kern(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Kern),
        names: &["\\kern", "\\mkern", "\\hskip", "\\mskip"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Size]),
            primitive: true,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(
            |context, args: Vec<ParseNode>, _opt_args: Vec<Option<ParseNode>>| {
                // Extract the size argument
                let size_arg = args
                    .first()
                    .ok_or_else(|| ParseError::new(ParseErrorKind::ExpectedSizeArgument))?;
                let AnyParseNode::Size(size_node) = size_arg else {
                    return Err(ParseError::new(ParseErrorKind::ExpectedSizeArgument));
                };

                // Strict mode validations
                if context.parser.settings.use_strict_behavior(
                    "mathVsTextUnits",
                    "",
                    context
                        .token
                        .as_ref()
                        .map(|t| *t as &dyn ErrorLocationProvider),
                ) {
                    let func_name = &context.func_name;
                    let math_function = func_name.chars().nth(1) == Some('m'); // \mkern, \mskip
                    let mu_unit = size_node.value.unit == "mu";

                    if math_function {
                        if !mu_unit {
                            context.parser.settings.report_nonstrict(
                                "mathVsTextUnits",
                                &format!(
                                    "LaTeX's {} supports only mu units, not {} units",
                                    func_name, size_node.value.unit
                                ),
                                context
                                    .token
                                    .as_ref()
                                    .map(|t| *t as &dyn ErrorLocationProvider),
                            )?;
                        }
                        if context.parser.mode != Mode::Math {
                            context.parser.settings.report_nonstrict(
                                "mathVsTextUnits",
                                &format!("LaTeX's {func_name} works only in math mode"),
                                context
                                    .token
                                    .as_ref()
                                    .map(|t| *t as &dyn ErrorLocationProvider),
                            )?;
                        }
                    } else {
                        // !math_function (\kern, \hskip)
                        if mu_unit {
                            context.parser.settings.report_nonstrict(
                                "mathVsTextUnits",
                                &format!("LaTeX's {func_name} doesn't support mu units"),
                                context
                                    .token
                                    .as_ref()
                                    .map(|t| *t as &dyn ErrorLocationProvider),
                            )?;
                        }
                    }
                }

                Ok(ParseNode::Kern(ParseNodeKern {
                    mode: context.parser.mode,
                    loc: context.loc(),
                    dimension: size_node.value.clone(),
                }))
            },
        ),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for kerning functions
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    if let ParseNode::Kern(kern_node) = node {
        // Use calculate_size to properly convert the measurement to ems
        if let Ok(size_value) = ctx.calculate_size(&kern_node.dimension, options) {
            // Create a span with the calculated width
            let mut style = CssStyle::default();
            style.insert(CssProperty::MarginRight, make_em(size_value));

            Ok(HtmlDomNode::DomSpan(
                Span::builder()
                    .children(vec![]) // Empty content for spacing
                    .classes(vec!["mspace".to_owned()])
                    .max_font_size(options.size_multiplier)
                    .style(style)
                    .build(Some(options)),
            ))
        } else {
            // If calculate_size fails, fall back to a simple calculation
            let dimension_static = MeasurementStatic {
                number: kern_node.dimension.number,
                unit: match kern_node.dimension.unit.as_str() {
                    "mu" => "mu",
                    "ex" => "ex",
                    "pt" => "pt",
                    "px" => "px",
                    _ => "em", // Default fallback
                },
            };
            Ok(ctx.make_glue(&dimension_static, options)?.into())
        }
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Kern,
        }))
    }
}

/// MathML builder for kerning functions
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    if let ParseNode::Kern(kern_node) = node {
        // Use calculate_size to properly convert the measurement to ems
        let dimension = ctx
            .calculate_size(&kern_node.dimension, options)
            .map_or(kern_node.dimension.number, |d| d);

        Ok(MathDomNode::Space(SpaceNode {
            width: dimension,
            character: None, // Use default space character
        }))
    } else {
        Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Kern,
        }))
    }
}
