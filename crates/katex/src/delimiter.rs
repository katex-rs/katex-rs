//! Delimiter handling for KaTeX
//!
//! This module provides functions for creating and sizing delimiters in
//! mathematical expressions, including parentheses, brackets, braces, and other
//! mathematical symbols. It handles the complex logic for choosing appropriate
//! delimiter sizes and styles based on the content they surround.

use phf::{Set, phf_set};

use crate::namespace::KeyMap;

use crate::build_common::{
    VListChild, VListElem, VListKern, VListParam, lookup_symbol, make_span, make_svg_span,
    make_symbol, make_v_list,
};
use crate::dom_tree::{DomSpan, HtmlDomNode, PathNode, SvgChildNode, SvgNode, SymbolNode};
use crate::options::Options;
use crate::style::{SCRIPT, SCRIPTSCRIPT, Style, TEXT};
use crate::svg_geometry::{inner_path, sqrt_path, tall_delim};
use crate::symbols::Mode;
use crate::types::{CssProperty, ParseError, ParseErrorKind};
use crate::units::make_em;
use crate::{CharacterMetrics, KatexContext};

const SIZE_TO_MAX_HEIGHT: [f64; 5] = [0.0, 1.2, 1.8, 2.4, 3.0];

/// Size to max height mapping for delimiter sizing
pub fn size_to_max_height<T>(size: T) -> f64
where
    T: Into<usize>,
{
    let size = size.into().min(SIZE_TO_MAX_HEIGHT.len() - 1); // Clamp size
    SIZE_TO_MAX_HEIGHT[size]
}

/// Padding above the surd vinculum in SVG units
const VB_PAD: f64 = 80.0;

/// Padding above the surd in em units
const EM_PAD: f64 = 0.08;

/// Lap overlap in ems for stacked delimiters
const LAP_IN_EMS: f64 = 0.008;

/// Delimiter types for sizing sequences
#[derive(Debug, Clone)]
pub enum DelimiterType {
    /// Small delimiter using a specific style
    Small(&'static Style),
    /// Large delimiter using a specific size index
    Large(usize),
    /// Stacked delimiter for very tall expressions
    Stack,
}

/// Result of delimiter sizing traversal
#[derive(Debug)]
pub struct DelimiterSizing {
    /// The type of delimiter to use
    pub delimiter_type: DelimiterType,
    /// The calculated height of the delimiter
    pub height: f64,
}

/// Result of sqrt image creation
#[derive(Debug)]
pub struct SqrtImageResult {
    /// The span containing the SVG image of the sqrt symbol
    pub span: DomSpan,
    /// The width of the rule (vinculum) in ems
    pub rule_width: f64,
    /// The advance width of the sqrt symbol in ems
    pub advance_width: f64,
}

/// Get the metrics for a given symbol and font, after transformation
fn get_metrics(
    ctx: &KatexContext,
    symbol: &str,
    font: &str,
    mode: Mode,
) -> Result<CharacterMetrics, ParseError> {
    let replace = ctx.symbols.get_math(symbol).and_then(|s| s.replace);
    let mut buf = [0u8; 4];
    let symbol = replace.map_or(symbol, |s| s.encode_utf8(&mut buf));
    let look_up = lookup_symbol(ctx, symbol, font, mode)?;

    if let Some(look_up) = look_up
        && let Some(metrics) = look_up.metrics
    {
        Ok(metrics)
    } else {
        Err(ParseError::new(ParseErrorKind::UnsupportedSymbolFont {
            symbol: symbol.to_owned(),
            font: font.to_owned(),
        }))
    }
}

/// Puts a delimiter span in a given style, and adds appropriate height, depth,
/// and maxFontSizes.
fn style_wrap(
    delim: HtmlDomNode,
    to_style: &'static Style,
    options: &Options,
    classes: Vec<String>,
) -> DomSpan {
    let new_options = options.having_base_style(Some(to_style));
    let mut combined_classes = classes;
    combined_classes.extend(new_options.sizing_classes(options));
    let mut span = make_span(combined_classes, vec![delim], Some(options), None);

    let multiplier = new_options.size_multiplier / options.size_multiplier;
    span.height *= multiplier;
    span.depth *= multiplier;
    span.max_font_size = new_options.size_multiplier;

    span
}

/// Makes a small delimiter. This is a delimiter that comes in the Main-Regular
/// font, but is restyled to either be in textstyle, scriptstyle, or
/// scriptscriptstyle.
pub fn make_small_delim(
    ctx: &KatexContext,
    delim: &str,
    style: &'static Style,
    center: bool,
    options: &Options,
    mode: Mode,
    classes: Vec<String>,
) -> Result<DomSpan, ParseError> {
    let text = make_symbol(
        ctx,
        delim,
        "Main-Regular",
        mode,
        Some(options),
        Some(classes.clone()),
    )?;
    let mut span = style_wrap(text.into(), style, options, classes);

    if center {
        span = center_span(span, options, TEXT);
    }

    Ok(span)
}

/// Builds a symbol in the given font size (note size is an integer)
fn mathrm_size(
    ctx: &KatexContext,
    value: &str,
    size: usize,
    mode: Mode,
    options: &Options,
) -> Result<SymbolNode, ParseError> {
    let font_name = format!("Size{size}-Regular");
    make_symbol(ctx, value, &font_name, mode, Some(options), None)
}

/// Makes a large delimiter. This is a delimiter that comes in the Size1, Size2,
/// Size3, or Size4 fonts. It is always rendered in textstyle.
pub fn make_large_delim(
    ctx: &KatexContext,
    delim: &str,
    size: usize,
    center: bool,
    options: &Options,
    mode: Mode,
    classes: Vec<String>,
) -> Result<DomSpan, ParseError> {
    let inner = mathrm_size(ctx, delim, size, mode, options)?;
    let mut span = style_wrap(
        make_span(
            vec!["delimsizing".to_owned(), format!("size{size}")],
            vec![inner.into()],
            Some(options),
            None,
        )
        .into(),
        TEXT,
        options,
        classes,
    );

    if center {
        span = center_span(span, options, TEXT);
    }

    Ok(span)
}

/// Center a span around the axis
fn center_span(mut span: DomSpan, options: &Options, style: &'static Style) -> DomSpan {
    let new_options = options.having_base_style(Some(style));
    let shift = (1.0 - options.size_multiplier / new_options.size_multiplier)
        * options.font_metrics().axis_height;

    // Apply the shift to center the span around the axis
    // In KaTeX.js, this modifies the span's style to apply vertical positioning
    // We'll create a modified span with the vertical shift applied
    span.classes.push("delimcenter".to_owned());
    span.height -= shift;
    span.depth += shift;
    span.style.insert(CssProperty::Top, make_em(shift));
    span
}

/// Make a span from a font glyph with the given offset and in the given font.
/// This is used in makeStackedDelim to make the stacking pieces for the
/// delimiter.
fn make_glyph_span(
    ctx: &KatexContext,
    symbol: &str,
    font: &str,
    mode: Mode,
) -> Result<VListChild, ParseError> {
    let size_class = if font == "Size1-Regular" {
        "delim-size1"
    } else {
        "delim-size4"
    };

    let corner = make_span(
        vec!["delimsizinginner".to_owned(), size_class.to_owned()],
        vec![
            make_span(
                vec![],
                vec![make_symbol(ctx, symbol, font, mode, None, None)?.into()],
                None,
                None,
            )
            .into(),
        ],
        None,
        None,
    );

    // Wrap in the appropriate tag that VList uses
    Ok(VListElem::builder().elem(corner.into()).build().into())
}

/// Create a span with inline SVG for the inner part of a tall stacked
/// delimiter.
fn make_inner(
    ctx: &KatexContext,
    ch: &str,
    height: f64,
    options: &Options,
) -> Result<VListChild, ParseError> {
    let Some(first_char) = ch.chars().next() else {
        return Err(ParseError::new(ParseErrorKind::EmptyDelimiterCharacter));
    };
    // Get font metrics data

    let width = if let Some(metric) = ctx
        .font_metrics
        .get_metric("Size4-Regular", first_char as u32)?
    {
        metric.width
    } else {
        ctx.font_metrics
            .get_metric("Size1-Regular", first_char as u32)?
            .map_or(0.0, |metrics| metrics.width)
    };

    let path = PathNode {
        path_name: "inner".to_owned(),
        alternate: Some(inner_path(ch, (1000.0 * height).round())),
    };

    let mut svg_attributes = KeyMap::default();
    svg_attributes.extend([
        ("width".to_owned(), make_em(width)),
        ("height".to_owned(), make_em(height)),
        ("style".to_owned(), format!("width:{}", make_em(width))),
        (
            "viewBox".to_owned(),
            format!("0 0 {} {}", (1000.0 * width), (1000.0 * height).round()),
        ),
        ("preserveAspectRatio".to_owned(), "xMinYMin".to_owned()),
    ]);

    let svg_children = vec![SvgChildNode::Path(path)];

    let svg_node = SvgNode::builder()
        .children(svg_children)
        .attributes(svg_attributes)
        .build();
    let mut span = make_svg_span(vec![], vec![svg_node], options);

    // Set span properties to match JavaScript version
    span.height = height;
    span.style.insert(CssProperty::Height, make_em(height));
    span.style.insert(CssProperty::Width, make_em(width));

    Ok(VListElem::builder().elem(span.into()).build().into())
}

/// Make a stacked delimiter out of a given delimiter, with the total height at
/// least `height_total`. This routine is mentioned on page 442 of the TeXbook.
pub fn make_stacked_delim(
    ctx: &KatexContext,
    delim: &str,
    height_total: f64,
    center: bool,
    options: &Options,
    mode: Mode,
    classes: Vec<String>,
) -> Result<DomSpan, ParseError> {
    // There are four parts, the top, an optional middle, a repeated part, and a
    // bottom.
    let mut top = delim.to_owned();
    let mut middle = None;
    let mut repeat = delim.to_owned();
    let mut bottom = delim.to_owned();
    let mut svg_label = String::new();
    let mut view_box_width = 0.0;
    let mut font = "Size1-Regular".to_owned();

    // Set the parts and font based on the symbol
    match delim {
        "\\uparrow" => {
            "\u{23d0}".clone_into(&mut repeat);
            "\u{23d0}".clone_into(&mut bottom);
        }
        "\\Uparrow" => {
            "\u{2016}".clone_into(&mut repeat);
            "\u{2016}".clone_into(&mut bottom);
        }
        "\\downarrow" => {
            "\u{23d0}".clone_into(&mut top);
            "\u{23d0}".clone_into(&mut repeat);
        }
        "\\Downarrow" => {
            "\u{2016}".clone_into(&mut top);
            "\u{2016}".clone_into(&mut repeat);
        }
        "\\updownarrow" => {
            "\\uparrow".clone_into(&mut top);
            "\u{23d0}".clone_into(&mut repeat);
            "\\downarrow".clone_into(&mut bottom);
        }
        "\\Updownarrow" => {
            "\\Uparrow".clone_into(&mut top);
            "\u{2016}".clone_into(&mut repeat);
            "\\Downarrow".clone_into(&mut bottom);
        }
        "|" | "\\lvert" | "\\rvert" | "\\vert" => {
            "\u{2223}".clone_into(&mut repeat);
            "vert".clone_into(&mut svg_label);
            view_box_width = 333.0;
        }
        "\\|" | "\\lVert" | "\\rVert" | "\\Vert" => {
            "\u{2225}".clone_into(&mut repeat);
            "doublevert".clone_into(&mut svg_label);
            view_box_width = 556.0;
        }
        "[" | "\\lbrack" => {
            "\u{23a1}".clone_into(&mut top);
            "\u{23a2}".clone_into(&mut repeat);
            "\u{23a3}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "lbrack".clone_into(&mut svg_label);
            view_box_width = 667.0;
        }
        "]" | "\\rbrack" => {
            "\u{23a4}".clone_into(&mut top);
            "\u{23a5}".clone_into(&mut repeat);
            "\u{23a6}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "rbrack".clone_into(&mut svg_label);
            view_box_width = 667.0;
        }
        "\\lfloor" | "\u{230a}" => {
            "\u{23a2}".clone_into(&mut top);
            "\u{23a2}".clone_into(&mut repeat);
            "\u{23a3}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "lfloor".clone_into(&mut svg_label);
            view_box_width = 667.0;
        }
        "\\lceil" | "\u{2308}" => {
            "\u{23a1}".clone_into(&mut top);
            "\u{23a2}".clone_into(&mut repeat);
            "\u{23a2}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "lceil".clone_into(&mut svg_label);
            view_box_width = 667.0;
        }
        "\\rfloor" | "\u{230b}" => {
            "\u{23a5}".clone_into(&mut top);
            "\u{23a5}".clone_into(&mut repeat);
            "\u{23a6}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "rfloor".clone_into(&mut svg_label);
            view_box_width = 667.0;
        }
        "\\rceil" | "\u{2309}" => {
            "\u{23a4}".clone_into(&mut top);
            "\u{23a5}".clone_into(&mut repeat);
            "\u{23a5}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "rceil".clone_into(&mut svg_label);
            view_box_width = 667.0;
        }
        "(" | "\\lparen" => {
            "\u{239b}".clone_into(&mut top);
            "\u{239c}".clone_into(&mut repeat);
            "\u{239d}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "lparen".clone_into(&mut svg_label);
            view_box_width = 875.0;
        }
        ")" | "\\rparen" => {
            "\u{239e}".clone_into(&mut top);
            "\u{239f}".clone_into(&mut repeat);
            "\u{23a0}".clone_into(&mut bottom);
            "Size4-Regular".clone_into(&mut font);
            "rparen".clone_into(&mut svg_label);
            view_box_width = 875.0;
        }
        "\\{" | "\\lbrace" => {
            "\u{23a7}".clone_into(&mut top);
            middle = Some("\u{23a8}".to_owned());
            "\u{23a9}".clone_into(&mut bottom);
            "\u{23aa}".clone_into(&mut repeat);
            "Size4-Regular".clone_into(&mut font);
        }
        "\\}" | "\\rbrace" => {
            "\u{23ab}".clone_into(&mut top);
            middle = Some("\u{23ac}".to_owned());
            "\u{23ad}".clone_into(&mut bottom);
            "\u{23aa}".clone_into(&mut repeat);
            "Size4-Regular".clone_into(&mut font);
        }
        "\\lgroup" | "\u{27ee}" => {
            "\u{23a7}".clone_into(&mut top);
            "\u{23a9}".clone_into(&mut bottom);
            "\u{23aa}".clone_into(&mut repeat);
            "Size4-Regular".clone_into(&mut font);
        }
        "\\rgroup" | "\u{27ef}" => {
            "\u{23ab}".clone_into(&mut top);
            "\u{23ad}".clone_into(&mut bottom);
            "\u{23aa}".clone_into(&mut repeat);
            "Size4-Regular".clone_into(&mut font);
        }
        "\\lmoustache" | "\u{23b0}" => {
            "\u{23a7}".clone_into(&mut top);
            "\u{23ad}".clone_into(&mut bottom);
            "\u{23aa}".clone_into(&mut repeat);
            "Size4-Regular".clone_into(&mut font);
        }
        "\\rmoustache" | "\u{23b1}" => {
            "\u{23ab}".clone_into(&mut top);
            "\u{23a9}".clone_into(&mut bottom);
            "\u{23aa}".clone_into(&mut repeat);
            "Size4-Regular".clone_into(&mut font);
        }
        _ => {}
    }

    // Get the metrics of the four sections
    let top_metrics = get_metrics(ctx, &top, &font, mode)?;
    let top_height_total = top_metrics.height + top_metrics.depth;

    let repeat_metrics = get_metrics(ctx, &repeat, &font, mode)?;
    let repeat_height_total = repeat_metrics.height + repeat_metrics.depth;

    let bottom_metrics = get_metrics(ctx, &bottom, &font, mode)?;
    let bottom_height_total = bottom_metrics.height + bottom_metrics.depth;

    let mut middle_height_total = 0.0;
    let middle_factor = if let Some(middle_sym) = &middle {
        let middle_metrics = get_metrics(ctx, middle_sym, &font, mode)?;
        middle_height_total = middle_metrics.height + middle_metrics.depth;
        2.0 // repeat symmetrically above and below middle
    } else {
        1.0
    };

    // Calculate the minimal height that the delimiter can have.
    let minimal_height = top_height_total + bottom_height_total + middle_height_total;

    let delta = (height_total - minimal_height) / (middle_factor * repeat_height_total);
    let repeat_count = delta.ceil().max(0.0) as i32;

    // Compute the total height of the delimiter including all the symbols
    let real_height_total =
        (f64::from(repeat_count) * middle_factor).mul_add(repeat_height_total, minimal_height);

    // Calculate the depth
    let axis_height = options.font_metrics().axis_height;
    let adjusted_axis = if center {
        axis_height * options.size_multiplier
    } else {
        axis_height
    };
    let depth = real_height_total / 2.0 - adjusted_axis;

    // Now, we start building the pieces that will go into the vlist
    let mut stack: Vec<VListChild> = Vec::new();

    if svg_label.is_empty() {
        // Stack glyphs
        // Start by adding the bottom symbol
        stack.push(make_glyph_span(ctx, &bottom, &font, mode)?);
        stack.push(VListChild::Kern(VListKern { size: -LAP_IN_EMS })); // overlap

        if middle.is_none() {
            // The middle section will be an SVG. Make it an extra 0.016em tall.
            let inner_height = 2.0f64.mul_add(
                LAP_IN_EMS,
                real_height_total - top_height_total - bottom_height_total,
            );
            stack.push(make_inner(ctx, &repeat, inner_height, options)?);
        } else {
            // When there is a middle bit, we need the middle part and two repeated sections
            let inner_height = 2.0f64.mul_add(
                LAP_IN_EMS,
                (real_height_total - top_height_total - bottom_height_total - middle_height_total)
                    / 2.0,
            );
            stack.push(make_inner(ctx, &repeat, inner_height, options)?);
            // Now insert the middle of the brace.
            stack.push(VListChild::Kern(VListKern { size: -LAP_IN_EMS }));
            if let Some(middle_sym) = &middle {
                stack.push(make_glyph_span(ctx, middle_sym, &font, mode)?);
            }
            stack.push(VListChild::Kern(VListKern { size: -LAP_IN_EMS }));
            stack.push(make_inner(ctx, &repeat, inner_height, options)?);
        }

        // Add the top symbol
        stack.push(VListChild::Kern(VListKern { size: -LAP_IN_EMS }));
        stack.push(make_glyph_span(ctx, &top, &font, mode)?);
    } else {
        // Instead of stacking glyphs, create a single SVG.
        let mid_height = real_height_total - top_height_total - bottom_height_total;
        let view_box_height = (real_height_total * 1000.0).round();
        let path_str = tall_delim(&svg_label, (mid_height * 1000.0).round())?;
        let path = PathNode {
            path_name: svg_label,
            alternate: Some(path_str),
        };
        // const width = (viewBoxWidth / 1000).toFixed(3) + "em";
        let width = format!("{:.3}em", view_box_width / 1000.0);
        let height = format!("{:.3}em", view_box_height / 1000.0);

        let mut svg_attributes = KeyMap::default();
        svg_attributes.extend([
            ("width".to_owned(), width.clone()),
            ("height".to_owned(), height.clone()),
            (
                "viewBox".to_owned(),
                format!("0 0 {view_box_width} {view_box_height}"),
            ),
        ]);
        let svg_node = SvgNode::builder()
            .children(vec![SvgChildNode::Path(path)])
            .attributes(svg_attributes)
            .build();
        let mut wrapper = make_svg_span(vec![], vec![svg_node], options);
        wrapper.height = view_box_height / 1000.0;
        wrapper.style.insert(CssProperty::Width, width);
        wrapper.style.insert(CssProperty::Height, height);
        stack.push(VListElem::builder().elem(wrapper.into()).build().into());
    }

    // Finally, build the vlist
    let new_options = options.having_base_style(Some(TEXT));
    let inner = make_v_list(
        VListParam::Bottom {
            position_data: depth,
            children: stack,
        },
        &new_options,
    )?;

    let span = style_wrap(
        make_span(
            vec!["delimsizing".to_owned(), "mult".to_owned()],
            vec![inner.into()],
            Some(&new_options),
            None,
        )
        .into(),
        TEXT,
        options,
        classes,
    );

    Ok(span)
}

/// Make a sqrt image of the given height.
pub fn make_sqrt_image(
    ctx: &KatexContext,
    height: f64,
    options: &Options,
) -> Result<SqrtImageResult, ParseError> {
    // Define a newOptions that removes the effect of size changes such as \Huge.
    let new_options = options.having_base_sizing();

    // Pick the desired surd glyph from a sequence of surds.
    let delimiter_type = traverse_sequence(
        ctx,
        "\\surd",
        height * new_options.size_multiplier,
        STACK_LARGE_DELIMITER_SEQUENCE,
        &new_options,
    )?;

    let mut size_multiplier = new_options.size_multiplier;

    // The standard sqrt SVGs each have a 0.04em thick vinculum.
    let extra_vinculum =
        (options.min_rule_thickness - options.font_metrics().sqrt_rule_thickness).max(0.0);

    // Create a span containing an SVG image of a sqrt symbol.
    let span_height;
    let tex_height;
    let view_box_height;
    let advance_width;

    let mut span = match delimiter_type {
        DelimiterType::Small(_style) => {
            // Get an SVG that is derived from glyph U+221A in font KaTeX-Main.
            view_box_height = 1000.0f64.mul_add(extra_vinculum, 1000.0) + VB_PAD;
            if height < 1.0 {
                size_multiplier = 1.0; // mimic a \textfont radical
            } else if height < 1.4 {
                size_multiplier = 0.7; // mimic a \scriptfont radical
            }
            span_height = (1.0 + extra_vinculum + EM_PAD) / size_multiplier;
            tex_height = (1.0 + extra_vinculum) / size_multiplier;
            let span = sqrt_svg(
                "sqrtMain",
                span_height,
                view_box_height as i32,
                extra_vinculum,
                options,
            );

            advance_width = 0.833 / size_multiplier; // from the font.

            span
        }
        DelimiterType::Large(size) => {
            // These SVGs come from fonts: KaTeX_Size1, _Size2, etc.
            view_box_height = (1000.0 + VB_PAD) * SIZE_TO_MAX_HEIGHT[*size];
            tex_height = (SIZE_TO_MAX_HEIGHT[*size] + extra_vinculum) / size_multiplier;
            span_height = (SIZE_TO_MAX_HEIGHT[*size] + extra_vinculum + EM_PAD) / size_multiplier;
            let mut span = sqrt_svg(
                &format!("sqrtSize{size}"),
                span_height,
                view_box_height as i32,
                extra_vinculum,
                options,
            );
            span.style
                .insert(CssProperty::MinWidth, "1.02em".to_owned());
            advance_width = 1.0 / size_multiplier; // 1.0 from the font.

            span
        }
        DelimiterType::Stack => {
            // Tall sqrt. In TeX, this would be stacked using multiple glyphs.
            span_height = height + extra_vinculum + EM_PAD;
            tex_height = height + extra_vinculum;
            view_box_height = 1000.0f64.mul_add(height, extra_vinculum).round();
            let mut span = sqrt_svg(
                "sqrtTall",
                span_height,
                view_box_height as i32,
                extra_vinculum,
                options,
            );
            span.style
                .insert(CssProperty::MinWidth, "0.742em".to_owned());
            advance_width = 1.056;
            span
        }
    };

    span.height = tex_height;
    span.style.insert(CssProperty::Height, make_em(span_height));
    // MinWidth is already set in sqrt_svg

    Ok(SqrtImageResult {
        span,
        rule_width: (options.font_metrics().sqrt_rule_thickness + extra_vinculum) * size_multiplier,
        advance_width,
    })
}

/// Generate SVG path for square root symbols
fn sqrt_svg(
    sqrt_name: &str,
    height: f64,
    view_box_height: i32,
    extra_vinculum: f64,
    options: &Options,
) -> DomSpan {
    let path = sqrt_path(
        sqrt_name,
        1000.0 * extra_vinculum,
        f64::from(view_box_height),
    );
    let path_node = PathNode {
        path_name: sqrt_name.to_owned(),
        alternate: Some(path),
    };

    let svg_children = vec![SvgChildNode::Path(path_node)];
    let mut svg_attributes = KeyMap::default();
    svg_attributes.extend([
        ("width".to_owned(), "400em".to_owned()),
        ("height".to_owned(), make_em(height)),
        (
            "viewBox".to_owned(),
            format!("0 0 400000 {view_box_height}"),
        ),
        (
            "preserveAspectRatio".to_owned(),
            "xMinYMin slice".to_owned(),
        ),
    ]);
    let svg_node = SvgNode::builder()
        .children(svg_children)
        .attributes(svg_attributes)
        .build();
    let mut svg = make_svg_span(vec!["hide-tail".to_owned()], vec![svg_node], options);

    // Set style properties
    svg.style
        .insert(CssProperty::MinWidth, "0.853em".to_owned());
    svg.style.insert(CssProperty::Height, make_em(height));

    svg
}

/// Used to create a delimiter of a specific size, where `size` is 1, 2, 3, or
/// 4.
pub fn sized_delim(
    ctx: &KatexContext,
    delim: &str,
    size: usize,
    options: &Options,
    mode: Mode,
    classes: Vec<String>,
) -> Result<DomSpan, ParseError> {
    // < and > turn into \langle and \rangle in delimiters
    let delim = match delim {
        "<" | "\\lt" | "\u{27e8}" => "\\langle",
        ">" | "\\gt" | "\u{27e9}" => "\\rangle",
        _ => delim,
    };

    // Sized delimiters are never centered.
    if STACK_LARGE_DELIMITERS.contains(delim) || STACK_NEVER_DELIMITERS.contains(delim) {
        make_large_delim(ctx, delim, size, false, options, mode, classes)
    } else if STACK_ALWAYS_DELIMITERS.contains(delim) {
        make_stacked_delim(
            ctx,
            delim,
            SIZE_TO_MAX_HEIGHT[size],
            false,
            options,
            mode,
            classes,
        )
    } else {
        Err(ParseError::new(ParseErrorKind::IllegalDelimiter {
            delim: delim.to_owned(),
        }))
    }
}

/// Traverse a sequence of types of delimiters to decide what kind of delimiter
/// should be used to create a delimiter of the given height+depth.
fn traverse_sequence<'a>(
    ctx: &KatexContext,
    delim: &str,
    height: f64,
    sequence: &'a [DelimiterType],
    options: &Options,
) -> Result<&'a DelimiterType, ParseError> {
    // Here, we choose the index we should start at in the sequences. In smaller
    // sizes (which correspond to larger numbers in style.size) we start earlier
    // in the sequence. Thus, scriptscript starts at index 3-3=0, script starts
    // at index 3-2=1, text starts at 3-1=2, and display starts at min(2,3-0)=2
    let start = (3 - options.style.size).min(2);
    for delim_type in sequence.iter().skip(start) {
        if matches!(delim_type, DelimiterType::Stack) {
            // This is always the last delimiter, so we just break the loop now.
            break;
        }

        let font = delim_type_to_font(delim_type);
        let metrics = get_metrics(ctx, delim, &font, Mode::Math)?;
        let mut height_depth = metrics.height + metrics.depth;

        // Small delimiters are scaled down versions of the same font, so we
        // account for the style change size.
        if let DelimiterType::Small(style) = &delim_type {
            let new_options = options.having_base_style(Some(style));
            height_depth *= new_options.size_multiplier;
        }

        // Check if the delimiter at this size works for the given height.
        if height_depth > height {
            return Ok(delim_type);
        }
    }

    // If we reached the end of the sequence, return the last sequence element.
    Ok(&sequence[sequence.len() - 1])
}

/// Get the font used in a delimiter based on what kind of delimiter it is.
fn delim_type_to_font(delimiter_type: &DelimiterType) -> String {
    match delimiter_type {
        DelimiterType::Small(_) => "Main-Regular".to_owned(),
        DelimiterType::Large(size) => format!("Size{size}-Regular"),
        DelimiterType::Stack => "Size4-Regular".to_owned(),
    }
}

/// Make a delimiter of a given height+depth, with optional centering. Here, we
/// traverse the sequences, and create a delimiter that the sequence tells us
/// to.
pub fn custom_sized_delim(
    ctx: &KatexContext,
    delim: &str,
    height: f64,
    center: bool,
    options: &Options,
    mode: Mode,
    classes: Vec<String>,
) -> Result<DomSpan, ParseError> {
    let delim = match delim {
        "<" | "\\lt" | "\u{27e8}" => "\\langle",
        ">" | "\\gt" | "\u{27e9}" => "\\rangle",
        _ => delim,
    };

    // Decide what sequence to use
    let sequence = if STACK_NEVER_DELIMITERS.contains(delim) {
        STACK_NEVER_DELIMITER_SEQUENCE
    } else if STACK_LARGE_DELIMITERS.contains(delim) {
        STACK_LARGE_DELIMITER_SEQUENCE
    } else {
        STACK_ALWAYS_DELIMITER_SEQUENCE
    };

    // Look through the sequence
    let delimiter_type = traverse_sequence(ctx, delim, height, sequence, options)?;

    // Get the delimiter from font glyphs.
    match delimiter_type {
        DelimiterType::Small(style) => {
            make_small_delim(ctx, delim, style, center, options, mode, classes)
        }
        DelimiterType::Large(size) => {
            make_large_delim(ctx, delim, *size, center, options, mode, classes)
        }
        DelimiterType::Stack => {
            make_stacked_delim(ctx, delim, height, center, options, mode, classes)
        }
    }
}

/// Make a delimiter for use with `\left` and `\right`, given a height and depth
/// of an expression that the delimiters surround.
pub fn left_right_delim(
    ctx: &KatexContext,
    delim: &str,
    height: f64,
    depth: f64,
    options: &Options,
    mode: Mode,
    classes: Vec<String>,
) -> Result<DomSpan, ParseError> {
    // We always center \left/\right delimiters, so the axis is always shifted
    let axis_height = options.font_metrics().axis_height * options.size_multiplier;

    // Taken from TeX source, tex.web, function make_left_right
    let delimiter_factor = 901.0;
    let delimiter_extend = 5.0 / options.font_metrics().pt_per_em;

    let max_dist_from_axis = (height - axis_height).max(depth + axis_height);

    let total_height = (max_dist_from_axis / 500.0 * delimiter_factor)
        .max(2.0f64.mul_add(max_dist_from_axis, -delimiter_extend));

    // Finally, we defer to `makeCustomSizedDelim` with our calculated total
    // height
    custom_sized_delim(ctx, delim, total_height, true, options, mode, classes)
}

// Delimiter classification arrays

const STACK_LARGE_DELIMITERS: Set<&str> = phf_set!(
    "(", "\\lparen", ")", "\\rparen", "[", "\\lbrack", "]", "\\rbrack", "\\{", "\\lbrace", "\\}",
    "\\rbrace", "\\lfloor", "\\rfloor", "\u{230a}", "\u{230b}", "\\lceil", "\\rceil", "\u{2308}",
    "\u{2309}", "\\surd",
);

const STACK_ALWAYS_DELIMITERS: Set<&str> = phf_set!(
    "\\uparrow",
    "\\downarrow",
    "\\updownarrow",
    "\\Uparrow",
    "\\Downarrow",
    "\\Updownarrow",
    "|",
    "\\|",
    "\\vert",
    "\\Vert",
    "\\lvert",
    "\\rvert",
    "\\lVert",
    "\\rVert",
    "\\lgroup",
    "\\rgroup",
    "\u{27ee}",
    "\u{27ef}",
    "\\lmoustache",
    "\\rmoustache",
    "\u{23b0}",
    "\u{23b1}",
);

const STACK_NEVER_DELIMITERS: Set<&str> = phf_set!(
    "<",
    ">",
    "\\langle",
    "\\rangle",
    "/",
    "\\backslash",
    "\\lt",
    "\\gt",
);

const STACK_NEVER_DELIMITER_SEQUENCE: &[DelimiterType] = &[
    DelimiterType::Small(SCRIPTSCRIPT),
    DelimiterType::Small(SCRIPT),
    DelimiterType::Small(TEXT),
    DelimiterType::Large(1),
    DelimiterType::Large(2),
    DelimiterType::Large(3),
    DelimiterType::Large(4),
];

const STACK_ALWAYS_DELIMITER_SEQUENCE: &[DelimiterType] = &[
    DelimiterType::Small(SCRIPTSCRIPT),
    DelimiterType::Small(SCRIPT),
    DelimiterType::Small(TEXT),
    DelimiterType::Stack,
];

const STACK_LARGE_DELIMITER_SEQUENCE: &[DelimiterType] = &[
    DelimiterType::Small(SCRIPTSCRIPT),
    DelimiterType::Small(SCRIPT),
    DelimiterType::Small(TEXT),
    DelimiterType::Large(1),
    DelimiterType::Large(2),
    DelimiterType::Large(3),
    DelimiterType::Large(4),
    DelimiterType::Stack,
];
