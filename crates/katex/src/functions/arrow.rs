//! Arrow function implementations for KaTeX Rust
//!
//! This module handles extensible arrow symbols in mathematical expressions,
//! migrated from KaTeX's arrow.js.

use crate::namespace::KeyMap;

use crate::build_common::{VListElemAndShift, VListParam, make_span, make_v_list};
use crate::build_html::build_group;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeXArrow};
use crate::stretchy::math_ml_node;
use crate::stretchy::svg_span;
use crate::types::{ParseError, ParseErrorKind};
use crate::{ClassList, KatexContext, build_mathml};

/// Extensible arrow commands
const ARROW_COMMANDS: &[&str] = &[
    "\\xleftarrow",
    "\\xrightarrow",
    "\\xLeftarrow",
    "\\xRightarrow",
    "\\xleftrightarrow",
    "\\xLeftrightarrow",
    "\\xhookleftarrow",
    "\\xhookrightarrow",
    "\\xmapsto",
    "\\xrightharpoondown",
    "\\xrightharpoonup",
    "\\xleftharpoondown",
    "\\xleftharpoonup",
    "\\xrightleftharpoons",
    "\\xleftrightharpoons",
    "\\xlongequal",
    "\\xtwoheadrightarrow",
    "\\xtwoheadleftarrow",
    "\\xtofrom",
    // The next 3 functions are here to support the mhchem extension.
    // Direct use of these functions is discouraged and may break someday.
    "\\xrightleftarrows",
    "\\xrightequilibrium",
    "\\xleftequilibrium",
    // The next 3 functions are here only to support the {CD} environment.
    "\\\\cdrightarrow",
    "\\\\cdleftarrow",
    "\\\\cdlongequal",
];

/// Registers arrow functions in the KaTeX context
pub fn define_arrow(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::XArrow),
        names: ARROW_COMMANDS,
        props: FunctionPropSpec {
            num_args: 1,
            num_optional_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, opt_args| {
            // Allow internal use of \\cdlongequal without a body
            let body = args.first();
            let below = opt_args
                .first()
                .and_then(|opt| opt.as_ref())
                .map(|node| Box::new(node.clone()));

            Ok(ParseNode::XArrow(ParseNodeXArrow {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                body: body.map(|node| Box::new(node.clone())),
                below,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for arrow nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::XArrow(xarrow) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::XArrow,
        }));
    };

    let style = &options.style;

    // Build the argument groups in the appropriate style.
    let new_options = options.having_style(style.sup());
    let mut upper_group = if let Some(body) = &xarrow.body {
        build_group(ctx, body, &new_options, Some(options))?
    } else {
        make_span(ClassList::Empty, vec![], None, None).into()
    };
    let arrow_prefix = if xarrow.label.starts_with("\\x") {
        "x"
    } else {
        "cd"
    };
    if let HtmlDomNode::DomSpan(span) = &mut upper_group {
        span.classes.push(format!("{arrow_prefix}-arrow-pad"));
    }

    let lower_group = if let Some(below) = &xarrow.below {
        // Build the lower group
        let new_options = options.having_style(style.sub());
        let mut lg = build_group(ctx, below, &new_options, Some(options))?;
        if let HtmlDomNode::DomSpan(span) = &mut lg {
            span.classes.push(format!("{arrow_prefix}-arrow-pad"));
        }
        Some(lg)
    } else {
        None
    };

    let arrow_body = svg_span(node, options)?;

    // Re shift: Note that stretchy.svgSpan returned arrowBody.depth = 0.
    // The point we want on the math axis is at 0.5 * arrowBody.height.
    let arrow_shift = 0.5f64.mul_add(arrow_body.height(), -options.font_metrics().axis_height);
    // 2 mu kern. Ref: amsmath.dtx: #7\if0#2\else\mkern#2mu\fi
    let mut upper_shift =
        0.5f64.mul_add(-arrow_body.height(), -options.font_metrics().axis_height) - 0.111; // 0.111 em = 2 mu
    if upper_group.depth() > 0.25 || xarrow.label == "\\xleftequilibrium" {
        upper_shift -= upper_group.depth(); // shift up if depth encroaches
    }

    // Generate the vlist
    let vlist = if let Some(lower_group) = lower_group {
        let lower_shift = 0.5f64.mul_add(
            arrow_body.height(),
            -options.font_metrics().axis_height + lower_group.height(),
        ) + 0.111;
        let children = vec![
            VListElemAndShift {
                elem: upper_group,
                shift: upper_shift,
                margin_left: None,
                margin_right: None,
                wrapper_classes: None,
                wrapper_style: None,
            },
            VListElemAndShift {
                elem: arrow_body,
                shift: arrow_shift,
                margin_left: None,
                margin_right: None,
                wrapper_classes: None,
                wrapper_style: None,
            },
            VListElemAndShift {
                elem: lower_group,
                shift: lower_shift,
                margin_left: None,
                margin_right: None,
                wrapper_classes: None,
                wrapper_style: None,
            },
        ];
        make_v_list(VListParam::IndividualShift { children }, options)?
    } else {
        let children = vec![
            VListElemAndShift {
                elem: upper_group,
                shift: upper_shift,
                margin_left: None,
                margin_right: None,
                wrapper_classes: None,
                wrapper_style: None,
            },
            VListElemAndShift {
                elem: arrow_body,
                shift: arrow_shift,
                margin_left: None,
                margin_right: None,
                wrapper_classes: None,
                wrapper_style: None,
            },
        ];
        make_v_list(VListParam::IndividualShift { children }, options)?
    };

    Ok(make_span(
        ClassList::Const(&["mrel", "x-arrow"]),
        vec![vlist.into()],
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for arrow nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::XArrow(xarrow) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::XArrow,
        }));
    };

    let mut arrow_node = math_ml_node(&xarrow.label);
    // Set minsize based on arrow type
    let minsize = if xarrow.label.starts_with("\\x") {
        "1.75em"
    } else {
        "3.0em"
    };
    arrow_node.set_attribute("minsize", minsize);
    let arrow_node = arrow_node.into();

    if let Some(body) = &xarrow.body {
        let upper_group = padded_node(Some(build_mathml::build_group(
            ctx,
            body.as_ref(),
            options,
        )?))
        .into();
        if let Some(below) = &xarrow.below {
            let lower_group = padded_node(Some(build_mathml::build_group(
                ctx,
                below.as_ref(),
                options,
            )?))
            .into();
            let munderover = MathNode::builder()
                .node_type(MathNodeType::Munderover)
                .children(vec![arrow_node, lower_group, upper_group])
                .build();
            Ok(munderover.into())
        } else {
            let mover = MathNode::builder()
                .node_type(MathNodeType::Mover)
                .children(vec![arrow_node, upper_group])
                .build();
            Ok(mover.into())
        }
    } else if let Some(below) = &xarrow.below {
        let lower_group = padded_node(Some(build_mathml::build_group(
            ctx,
            below.as_ref(),
            options,
        )?))
        .into();
        let munder = MathNode::builder()
            .node_type(MathNodeType::Munder)
            .children(vec![arrow_node, lower_group])
            .build();
        Ok(munder.into())
    } else {
        let node = MathNode::builder()
            .node_type(MathNodeType::Mover)
            .children(vec![arrow_node, padded_node(None).into()])
            .build();
        Ok(node.into())
    }
}

/// Helper function to create a padded MathML node
fn padded_node(child: Option<MathDomNode>) -> MathNode {
    let mut attributes = KeyMap::default();
    attributes.extend([
        ("width".to_owned(), "+0.6em".to_owned()),
        ("lspace".to_owned(), "0.3em".to_owned()),
    ]);
    let children = child.map_or_else(Vec::new, |child| vec![child]);
    MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .attributes(attributes)
        .children(children)
        .build()
}
