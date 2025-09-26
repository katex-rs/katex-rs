//! Enclose function implementations for KaTeX Rust
//!
//! This module handles enclosure symbols in mathematical expressions,
//! migrated from KaTeX's enclose.js.

use crate::build_common::{VListElemAndShift, VListParam, make_span, make_v_list};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::{HtmlDomNode, PathNode, SvgChildNode, SvgNode};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeEnclose};
use crate::spacing_data::Measurement;
use crate::stretchy::enclose_span;
use crate::svg_geometry::phase_path;
use crate::types::{ArgType, CssProperty, Mode, ParseError, ParseErrorKind};
use crate::units::make_em as units_make_em;
use crate::{KatexContext, build_common};
use crate::{build_html, build_mathml};

/// Registers enclose functions in the KaTeX context
pub fn define_enclose(ctx: &mut KatexContext) {
    // \colorbox
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Enclose),
        names: &["\\colorbox"],
        props: FunctionPropSpec {
            num_args: 2,
            allowed_in_text: true,
            arg_types: Some(vec![ArgType::Color, ArgType::Mode(Mode::Text)]),
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let color = match &args[0] {
                AnyParseNode::ColorToken(color_token) => color_token.color.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ExpectedColorToken {
                        argument: "first argument",
                    }));
                }
            };

            let body = args[1].clone();

            Ok(ParseNode::Enclose(ParseNodeEnclose {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                background_color: Some(color),
                border_color: None,
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \fcolorbox
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Enclose),
        names: &["\\fcolorbox"],
        props: FunctionPropSpec {
            num_args: 3,
            allowed_in_text: true,
            arg_types: Some(vec![
                ArgType::Color,
                ArgType::Color,
                ArgType::Mode(Mode::Text),
            ]),
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let border_color = match &args[0] {
                AnyParseNode::ColorToken(color_token) => color_token.color.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ExpectedColorToken {
                        argument: "first argument",
                    }));
                }
            };

            let background_color = match &args[1] {
                AnyParseNode::ColorToken(color_token) => color_token.color.clone(),
                _ => {
                    return Err(ParseError::new(ParseErrorKind::ExpectedColorToken {
                        argument: "second argument",
                    }));
                }
            };

            let body = args[2].clone();

            Ok(ParseNode::Enclose(ParseNodeEnclose {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                background_color: Some(background_color),
                border_color: Some(border_color),
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \fbox
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Enclose),
        names: &["\\fbox"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Hbox]),
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let body = args[0].clone();

            Ok(ParseNode::Enclose(ParseNodeEnclose {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                background_color: None,
                border_color: None,
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // Cancel functions: \cancel, \bcancel, \xcancel, \sout, \phase
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Enclose),
        names: &["\\cancel", "\\bcancel", "\\xcancel", "\\sout", "\\phase"],
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            if args.len() != 1 {
                return Err(ParseError::new(
                    ParseErrorKind::CancelFunctionSingleArgument,
                ));
            }

            let body = args[0].clone();

            Ok(ParseNode::Enclose(ParseNodeEnclose {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                background_color: None,
                border_color: None,
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \angl
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Enclose),
        names: &["\\angl"],
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Hbox]),
            allowed_in_text: false,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let body = args[0].clone();

            Ok(ParseNode::Enclose(ParseNodeEnclose {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                background_color: None,
                border_color: None,
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for enclose nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Enclose(enclose_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Enclose,
        }));
    };

    // Build the inner content
    let mut inner = build_common::wrap_fragment(
        build_html::build_group(ctx, &enclose_node.body, options, None)?,
        options,
    );

    let label = enclose_node.label.trim_start_matches('\\');
    let scale = options.size_multiplier;
    let img_shift;

    // Check if single character
    let is_single_char = enclose_node.body.is_character_box()?;

    if label == "sout" {
        let mut img = make_span(
            vec!["stretchy".to_owned(), "sout".to_owned()],
            vec![],
            None,
            None,
        );
        img.height = options.font_metrics().default_rule_thickness / scale;
        img_shift = -0.5 * options.font_metrics().x_height;

        // Create the vlist
        let vlist = make_v_list(
            VListParam::IndividualShift {
                children: vec![
                    VListElemAndShift::builder().elem(inner).shift(0.0).build(),
                    VListElemAndShift::builder()
                        .elem(img.into())
                        .shift(img_shift)
                        .build(),
                ],
            },
            options,
        )?;

        if label == "cancel" && !is_single_char {
            return Ok(make_span(
                vec!["mord".to_owned(), "cancel-lap".to_owned()],
                vec![vlist.into()],
                Some(options),
                None,
            )
            .into());
        }

        return Ok(make_span(
            vec!["mord".to_owned()],
            vec![vlist.into()],
            Some(options),
            None,
        )
        .into());
    }

    if label == "phase" {
        // Set dimensions from steinmetz package
        let line_weight = ctx.calculate_size(
            &Measurement {
                number: 0.6,
                unit: "pt",
            },
            options,
        )?;
        let clearance = ctx.calculate_size(
            &Measurement {
                number: 0.35,
                unit: "ex",
            },
            options,
        )?;

        // Prevent size changes
        let new_options = options.having_base_sizing();
        let scale = scale / new_options.size_multiplier;

        let angle_height = inner.height() + inner.depth() + line_weight + clearance;
        if let Some(style) = inner.style_mut() {
            style.insert(
                CssProperty::PaddingLeft,
                units_make_em(angle_height / 2.0 + line_weight),
            );
        }

        // Create SVG
        let view_box_height = 1000.0 * angle_height * scale;
        let path = phase_path(view_box_height);
        let mut svg_node = SvgNode::builder()
            .children(vec![SvgChildNode::Path(PathNode {
                path_name: "phase".to_owned(),
                alternate: Some(path),
            })])
            .build();

        svg_node.attributes.extend([
            ("width".to_owned(), "400em".to_owned()),
            ("height".to_owned(), units_make_em(view_box_height / 1000.0)),
            (
                "viewBox".to_owned(),
                format!("0 0 400000 {view_box_height}"),
            ),
            (
                "preserveAspectRatio".to_owned(),
                "xMinYMin slice".to_owned(),
            ),
        ]);

        let mut img =
            build_common::make_svg_span(vec!["hide-tail".to_owned()], vec![svg_node], options);
        img.style
            .insert(CssProperty::Height, units_make_em(angle_height));
        img_shift = inner.depth() + line_weight + clearance;

        // Create the vlist
        let vlist = make_v_list(
            VListParam::IndividualShift {
                children: vec![
                    VListElemAndShift::builder().elem(inner).shift(0.0).build(),
                    VListElemAndShift::builder()
                        .elem(img.into())
                        .shift(img_shift)
                        .wrapper_classes(vec!["svg-align".to_owned()])
                        .build(),
                ],
            },
            options,
        )?;

        return Ok(make_span(vec!["mord".to_owned()], vec![vlist.into()], None, None).into());
    }

    // Handle other enclosures (cancel, box, angl)
    let top_pad;
    let bottom_pad;
    let mut rule_thickness = 0.0;

    // Add padding classes
    if let Some(classes) = inner.classes_mut() {
        if label.contains("cancel") {
            if !is_single_char {
                classes.push("cancel-pad".to_owned());
            }
        } else if label == "angl" {
            classes.push("anglpad".to_owned());
        } else {
            classes.push("boxpad".to_owned());
        }
    }

    // Record the dimensions of the inner element before it is moved into the
    // vertical list. JavaScript KaTeX preserves the original height/depth for
    // cancel-style enclosures so that the strike does not affect surrounding
    // layout. Without capturing these values beforehand we lose access once the
    // node is consumed.
    let inner_height = inner.height();
    let inner_depth = inner.depth();

    // Calculate padding
    if label.contains("box") {
        rule_thickness = options
            .font_metrics()
            .fboxrule
            .max(options.min_rule_thickness);
        top_pad = options.font_metrics().fboxsep
            + if enclose_node.label == "\\colorbox" {
                0.0
            } else {
                rule_thickness
            };
        bottom_pad = top_pad;
    } else if label == "angl" {
        rule_thickness = options
            .font_metrics()
            .default_rule_thickness
            .max(options.min_rule_thickness);
        top_pad = 4.0 * rule_thickness; // gap = 3 × line, plus the line itself
        bottom_pad = 0.0f64.max(0.25 - inner.depth());
    } else {
        top_pad = if is_single_char { 0.2 } else { 0.0 };
        bottom_pad = top_pad;
    }

    // Create the enclosure span
    let mut img = enclose_span(&inner, label, top_pad, bottom_pad, options);

    // Apply border styles
    if label.contains("fbox") || label.contains("boxed") || label.contains("fcolorbox") {
        img.style
            .insert(CssProperty::BorderStyle, "solid".to_owned());
        img.style
            .insert(CssProperty::BorderWidth, units_make_em(rule_thickness));
    } else if label == "angl" && rule_thickness != 0.049 {
        img.style
            .insert(CssProperty::BorderTopWidth, units_make_em(rule_thickness));
        img.style
            .insert(CssProperty::BorderRightWidth, units_make_em(rule_thickness));
    }

    img_shift = inner.depth() + bottom_pad;

    // Handle background and border colors
    if let Some(bg_color) = &enclose_node.background_color {
        img.style
            .insert(CssProperty::BackgroundColor, bg_color.clone());
        if let Some(border_color) = &enclose_node.border_color {
            img.style
                .insert(CssProperty::BorderColor, border_color.clone());
        }
    }

    // Create the vlist
    let mut vlist = if enclose_node.background_color.is_some() {
        make_v_list(
            VListParam::IndividualShift {
                children: vec![
                    VListElemAndShift::builder()
                        .elem(img.into())
                        .shift(img_shift)
                        .build(),
                    VListElemAndShift::builder().elem(inner).shift(0.0).build(),
                ],
            },
            options,
        )?
    } else {
        let wrapper_classes =
            (label.contains("cancel") || label == "phase").then(|| vec!["svg-align".to_owned()]);

        make_v_list(
            VListParam::IndividualShift {
                children: vec![
                    VListElemAndShift::builder().elem(inner).shift(0.0).build(),
                    VListElemAndShift::builder()
                        .elem(img.into())
                        .shift(img_shift)
                        .maybe_wrapper_classes(wrapper_classes)
                        .build(),
                ],
            },
            options,
        )?
    };

    // Strike-through operators should not extend the surrounding box. Align
    // with the reference implementation by restoring the inner height/depth so
    // that only the glyph content contributes to layout metrics.
    if label.contains("cancel") {
        vlist.height = inner_height;
        vlist.depth = inner_depth;
    }

    if label.contains("cancel") && !is_single_char {
        Ok(make_span(
            vec!["mord".to_owned(), "cancel-lap".to_owned()],
            vec![vlist.into()],
            Some(options),
            None,
        )
        .into())
    } else {
        Ok(make_span(
            vec!["mord".to_owned()],
            vec![vlist.into()],
            Some(options),
            None,
        )
        .into())
    }
}

/// MathML builder for enclose nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Enclose(enclose_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Enclose,
        }));
    };

    let node_type = if enclose_node.label.contains("colorbox") {
        MathNodeType::Mpadded
    } else {
        MathNodeType::Menclose
    };

    let mut math_node = MathNode::builder()
        .node_type(node_type)
        .children(vec![build_mathml::build_group(
            ctx,
            &enclose_node.body,
            options,
        )?])
        .build();

    match enclose_node.label.as_str() {
        "\\cancel" => {
            math_node.set_attribute("notation", "updiagonalstrike");
        }
        "\\bcancel" => {
            math_node.set_attribute("notation", "downdiagonalstrike");
        }
        "\\phase" => {
            math_node.set_attribute("notation", "phasorangle");
        }
        "\\sout" => {
            math_node.set_attribute("notation", "horizontalstrike");
        }
        "\\fbox" => {
            math_node.set_attribute("notation", "box");
        }
        "\\angl" => {
            math_node.set_attribute("notation", "actuarial");
        }
        "\\fcolorbox" | "\\colorbox" => {
            // <menclose> doesn't have a good notation option. So use <mpadded>
            // instead. Set some attributes that come included with <menclose>.
            let fboxsep_pt = options.font_metrics().fboxsep * options.font_metrics().pt_per_em;
            math_node.set_attribute("width", format!("+{}pt", 2.0 * fboxsep_pt));
            math_node.set_attribute("height", format!("+{}pt", 2.0 * fboxsep_pt));
            math_node.set_attribute("lspace", format!("{fboxsep_pt}pt"));
            math_node.set_attribute("voffset", format!("{fboxsep_pt}pt"));

            if enclose_node.label == "\\fcolorbox" {
                let thk = options
                    .font_metrics()
                    .fboxrule
                    .max(options.min_rule_thickness);
                let border_color = enclose_node.border_color.as_deref().unwrap_or("");
                math_node.set_attribute("style", format!("border: {thk}em solid {border_color}"));
            }
        }
        "\\xcancel" => {
            math_node.set_attribute("notation", "updiagonalstrike downdiagonalstrike");
        }
        _ => {}
    }

    if let Some(bg_color) = &enclose_node.background_color {
        math_node.set_attribute("mathbackground", bg_color);
    }

    Ok(MathDomNode::Math(math_node))
}
