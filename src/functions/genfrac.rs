//! Minimal genfrac-related function registrations to match KaTeX design.
//! Registers infix primitives: \over, \choose, \above.

use crate::build_common::{VListElemAndShift, VListParam, make_span, make_v_list};
use crate::build_html::make_null_delimiter;
use crate::build_mathml::make_row;
use crate::define_function::{FunctionContext, normalize_argument};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::delimiter::custom_sized_delim;
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeGenfrac, ParseNodeInfix};
use crate::style::{DISPLAY, SCRIPT, SCRIPTSCRIPT, Style, TEXT};
use crate::symbols::Atom;
use crate::types::{ArgType, Mode, ParseError, ParseErrorKind};
use crate::units::make_em;
use crate::{KatexContext, build_html, build_mathml, make_line_span};
use phf::Map;

fn delim_from_value(delim: &str) -> Option<String> {
    if delim.is_empty() || delim == "." {
        None
    } else {
        Some(delim.to_owned())
    }
}

const INFIX_REPLACE_MAP: Map<&'static str, &'static str> = phf::phf_map! {
    "\\over" => "\\frac",
    "\\choose" => "\\binom",
    "\\atop" => r"\\atopfrac",
    "\\brace" => r"\\bracefrac",
    "\\brack" => r"\\brackfrac",
};

/// Adjusts the style based on the fraction size and original style
const fn adjust_style<'a>(size: Option<&'a Style>, original_style: &'a Style) -> &'a Style {
    let mut style = original_style;
    if let Some(size) = size {
        if size.id == DISPLAY.id {
            // Get display style as a default.
            // If incoming style is sub/sup, use text() to get correct size.
            style = if style.id >= SCRIPT.id {
                style.text()
            } else {
                DISPLAY
            };
        } else if size.id == TEXT.id && style.size == DISPLAY.size {
            // We're in a \tfrac but incoming style is displaystyle, so:
            style = TEXT;
        } else if size.id == SCRIPT.id {
            style = SCRIPT;
        } else if size.id == SCRIPTSCRIPT.id {
            style = SCRIPTSCRIPT;
        }
    }
    style
}

/// Infix generalized fractions -- these are not rendered directly, but replaced
/// immediately by one of the variants above.
pub fn define_genfrac(ctx: &mut crate::KatexContext) {
    // Regular genfrac functions
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Genfrac),
        names: &[
            r"\dfrac",
            r"\frac",
            r"\tfrac",
            r"\dbinom",
            r"\binom",
            r"\tbinom",
            r"\\atopfrac", // can't be entered directly
            r"\\bracefrac",
            r"\\brackfrac", // ditto
        ],
        props: FunctionPropSpec {
            num_args: 2,
            allowed_in_argument: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let numer = args[0].clone();
            let denom = args[1].clone();

            let (left_delim, right_delim, has_bar_line) = match context.func_name.as_str() {
                "\\dfrac" | "\\frac" | "\\tfrac" => (None, None, true),
                "\\dbinom" | "\\binom" | "\\tbinom" => {
                    (Some("(".to_owned()), Some(")".to_owned()), false)
                }
                "\\\\atopfrac" => (None, None, false),
                "\\\\bracefrac" => (Some(r"\{".to_owned()), Some(r"\}".to_owned()), false),
                "\\\\brackfrac" => (Some("[".to_owned()), Some("]".to_owned()), false),
                _ => {
                    return Err(ParseError::new(
                        ParseErrorKind::UnrecognizedGenfracCommand {
                            command: context.func_name.clone(),
                        },
                    ));
                }
            };

            let size = match context.func_name.as_str() {
                "\\dfrac" | "\\dbinom" => Some(DISPLAY),
                "\\tfrac" | "\\tbinom" => Some(TEXT),
                _ => None,
            };

            Ok(ParseNode::Genfrac(Box::new(ParseNodeGenfrac {
                mode: context.parser.mode,
                loc: context.loc(),
                continued: false,
                numer: Box::new(numer),
                denom: Box::new(denom),
                has_bar_line,
                left_delim,
                right_delim,
                size,
                bar_size: None,
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Genfrac),
        names: &["\\cfrac"],
        props: FunctionPropSpec {
            num_args: 2,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let numer = args[0].clone();
            let denom = args[1].clone();

            Ok(ParseNode::Genfrac(Box::new(ParseNodeGenfrac {
                mode: context.parser.mode,
                loc: context.loc(),
                continued: true,
                numer: Box::new(numer),
                denom: Box::new(denom),
                has_bar_line: true,
                left_delim: None,
                right_delim: None,
                size: Some(DISPLAY),
                bar_size: None,
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \over: infix primitive (no args)
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Infix),
        names: &["\\over", "\\choose", "\\atop", "\\brace", "\\brack"],
        props: FunctionPropSpec {
            infix: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            if let Some(replace_with) = INFIX_REPLACE_MAP.get(context.func_name.as_str()) {
                Ok(ParseNode::Infix(ParseNodeInfix {
                    mode: context.parser.mode,
                    loc: context.loc(),
                    replace_with: (*replace_with).to_owned(),
                    size: None,
                    token: None,
                }))
            } else {
                let kind = ParseErrorKind::UnrecognizedInfixGenfracCommand {
                    command: context.func_name.clone(),
                };
                if let Some(token) = context.token {
                    Err(ParseError::with_token(kind, token))
                } else {
                    Err(ParseError::new(kind))
                }
            }
        }),
        html_builder: None,
        mathml_builder: None,
    });

    // \above is special
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Infix),
        names: &["\\above"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Size]),
            infix: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let size_node = &args[0];
            // Extract measurement from Size node
            let size_measurement = if let ParseNode::Size(size) = size_node {
                Some(size.value.clone())
            } else {
                return Err(ParseError::new("\\above argument must be a size"));
            };
            Ok(ParseNode::Infix(ParseNodeInfix {
                mode: context.parser.mode,
                loc: context.loc(),
                replace_with: "\\\\abovefrac".to_owned(),
                size: size_measurement,
                token: None,
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });

    // \\\\abovefrac
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Genfrac),
        names: &["\\\\abovefrac"],
        props: FunctionPropSpec {
            num_args: 3,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let numer = args[0].clone();
            let infix_node = &args[1];
            let denom = args[2].clone();

            // Extract measurement from Infix node
            let bar_size = if let ParseNode::Infix(infix) = infix_node {
                infix.size.clone()
            } else {
                return Err(ParseError::new(
                    "\\\\abovefrac second argument must be an Infix node",
                ));
            };

            let has_bar_line = bar_size
                .as_ref()
                .is_some_and(|measurement| measurement.number > 0.0);

            Ok(ParseNode::Genfrac(Box::new(ParseNodeGenfrac {
                mode: context.parser.mode,
                loc: context.loc(),
                continued: false,
                numer: Box::new(numer),
                denom: Box::new(denom),
                has_bar_line,
                left_delim: None,
                right_delim: None,
                size: None,
                bar_size,
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \genfrac
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Genfrac),
        names: &["\\genfrac"],
        props: FunctionPropSpec {
            num_args: 6,
            arg_types: Some(vec![
                ArgType::Mode(Mode::Math),
                ArgType::Mode(Mode::Math),
                ArgType::Size,
                ArgType::Mode(Mode::Text),
                ArgType::Mode(Mode::Math),
                ArgType::Mode(Mode::Math),
            ]),
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let numer = args[4].clone();
            let denom = args[5].clone();

            // Handle left delimiter
            let left_node = normalize_argument(&args[0]);
            let left_delim = match left_node {
                ParseNode::Atom(node) if node.family == Atom::Open => delim_from_value(&node.text),
                _ => None,
            };

            // Handle right delimiter
            let right_node = normalize_argument(&args[1]);
            let right_delim = match right_node {
                ParseNode::Atom(node) if node.family == Atom::Close => delim_from_value(&node.text),
                _ => None,
            };

            // Handle bar size
            let mut has_bar_line = true;
            let bar_size = if let ParseNode::Size(size_node) = &args[2] {
                if size_node.is_blank {
                    None
                } else {
                    has_bar_line = size_node.value.number > 0.0;
                    Some(size_node.value.clone())
                }
            } else {
                None
            };

            // Handle style/size
            let mut size = None;
            let convert_style = |text: &str| {
                let level = text.parse::<u8>().map_err(|_| {
                    ParseError::new(ParseErrorKind::InvalidGenfracStyle {
                        level: text.to_owned(),
                    })
                })?;
                match level {
                    0 => Ok(DISPLAY),
                    1 => Ok(TEXT),
                    2 => Ok(SCRIPT),
                    3 => Ok(SCRIPTSCRIPT),
                    _ => Err(ParseError::new(ParseErrorKind::InvalidGenfracStyle {
                        level: level.to_string(),
                    })),
                }
            };

            match &args[3] {
                ParseNode::OrdGroup(ord_group) => {
                    if let Some(ParseNode::TextOrd(text_ord)) = ord_group.body.first() {
                        size = Some(convert_style(&text_ord.text)?);
                    }
                }
                ParseNode::TextOrd(text_ord) => {
                    size = Some(convert_style(&text_ord.text)?);
                }
                _ => {}
            }

            Ok(ParseNode::Genfrac(Box::new(ParseNodeGenfrac {
                mode: context.parser.mode,
                loc: context.loc(),
                continued: false,
                numer: Box::new(numer),
                denom: Box::new(denom),
                has_bar_line,
                left_delim,
                right_delim,
                size,
                bar_size,
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for genfrac nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Genfrac(group) = node else {
        return Err(ParseError::new("Expected Genfrac node"));
    };

    // Adjust style based on fraction size (like JavaScript version)
    let style = adjust_style(group.size, options.style);

    // Get numerator and denominator styles
    let nstyle = style.frac_num();
    let dstyle = style.frac_den();

    // Create new options with adjusted styles
    let numer_options = options.having_style(nstyle);
    let denom_options = options.having_style(dstyle);

    // Build numerator and denominator with adjusted styles
    let mut numer = build_html::build_group(ctx, &group.numer, &numer_options, Some(options))?;
    if group.continued {
        // \cfrac inserts a \strut into the numerator.
        // Get \strut dimensions from TeXbook page 353.
        let h_strut = 8.5 / options.font_metrics().pt_per_em;
        let d_strut = 3.5 / options.font_metrics().pt_per_em;
        if let Some(height) = numer.height_mut() {
            *height = height.max(h_strut);
        }
        if let Some(depth) = numer.depth_mut() {
            *depth = depth.max(d_strut);
        }
    }
    let denom = build_html::build_group(ctx, &group.denom, &denom_options, Some(options))?;

    // Calculate shifts
    let fm = options.font_metrics();
    let axis_height = fm.axis_height;
    let (rule, rule_width, rule_spacing) = if group.has_bar_line {
        let rule = if let Some(bar_size) = &group.bar_size {
            let rule_width = ctx.calculate_size(bar_size, options)?;
            make_line_span("frac-line", options, Some(rule_width))
        } else {
            make_line_span("frac-line", options, None)
        };
        let rule_height = rule.height;
        (Some(rule), rule_height, rule_height)
    } else {
        (None, 0.0, options.font_metrics().default_rule_thickness)
    };

    let (mut num_shift, mut denom_shift, clearance) =
        if style.id == DISPLAY.id || group.size == Some(DISPLAY) {
            let num_shift = fm.num1;
            let clearance = if rule_width > 0.0 {
                3.0 * rule_spacing
            } else {
                7.0 * rule_spacing
            };
            let denom_shift = fm.denom1;
            (num_shift, denom_shift, clearance)
        } else {
            let (num_shift, clearance) = if rule_width > 0.0 {
                (fm.num2, rule_spacing)
            } else {
                (fm.num3, 3.0 * rule_spacing)
            };
            let denom_shift = fm.denom2;
            (num_shift, denom_shift, clearance)
        };

    if group.has_bar_line {
        // Rule 15d: With fraction bar
        if (num_shift - numer.depth()) - 0.5f64.mul_add(rule_width, axis_height) < clearance {
            num_shift +=
                clearance - ((num_shift - numer.depth()) - 0.5f64.mul_add(rule_width, axis_height));
        }

        if 0.5f64.mul_add(-rule_width, axis_height) - (denom.height() - denom_shift) < clearance {
            denom_shift += clearance
                - (0.5f64.mul_add(-rule_width, axis_height) - (denom.height() - denom_shift));
        }
    } else {
        // Rule 15c: Without fraction bar
        let candidate_clearance = (num_shift - numer.depth()) - (denom.height() - denom_shift);
        if candidate_clearance < clearance {
            let adjustment = 0.5 * (clearance - candidate_clearance);
            num_shift += adjustment;
            denom_shift += adjustment;
        }
    }

    let mut children = Vec::new();

    // Add denominator
    children.push(
        VListElemAndShift::builder()
            .elem(denom)
            .shift(denom_shift)
            .build(),
    );

    // Add fraction line if needed
    if let Some(rule) = rule {
        // Add a little extra clearance above and below the rule
        let mid_shift = -0.5f64.mul_add(-rule_width, axis_height);
        children.push(
            VListElemAndShift::builder()
                .elem(rule.into())
                .shift(mid_shift)
                .build(),
        );
    }

    // Add numerator
    children.push(
        VListElemAndShift::builder()
            .elem(numer)
            .shift(-num_shift)
            .build(),
    );

    // Create vertical list
    let mut frac = make_v_list(VListParam::IndividualShift { children }, options)?;

    // Since we manually change the style sometimes (like \dfrac or \tfrac),
    // account for the possible size change here.
    let new_options = options.having_style(style);
    frac.height *= new_options.size_multiplier / options.size_multiplier;
    frac.depth *= new_options.size_multiplier / options.size_multiplier;

    // Calculate delimSize
    let delim_size = if style.size == DISPLAY.size {
        options.font_metrics().delim1
    } else if style.size == SCRIPTSCRIPT.size {
        options.having_style(SCRIPT).font_metrics().delim2
    } else {
        options.font_metrics().delim2
    };

    // Create delimiters
    let left_delim_span = if let Some(left_delim) = &group.left_delim {
        custom_sized_delim(
            ctx,
            left_delim,
            delim_size,
            true,
            &options.having_style(style),
            group.mode,
            &[String::from("mopen")],
        )?
    } else {
        make_null_delimiter(options, &[String::from("mopen")])
    };
    let right_delim_span = if group.continued {
        make_span(vec![], vec![], None, None)
    } else if let Some(right_delim) = &group.right_delim {
        custom_sized_delim(
            ctx,
            right_delim,
            delim_size,
            true,
            &options.having_style(style),
            group.mode,
            &[String::from("mclose")],
        )?
    } else {
        make_null_delimiter(options, &[String::from("mclose")])
    };

    let frac_span = make_span(vec![String::from("mfrac")], vec![frac.into()], None, None);

    // Create final span
    let final_children = vec![
        left_delim_span.into(),
        frac_span.into(),
        right_delim_span.into(),
    ];
    let classes = vec![String::from("mord")]
        .into_iter()
        .chain(new_options.sizing_classes(options))
        .collect();
    Ok(make_span(classes, final_children, Some(options), None).into())
}

/// MathML builder for genfrac nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Genfrac(genfrac_node) = node else {
        return Err(ParseError::new("Expected Genfrac node"));
    };

    // Adjust style based on fraction size (like JavaScript version)
    let style = adjust_style(genfrac_node.size, options.style);

    // Create new options with adjusted styles for numerator and denominator
    let numer_options = options.having_style(style.frac_num());
    let denom_options = options.having_style(style.frac_den());

    // Build numerator and denominator with adjusted styles
    let numer = build_mathml::build_group(ctx, &genfrac_node.numer, &numer_options)?;
    let denom = build_mathml::build_group(ctx, &genfrac_node.denom, &denom_options)?;

    // Create mfrac element
    let mut mfrac_node = MathNode::builder()
        .node_type(MathNodeType::Mfrac)
        .children(vec![numer, denom])
        .build();

    // Set linethickness attribute
    if !genfrac_node.has_bar_line {
        mfrac_node.set_attribute("linethickness", "0px");
    } else if let Some(bar_size) = &genfrac_node.bar_size {
        // Calculate the size using ctx.calculate_size
        let size = ctx.calculate_size(bar_size, options)?;
        mfrac_node.set_attribute("linethickness", make_em(size));
    }

    // Handle style changes (like JavaScript version)
    let mut final_node = mfrac_node;
    if style.size != options.style.size {
        let mut mstyle_node = MathNode::builder()
            .node_type(MathNodeType::Mstyle)
            .children(vec![MathDomNode::Math(final_node)])
            .build();

        let is_display = if style.size == DISPLAY.size {
            "true"
        } else {
            "false"
        };
        mstyle_node.set_attribute("displaystyle", is_display);
        mstyle_node.set_attribute("scriptlevel", "0");
        final_node = mstyle_node;
    }

    // Handle delimiters
    if genfrac_node.left_delim.is_some() || genfrac_node.right_delim.is_some() {
        let mut children = Vec::new();

        if let Some(left_delim) = &genfrac_node.left_delim {
            let left_op = MathNode::builder()
                .node_type(MathNodeType::Mo)
                .children(vec![MathDomNode::Text(TextNode {
                    text: left_delim.replace('\\', ""),
                })])
                .build();
            let mut left_op_with_attr = left_op;
            left_op_with_attr.set_attribute("fence", "true");
            children.push(MathDomNode::Math(left_op_with_attr));
        }

        children.push(MathDomNode::Math(final_node));

        if let Some(right_delim) = &genfrac_node.right_delim {
            let right_op = MathNode::builder()
                .node_type(MathNodeType::Mo)
                .children(vec![MathDomNode::Text(TextNode {
                    text: right_delim.replace('\\', ""),
                })])
                .build();
            let mut right_op_with_attr = right_op;
            right_op_with_attr.set_attribute("fence", "true");
            children.push(MathDomNode::Math(right_op_with_attr));
        }

        Ok(make_row(&children))
    } else {
        Ok(MathDomNode::Math(final_node))
    }
}
