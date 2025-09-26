//! Utility functions for assembling superscript and subscript in mathematical
//! expressions.
//!
//! This module provides functions to properly position and render superscript
//! and subscript elements relative to their base expressions, particularly for
//! operators with limits.
//!
//! Migrated from KaTeX's assembleSupSub.js.

use crate::build_common::{VListChild, VListElem, VListKern, VListParam, make_span, make_v_list};
use crate::dom_tree::HtmlDomNode;
use crate::options::Options;
use crate::parser::parse_node::ParseNode;
use crate::style::Style;
use crate::types::{CssProperty, ParseError};
use crate::units::make_em;
use crate::{KatexContext, build_html};

/// Helper struct for superscript/subscript elements with kerning information.
struct SupSubElem {
    elem: HtmlDomNode,
    kern: f64,
}

/// Assembles superscript and subscript elements with their base for operators
/// with limits.
///
/// This function creates a vertical list layout containing the base,
/// superscript, and/or subscript elements with proper spacing and positioning.
/// It's primarily used for operators that can have limits
/// (subscripts/superscripts above/below rather than beside the operator).
///
/// # Parameters
///
/// * `ctx` - The KaTeX context
/// * `base` - The base HTML element (DomSpan or SymbolNode)
/// * `super_group` - Optional superscript parse node
/// * `sub_group` - Optional subscript parse node
/// * `options` - Rendering options
/// * `style` - Style context for the elements
/// * `slant` - Horizontal slant adjustment for positioning
/// * `base_shift` - Vertical shift adjustment for the base element
///
/// # Returns
///
/// A `Result` containing the assembled `HtmlDomNode` with proper spacing and
/// positioning, or a `ParseError` if rendering fails.
#[expect(clippy::too_many_arguments)]
pub fn assemble_sup_sub(
    ctx: &KatexContext,
    base: HtmlDomNode,
    super_group: Option<&ParseNode>,
    sub_group: Option<&ParseNode>,
    options: &Options,
    style: &Style,
    slant: f64,
    base_shift: f64,
) -> Result<HtmlDomNode, ParseError> {
    // Wrap base in a span if it's not already
    let base = make_span(vec![], vec![base], Some(options), None);

    // Check if sub_group is a single character box
    let sub_is_single_character = sub_group
        .as_ref()
        .is_some_and(|sub| sub.is_character_box().unwrap_or(false));

    // Build superscript if present
    let sup = if let Some(sup_group) = super_group {
        let elem = build_html::build_group(
            ctx,
            sup_group,
            &options.having_style(style.sup()),
            Some(options),
        )?;
        let elem_depth = elem.depth();
        Some(SupSubElem {
            elem,
            kern: options
                .font_metrics()
                .big_op_spacing1
                .max(options.font_metrics().big_op_spacing3 - elem_depth),
        })
    } else {
        None
    };

    // Build subscript if present
    let sub = if let Some(sub_group) = sub_group {
        let elem = build_html::build_group(
            ctx,
            sub_group,
            &options.having_style(style.sub()),
            Some(options),
        )?;
        let elem_height = elem.height();
        Some(SupSubElem {
            elem,
            kern: options
                .font_metrics()
                .big_op_spacing2
                .max(options.font_metrics().big_op_spacing4 - elem_height),
        })
    } else {
        None
    };

    // Create the final layout based on sup/sub combination
    let final_group = match (&sup, &sub) {
        (Some(sup), Some(sub)) => {
            // Both superscript and subscript
            let bottom = options.font_metrics().big_op_spacing5
                + sub.elem.height()
                + sub.elem.depth()
                + sub.kern
                + base.depth
                + base_shift;

            make_v_list(
                VListParam::Bottom {
                    position_data: bottom,
                    children: vec![
                        VListChild::Kern(VListKern {
                            size: options.font_metrics().big_op_spacing5,
                        }),
                        VListChild::Elem(Box::new(VListElem {
                            elem: sub.elem.clone(),
                            shift: None,
                            margin_left: Some(make_em(-slant)),
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                        VListChild::Kern(VListKern { size: sub.kern }),
                        VListChild::Elem(Box::new(VListElem {
                            elem: base.into(),
                            shift: None,
                            margin_left: None,
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                        VListChild::Kern(VListKern { size: sup.kern }),
                        VListChild::Elem(Box::new(VListElem {
                            elem: sup.elem.clone(),
                            shift: None,
                            margin_left: Some(make_em(slant)),
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                        VListChild::Kern(VListKern {
                            size: options.font_metrics().big_op_spacing5,
                        }),
                    ],
                },
                options,
            )?
        }
        (None, Some(sub)) => {
            // Only subscript
            let top = base.height - base_shift;

            make_v_list(
                VListParam::Top {
                    position_data: top,
                    children: vec![
                        VListChild::Kern(VListKern {
                            size: options.font_metrics().big_op_spacing5,
                        }),
                        VListChild::Elem(Box::new(VListElem {
                            elem: sub.elem.clone(),
                            shift: None,
                            margin_left: Some(make_em(-slant)),
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                        VListChild::Kern(VListKern { size: sub.kern }),
                        VListChild::Elem(Box::new(VListElem {
                            elem: base.into(),
                            shift: None,
                            margin_left: None,
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                    ],
                },
                options,
            )?
        }
        (Some(sup), None) => {
            // Only superscript
            let bottom = base.depth + base_shift;

            make_v_list(
                VListParam::Bottom {
                    position_data: bottom,
                    children: vec![
                        VListChild::Elem(Box::new(VListElem {
                            elem: base.into(),
                            shift: None,
                            margin_left: None,
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                        VListChild::Kern(VListKern { size: sup.kern }),
                        VListChild::Elem(Box::new(VListElem {
                            elem: sup.elem.clone(),
                            shift: None,
                            margin_left: Some(make_em(slant)),
                            margin_right: None,
                            wrapper_classes: None,
                            wrapper_style: None,
                        })),
                        VListChild::Kern(VListKern {
                            size: options.font_metrics().big_op_spacing5,
                        }),
                    ],
                },
                options,
            )?
        }
        (None, None) => {
            // No superscript or subscript - return base
            return Ok(base.into());
        }
    };

    // Handle spacing adjustments for slant
    let mut parts = vec![final_group.into()];
    if sub.is_some() && slant != 0.0 && !sub_is_single_character {
        // Add spacer to prevent overlap
        let mut spacer = make_span(vec!["mspace".to_owned()], vec![], Some(options), None);
        spacer
            .style
            .insert(CssProperty::MarginRight, make_em(slant));
        parts.insert(0, spacer.into());
    }

    Ok(make_span(
        vec!["mop".to_owned(), "op-limits".to_owned()],
        parts,
        Some(options),
        None,
    )
    .into())
}
