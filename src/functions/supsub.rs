//! Superscript and subscript function implementations for KaTeX Rust
//!
//! This module handles the rendering of superscript and subscript expressions,
//! following TeXbook rules 18(a-f) for precise positioning and styling.
//!
//! Migrated from KaTeX's supsub.js.

use crate::build_common::{
    VListChild, VListElem, VListElemAndShift, VListParam, make_span, make_v_list,
};
use crate::build_html::{DomType, Side};
use crate::define_function::HtmlBuilder;
use crate::dom_tree::HtmlDomNode;
use crate::functions::{accent, horiz_brace, op, operatorname};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeSupSub};
use crate::style::DISPLAY;
use crate::types::ParseError;
use crate::units::make_em;
use crate::{KatexContext, build_html, build_mathml};

/// HTML builder delegate for supsub nodes
///
/// Sometimes groups perform special rules when they have superscripts or
/// subscripts attached to them. This function lets the `supsub` group know that
/// its inner element should handle the superscripts and subscripts instead of
/// handling them itself.
fn html_builder_delegate(group: &ParseNodeSupSub, options: &Options) -> Option<HtmlBuilder> {
    let base = group.base.as_deref()?;

    match base {
        AnyParseNode::Op(op_node) => {
            // Operators handle supsubs differently when they have limits
            // (e.g. `\displaystyle\sum_2^3`)
            let delegate = op_node.limits()
                && (options.style.size == DISPLAY.size || op_node.always_handle_sup_sub());
            if delegate {
                return Some(op::html_builder);
            }
        }
        AnyParseNode::OperatorName(op_name) => {
            // Operator names handle supsubs differently when they have limits
            let delegate = op_name.always_handle_sup_sub
                && (op_name.limits || options.style.size == DISPLAY.size);
            if delegate {
                return Some(operatorname::html_builder);
            }
        }
        AnyParseNode::Accent(accent) => {
            // Accents handle supsubs when base is a character box
            if accent.base.is_character_box().unwrap_or(false) {
                return Some(accent::html_builder);
            }
        }
        AnyParseNode::HorizBrace(hbrace) => {
            // Horizontal braces handle supsubs based on position
            let is_sup = group.sub.is_none();
            if is_sup == hbrace.is_over {
                return Some(horiz_brace::html_builder);
            }
        }
        _ => {}
    }

    None
}

/// HTML builder for supsub nodes
///
/// Super scripts and subscripts are handled in the TeXbook on page
/// 445-446, rules 18(a-f).
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::SupSub(group) = node else {
        return Err(ParseError::new("Expected SupSub node"));
    };

    // Here is where we defer to the inner group if it should handle
    // superscripts and subscripts itself.
    if let Some(delegate_builder) = html_builder_delegate(group, options) {
        return delegate_builder(node, options, ctx);
    }

    let value_base = group.base.as_deref();
    let value_super = group.sup.as_deref();
    let value_sub = group.sub.as_deref();

    let base_html = if let Some(base) = value_base {
        build_html::build_group(ctx, base, options, None)?
    } else {
        // When base is None, create an empty span
        make_span(vec![], vec![], Some(options), None).into()
    };

    let mut super_m = None;
    let mut sub_m = None;

    let metrics = options.font_metrics();

    // Rule 18a
    let mut super_shift = 0.0;
    let mut sub_shift = 0.0;

    let is_character_box = value_base.is_some_and(|b| b.is_character_box().unwrap_or(false));

    if let Some(sup_group) = value_super {
        let new_options = options.having_style(options.style.sup());
        super_m = Some(build_html::build_group(
            ctx,
            sup_group,
            &new_options,
            Some(options),
        )?);
        if !is_character_box {
            super_shift = base_html.height()
                - new_options.font_metrics().sup_drop * new_options.size_multiplier
                    / options.size_multiplier;
        }
    }

    if let Some(sub_group) = value_sub {
        let new_options = options.having_style(options.style.sub());
        sub_m = Some(build_html::build_group(
            ctx,
            sub_group,
            &new_options,
            Some(options),
        )?);
        if !is_character_box {
            sub_shift = base_html.depth()
                + new_options.font_metrics().sub_drop * new_options.size_multiplier
                    / options.size_multiplier;
        }
    }

    // Rule 18c
    let min_sup_shift = if options.style == DISPLAY {
        metrics.sup1
    } else if options.style.cramped {
        metrics.sup3
    } else {
        metrics.sup2
    };

    // scriptspace is a font-size-independent size, so scale it
    // appropriately for use as the marginRight.
    let multiplier = options.size_multiplier;
    let margin_right = make_em((0.5 / metrics.pt_per_em) / multiplier);

    let mut margin_left = None;
    if sub_m.is_some() {
        // Subscripts shouldn't be shifted by the base's italic correction.
        // Account for that by shifting the subscript back the appropriate
        // amount. Note we only do this when the base is a single symbol.
        let is_oiint = if let Some(AnyParseNode::Op(op_node)) = value_base
            && let Some(name) = op_node.name()
        {
            matches!(name, "\\oiint" | "\\oiiint")
        } else {
            false
        };

        if matches!(base_html, HtmlDomNode::Symbol(_)) || is_oiint {
            match &base_html {
                HtmlDomNode::Symbol(sym) => {
                    margin_left = Some(make_em(-sym.italic));
                }
                HtmlDomNode::DomSpan(span) => {
                    if let Some(italic) = span.italic {
                        margin_left = Some(make_em(-italic));
                    }
                }
                _ => {}
            }
        }
    }

    let supsub = if let (Some(sup_elem), Some(sub_elem)) = (&super_m, &sub_m) {
        // Both superscript and subscript
        super_shift = super_shift
            .max(min_sup_shift)
            .max(0.25f64.mul_add(metrics.x_height, sup_elem.depth()));
        sub_shift = sub_shift.max(metrics.sub2);

        let rule_width = metrics.default_rule_thickness;

        // Rule 18e
        let max_width = 4.0 * rule_width;
        if (super_shift - sup_elem.depth()) - (sub_elem.height() - sub_shift) < max_width {
            sub_shift = max_width - (super_shift - sup_elem.depth()) + sub_elem.height();
            let psi = 0.8f64.mul_add(metrics.x_height, -(super_shift - sup_elem.depth()));
            if psi > 0.0 {
                super_shift += psi;
                sub_shift -= psi;
            }
        }

        make_v_list(
            VListParam::IndividualShift {
                children: vec![
                    VListElemAndShift {
                        elem: sub_elem.clone(),
                        shift: sub_shift,
                        margin_left,
                        margin_right: Some(margin_right.clone()),
                        wrapper_classes: None,
                        wrapper_style: None,
                    },
                    VListElemAndShift {
                        elem: sup_elem.clone(),
                        shift: -super_shift,
                        margin_left: None,
                        margin_right: Some(margin_right),
                        wrapper_classes: None,
                        wrapper_style: None,
                    },
                ],
            },
            options,
        )?
    } else if let Some(sub_elem) = &sub_m {
        // Rule 18b
        sub_shift = sub_shift
            .max(metrics.sub1)
            .max(0.8f64.mul_add(-metrics.x_height, sub_elem.height()));

        let vlist_elem = vec![VListChild::Elem(Box::new(VListElem {
            elem: sub_elem.clone(),
            shift: None,
            margin_left,
            margin_right: Some(margin_right),
            wrapper_classes: None,
            wrapper_style: None,
        }))];

        make_v_list(
            VListParam::Shift {
                position_data: sub_shift,
                children: vlist_elem,
            },
            options,
        )?
    } else if let Some(sup_elem) = &super_m {
        // Rule 18c, d
        super_shift = super_shift
            .max(min_sup_shift)
            .max(0.25f64.mul_add(metrics.x_height, sup_elem.depth()));

        make_v_list(
            VListParam::Shift {
                position_data: -super_shift,
                children: vec![VListChild::Elem(Box::new(VListElem {
                    elem: sup_elem.clone(),
                    shift: None,
                    margin_left: None,
                    margin_right: Some(margin_right),
                    wrapper_classes: None,
                    wrapper_style: None,
                }))],
            },
            options,
        )?
    } else {
        return Err(ParseError::new("supsub must have either sup or sub."));
    };

    // Wrap the supsub vlist in a span.msupsub to reset text-align.
    let mclass =
        build_html::get_type_of_dom_tree(&base_html, Some(Side::Right)).unwrap_or(DomType::Mord);
    Ok(make_span(
        vec![mclass.as_ref().to_owned()],
        vec![
            base_html,
            make_span(vec!["msupsub".to_owned()], vec![supsub.into()], None, None).into(),
        ],
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for supsub nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::SupSub(group) = node else {
        return Err(ParseError::new("Expected SupSub node"));
    };

    // Is the inner group a relevant horizontal brace?
    let mut is_brace = false;
    let mut is_over = false;
    let is_sup;

    if let Some(base) = group.base.as_deref()
        && let AnyParseNode::HorizBrace(hbrace) = base
    {
        is_sup = group.sup.is_some();
        if is_sup == hbrace.is_over {
            is_brace = true;
            is_over = hbrace.is_over;
        }
    }

    let mut children = if let Some(base) = group.base.as_deref() {
        vec![build_mathml::build_group(ctx, base, options)?]
    } else {
        vec![
            MathNode::builder()
                .node_type(MathNodeType::Mrow)
                .build()
                .into(),
        ]
    };

    if let Some(sub) = &group.sub {
        children.push(build_mathml::build_group(ctx, sub, options)?);
    }

    if let Some(sup) = &group.sup {
        children.push(build_mathml::build_group(ctx, sup, options)?);
    }

    let node_type = if is_brace {
        if is_over {
            MathNodeType::Mover
        } else {
            MathNodeType::Munder
        }
    } else if group.sub.is_none() {
        let base = group.base.as_deref();
        if let Some(base) = base
            && let AnyParseNode::Op(op_node) = base
        {
            if op_node.limits() && (options.style == DISPLAY || op_node.always_handle_sup_sub()) {
                MathNodeType::Mover
            } else {
                MathNodeType::Msup
            }
        } else if let Some(base) = base
            && let AnyParseNode::OperatorName(op_name) = base
        {
            if op_name.always_handle_sup_sub && (op_name.limits || options.style == DISPLAY) {
                MathNodeType::Mover
            } else {
                MathNodeType::Msup
            }
        } else {
            MathNodeType::Msup
        }
    } else if group.sup.is_none() {
        let base = group.base.as_deref();
        if let Some(base) = base
            && let AnyParseNode::Op(op_node) = base
        {
            if op_node.limits() && (options.style == DISPLAY || op_node.always_handle_sup_sub()) {
                MathNodeType::Munder
            } else {
                MathNodeType::Msub
            }
        } else if let Some(base) = base
            && let AnyParseNode::OperatorName(op_name) = base
        {
            if op_name.always_handle_sup_sub && (op_name.limits || options.style == DISPLAY) {
                MathNodeType::Munder
            } else {
                MathNodeType::Msub
            }
        } else {
            MathNodeType::Msub
        }
    } else {
        let base = group.base.as_deref();
        if let Some(base) = base
            && let AnyParseNode::Op(op_node) = base
        {
            if op_node.limits() && options.style == DISPLAY {
                MathNodeType::Munderover
            } else {
                MathNodeType::Msubsup
            }
        } else if let Some(base) = base
            && let AnyParseNode::OperatorName(op_name) = base
        {
            if op_name.always_handle_sup_sub && (options.style == DISPLAY || op_name.limits) {
                MathNodeType::Munderover
            } else {
                MathNodeType::Msubsup
            }
        } else {
            MathNodeType::Msubsup
        }
    };

    Ok(MathNode::builder()
        .node_type(node_type)
        .children(children)
        .build()
        .into())
}

/// Registers supsub functions in the KaTeX context
pub fn define_supsub(ctx: &mut KatexContext) {
    ctx.define_function_builders(NodeType::SupSub, Some(html_builder), Some(mathml_builder));
}
