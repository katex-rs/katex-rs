//! Accent function implementations for KaTeX Rust
//!
//! This module handles accent symbols in mathematical expressions,
//! migrated from KaTeX's accent.js.

use alloc::vec;

use crate::build_common::{
    self, VEC_SVG_DATA, VListChild, VListElem, VListParam, make_span, make_v_list, static_svg,
};
use crate::build_mathml::make_text;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec, normalize_argument};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeAccent, ParseNodeTextOrd};
use crate::stretchy::{math_ml_node, svg_span};
use crate::types::{
    ArgType, CssProperty, CssStyle, ErrorLocationProvider, Mode, ParseError, ParseErrorKind,
};
use crate::units::make_em;
use crate::{KatexContext, build_html, build_mathml};
use phf::phf_set;

/// Non-stretchy accent commands that should not be stretched to fit their base
static NON_STRETCHY_ACCENTS: phf::Set<&'static str> = phf_set! {
    "\\acute", "\\grave", "\\ddot", "\\tilde", "\\bar", "\\breve",
    "\\check", "\\hat", "\\vec", "\\dot", "\\mathring"
};

/// Math mode accent commands
const MATH_ACCENTS: &[&str] = &[
    "\\acute",
    "\\grave",
    "\\ddot",
    "\\tilde",
    "\\bar",
    "\\breve",
    "\\check",
    "\\hat",
    "\\vec",
    "\\dot",
    "\\mathring",
    "\\widecheck",
    "\\widehat",
    "\\widetilde",
    "\\overrightarrow",
    "\\overleftarrow",
    "\\Overrightarrow",
    "\\overleftrightarrow",
    "\\overgroup",
    "\\overlinesegment",
    "\\overleftharpoon",
    "\\overrightharpoon",
];

/// Text mode accent commands
const TEXT_ACCENTS: &[&str] = &[
    "\\'",
    "\\`",
    "\\^",
    "\\~",
    "\\=",
    "\\u",
    "\\.",
    "\\\"",
    "\\c",
    "\\r",
    "\\H",
    "\\v",
    "\\textcircled",
];

/// Registers accent functions in the KaTeX context
pub fn define_accent(ctx: &mut KatexContext) {
    // Math mode accents
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Accent),
        names: MATH_ACCENTS,
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let base = normalize_argument(&args[0]);
            let is_stretchy = !NON_STRETCHY_ACCENTS.contains(context.func_name);
            let is_shifty = !is_stretchy
                || context.func_name == "\\widehat"
                || context.func_name == "\\widetilde"
                || context.func_name == "\\widecheck";

            Ok(ParseNode::Accent(Box::new(ParseNodeAccent {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                is_stretchy: Some(is_stretchy),
                is_shifty: Some(is_shifty),
                base: base.clone(),
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // Text mode accents
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Accent),
        names: TEXT_ACCENTS,
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            allowed_in_math: true,
            arg_types: Some(vec![ArgType::Primitive]),
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let base = args[0].clone();

            // In math mode, switch to text mode for accents
            let mode = if context.parser.mode == Mode::Math {
                context.parser.settings.report_nonstrict(
                    "mathVsTextAccents",
                    &format!(
                        "LaTeX's accent {} works only in text mode",
                        context.func_name
                    ),
                    context.token.map(|t| t as &dyn ErrorLocationProvider),
                )?;
                Mode::Text
            } else {
                context.parser.mode
            };

            Ok(ParseNode::Accent(Box::new(ParseNodeAccent {
                mode,
                loc: context.loc(),
                label: context.func_name.to_owned(),
                is_stretchy: Some(false),
                is_shifty: Some(true),
                base,
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for accent nodes
/// NOTE: Unlike most `htmlBuilder`s, this one handles not only "accent", but
/// also "supsub" since an accent can affect super/subscripting.
pub fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    // Accents are handled in the TeXbook pg. 443, rule 12.
    let (group, base, supsub_group) = match node {
        ParseNode::Accent(accent_node) => (accent_node, &accent_node.base, None),
        ParseNode::SupSub(supsub) => {
            // If our base is a character box, and we have superscripts and
            // subscripts, the supsub will defer to us. In particular, we want
            // to attach the superscripts and subscripts to the inner body (so
            // that the position of the superscripts and subscripts won't be
            // affected by the height of the accent). We accomplish this by
            // sticking the base of the accent into the base of the supsub, and
            // rendering that, while keeping track of where the accent is.
            if let Some(base) = &supsub.base
                && let ParseNode::Accent(accent) = &**base
            {
                // The real accent group is the base of the supsub group
                let group = accent;
                // The character box is the base of the accent group
                let base = &group.base;
                let mut cloned = supsub.clone();
                // Stick the character box into the base of the supsub group
                cloned.base = Some(Box::new(base.clone()));
                let grp = ParseNode::SupSub(cloned);
                // Rerender the supsub group with its new base, and store that
                // result.
                let supsub_group = build_html::build_group(ctx, &grp, options, None)?;

                (group, base, Some(supsub_group))
            } else {
                return Err(ParseError::new(ParseErrorKind::ExpectedSupSubBaseNode {
                    node: NodeType::Accent,
                }));
            }
        }
        _ => {
            return Err(ParseError::new(ParseErrorKind::ExpectedNodeOrSupSub {
                node: NodeType::Accent,
            }));
        }
    };

    // Build the base group
    let body = build_html::build_group(ctx, base, options, Some(&options.having_cramped_style()))?;
    // Does the accent need to shift for the skew of a character?
    let must_shift = group.is_shifty.unwrap_or(false) && base.is_character_box()?;

    // Calculate the skew of the accent. This is based on the line "If the
    // nucleus is not a single character, let s = 0; otherwise set s to the
    // kern amount for the nucleus followed by the \skewchar of its font."
    // Note that our skew metrics are just the kern between each character
    // and the skewchar.
    let skew = if must_shift {
        // If the base is a character box, then we want the skew of the
        // innermost character. To do that, we find the innermost character:
        let base_char = base.to_base_elem()?;
        // Then, we render its group to get the symbol inside it
        let base_group =
            build_html::build_group(ctx, base_char, &options.having_cramped_style(), None)?;
        // Finally, we pull the skew off of the symbol.
        if let HtmlDomNode::Symbol(symbol) = base_group {
            symbol.skew
        } else {
            return Err(ParseError::new(ParseErrorKind::ExpectedSymbolNode {
                context: "base character",
            }));
        }
        // Note that we now throw away baseGroup, because the layers we
        // removed with getBaseElem might contain things like \color which
        // we can't get rid of.
        // TODO(emily): Find a better way to get the skew
    } else {
        0f64
    };

    let accent_below = group.label == "\\c";

    // calculate the amount of space between the body and the accent
    let mut clearance = if accent_below {
        body.height() + body.depth()
    } else {
        body.height().min(options.font_metrics().x_height)
    };

    // Build the accent
    let accent_body = if group.is_stretchy.unwrap_or(false) {
        let accent_body = svg_span(&ParseNode::Accent(group.clone()), options)?;

        let wrapper_style = (skew > 0.0).then(|| {
            let mut style = CssStyle::default();
            style.insert(
                CssProperty::Width,
                format!("calc(100% - {})", make_em(2.0 * skew)),
            );
            style.insert(CssProperty::MarginLeft, make_em(2.0 * skew));
            style
        });

        let children = vec![
            VListElem::builder().elem(body).build().into(),
            VListElem::builder()
                .elem(accent_body)
                .wrapper_classes(vec!["svg-align".to_owned()])
                .maybe_wrapper_style(wrapper_style)
                .build()
                .into(),
        ];

        make_v_list(VListParam::FirstBaseline { children }, options)?
    } else {
        let (accent, width): (HtmlDomNode, f64) = if group.label == "\\vec" {
            // Before version 0.9, \vec used the combining font glyph U+20D7.
            // But browsers, especially Safari, are not consistent in how they
            // render combining characters when not preceded by a character.
            // So now we use an SVG.
            // If Safari reforms, we should consider reverting to the glyph.
            (static_svg("vec", options)?.into(), VEC_SVG_DATA.0)
        } else {
            // Create accent using makeOrd
            let ord = ParseNode::TextOrd(ParseNodeTextOrd {
                mode: group.mode,
                loc: group.loc.clone(),
                text: group.label.clone(),
            });
            let HtmlDomNode::Symbol(mut accent) = build_common::make_ord(ctx, &ord, options)?
            else {
                return Err(ParseError::new(ParseErrorKind::ExpectedSymbolNode {
                    context: "accent",
                }));
            };

            // Remove the italic correction of the accent, because it only serves to
            // shift the accent over to a place we don't want.
            accent.italic = 0.0;
            if accent_below {
                clearance += accent.depth;
            }

            let width = accent.width;
            (accent.into(), width)
        };

        let mut accent_body = make_span(vec!["accent-body".to_owned()], vec![accent], None, None);

        // "Full" accents expand the width of the resulting symbol to be
        // at least the width of the accent, and overlap directly onto the
        // character without any vertical offset.
        let accent_full = group.label == "\\textcircled";
        if accent_full {
            accent_body.classes.push("accent-full".to_owned());
            clearance = body.height();
        }

        // Shift the accent over by the skew.
        let mut left = skew;

        // CSS defines `.katex .accent .accent-body:not(.accent-full) { width: 0 }`
        // so that the accent doesn't contribute to the bounding box.
        // We need to shift the character by its width (effectively half
        // its width) to compensate.
        if !accent_full {
            left -= width / 2.0;
        }

        accent_body.style.insert(CssProperty::Left, make_em(left));

        // \textcircled uses the \bigcirc glyph, so it needs some
        // vertical adjustment to match LaTeX.
        if group.label == "\\textcircled" {
            accent_body
                .style
                .insert(CssProperty::Top, ".2em".to_owned());
        }

        let children = vec![
            VListElem::builder().elem(body).build().into(),
            VListChild::Kern(build_common::VListKern { size: -clearance }),
            VListElem::builder().elem(accent_body.into()).build().into(),
        ];

        make_v_list(VListParam::FirstBaseline { children }, options)?
    };

    let accent_wrap: HtmlDomNode = make_span(
        vec!["mord".to_owned(), "accent".to_owned()],
        vec![accent_body.into()],
        Some(options),
        None,
    )
    .into();

    if let Some(mut supsub_group) = supsub_group {
        // Get the height before moving accent_wrap
        let accent_wrap_height = accent_wrap.height();

        // Here, we replace the "base" child of the supsub with our newly
        // generated accent.
        if let HtmlDomNode::DomSpan(span) = &mut supsub_group {
            if !span.children.is_empty() {
                span.children[0] = accent_wrap;
            }
            // Since we don't rerun the height calculation after replacing the
            // accent, we manually recalculate height.
            span.height = span.height.max(accent_wrap_height);

            // Accents should always be ords, even when their innards are not.
            if !span.classes.is_empty() {
                "mord".clone_into(&mut span.classes[0]);
            }
        }
        Ok(supsub_group)
    } else {
        Ok(accent_wrap)
    }
}

/// MathML builder for accent nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Accent(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Accent,
        }));
    };

    let accent_node = if group.is_stretchy.unwrap_or(false) {
        // Use stretchy mathML node for stretchy accents
        math_ml_node(&group.label)
    } else {
        let text_node = make_text(&group.label, group.mode, None, &ctx.symbols);
        // Create mo element for non-stretchy accents

        MathNode::builder()
            .node_type(MathNodeType::Mo)
            .children(vec![text_node.into()])
            .build()
    };

    // Build the base group
    let base_group = build_mathml::build_group(ctx, &group.base, options)?;

    // Create mover element
    let mut mover = MathNode::builder()
        .node_type(MathNodeType::Mover)
        .children(vec![base_group, MathDomNode::Math(accent_node)])
        .build();

    mover
        .attributes
        .insert("accent".to_owned(), "true".to_owned());

    Ok(MathDomNode::Math(mover))
}
