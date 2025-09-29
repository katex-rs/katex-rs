//! Array environment implementation for KaTeX Rust
//!
//! This module handles various array-like environments including matrices,
//! aligned equations, cases, and other tabular mathematical constructs.
//!
//! Migrated from KaTeX's array.js.

use crate::build_common::{
    VListElemAndShift, VListParam, make_fragment, make_line_span, make_span, make_v_list,
};
use crate::define_environment::cd::parse_cd;
use crate::define_environment::{EnvContext, EnvDefSpec, EnvHandler, EnvProps};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::macros::{MacroContextInterface as _, MacroDefinition};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::Parser;
use crate::parser::parse_node::{
    AlignSpec, AnyParseNode, ColSeparationType, NodeType, ParseNode, ParseNodeArray,
    ParseNodeArrayTag, ParseNodeLeftRight, ParseNodeOrdGroup, ParseNodeStyling,
    check_symbol_node_type,
};
use crate::spacing_data::Measurement;
use crate::style::{DISPLAY, SCRIPT, Style, TEXT};
use crate::types::{BreakToken, CssProperty, ParseError, ParseErrorKind, Token};
use crate::utils::{push_and_get_mut, push_and_get_ref};
use crate::{ClassList, KatexContext, build_html, build_mathml, units};
use alloc::borrow::Cow;
use core::iter::repeat_n;
// Type definitions for array environment

/// Helper function to get horizontal lines from parser
fn get_hlines(parser: &mut Parser) -> Result<Vec<bool>, ParseError> {
    // Return an array. The array length = number of hlines.
    // Each element in the array tells if the line is dashed.
    let mut hline_info = Vec::new();
    parser.gullet.consume_spaces()?;

    let mut nxt = parser.fetch()?.text.to_owned_string();
    if nxt == "\\relax" {
        // \relax is an artifact of the \cr macro below
        parser.consume();
        parser.gullet.consume_spaces()?;
        parser.fetch()?.text.clone_into(&mut nxt);
    }

    while nxt == "\\hline" || nxt == "\\hdashline" {
        parser.consume();
        hline_info.push(nxt == "\\hdashline");
        parser.gullet.consume_spaces()?;
        parser.fetch()?.text.clone_into(&mut nxt);
    }

    Ok(hline_info)
}

/// Validates AMS environment context (must be in display mode)
fn validate_ams_environment_context(context: &EnvContext) -> Result<(), ParseError> {
    if !context.parser.settings.display_mode {
        return Err(ParseError::new(ParseErrorKind::DisplayModeOnly {
            env: context.env_name.clone(),
        }));
    }
    Ok(())
}

/// Determines auto-tagging behavior for environments
fn get_auto_tag(name: &str) -> Option<bool> {
    if name.contains("ed") {
        // return undefined;
        None
    } else {
        Some(!name.contains('*'))
    }
}

/// Parse the body of the environment, with rows delimited by \\ and
/// columns delimited by &, and create a nested list in row-major order
/// with one group per cell.
pub fn parse_array(
    parser: &mut Parser,
    config: ArrayParseConfig,
    style: &'static Style,
) -> Result<ParseNodeArray, ParseError> {
    parser.gullet.begin_group();

    if !config.single_row {
        // \cr is equivalent to \\ without the optional size argument (see below)
        // TODO: provide helpful error when \cr is used outside array environment
        parser.gullet.macros_mut().set(
            "\\cr",
            Some(MacroDefinition::StaticStr("\\\\\\relax")),
            false,
        );
    }

    // Get current arraystretch if it's not set by the environment
    let arraystretch = if let Some(stretch) = config.arraystretch {
        stretch
    } else {
        // Get current arraystretch if it's not set by the environment
        let stretch = parser.gullet.expand_macro_as_text("\\arraystretch")?;
        if let Some(stretch) = stretch {
            let stretch_val = stretch.parse::<f64>().map_err(|_| {
                ParseError::new(ParseErrorKind::InvalidArrayStretch {
                    stretch: stretch.clone(),
                })
            })?;
            if stretch_val <= 0.0 {
                return Err(ParseError::new(ParseErrorKind::InvalidArrayStretch {
                    stretch,
                }));
            }
            stretch_val
        } else {
            // Default \arraystretch from lttab.dtx
            1.0
        }
    };

    // Start group for first cell
    parser.gullet.begin_group();

    let mut body = vec![Vec::new()];
    let mut row = &mut body[0];
    let mut row_gaps = Vec::new();
    let mut h_lines_before_row = Vec::new();

    let mut tags = config
        .auto_tag
        .is_some()
        .then(Vec::<ParseNodeArrayTag>::new);

    // amsmath uses \global\@eqnswtrue and \global\@eqnswfalse to represent
    // whether this row should have an equation number.  Simulate this with
    // a \@eqnsw macro set to 1 or 0.
    let begin_row = |parser: &mut Parser| -> Result<(), ParseError> {
        if config.auto_tag == Some(true) {
            parser
                .gullet
                .macros_mut()
                .set("\\@eqnsw", Some(MacroDefinition::StaticStr("1")), true);
        }
        Ok(())
    };

    let mut end_row = |parser: &mut Parser| -> Result<(), ParseError> {
        if let Some(ref mut tags) = tags {
            if parser.gullet.macros().get("\\df@tag").is_some() {
                let node = parser.subparse(vec![Token::new("\\df@tag", None)])?;
                tags.push(node.into());
                parser.gullet.macros_mut().set("\\df@tag", None, true);
            } else {
                let flag = parser
                    .gullet
                    .macros()
                    .get("\\@eqnsw")
                    .is_some_and(|definition| definition.as_str() == Some("1"));
                let val = config.auto_tag.unwrap_or(false) && flag;
                tags.push(val.into());
            }
        }
        Ok(())
    };

    begin_row(parser)?;

    // Test for \hline at the top of the array.
    h_lines_before_row.push(get_hlines(parser)?);

    loop {
        // Parse each cell in its own group (namespace)
        let break_token = if config.single_row {
            Some(&BreakToken::End)
        } else {
            Some(&BreakToken::DoubleBackslash)
        };
        let cell = parser.parse_expression(false, break_token)?;
        parser.gullet.end_group()?;
        parser.gullet.begin_group();

        let cell = ParseNode::Styling(ParseNodeStyling {
            mode: parser.mode,
            loc: None,
            style,
            body: vec![ParseNode::OrdGroup(ParseNodeOrdGroup {
                mode: parser.mode,
                loc: None,
                body: cell,
                semisimple: None,
            })],
        });

        let (cell, row_immut) = push_and_get_ref(row, cell);
        let next = parser.fetch()?.text.to_owned_string();

        match next.as_str() {
            "&" => {
                if let Some(max_num_cols) = config.max_num_cols
                    && row.len() == max_num_cols
                {
                    if config.single_row || config.col_separation_type.is_some() {
                        // {equation} or {split}
                        return Err(ParseError::new(ParseErrorKind::TooManyTabCharacters));
                    }
                    // {array} environment
                    parser.settings.report_nonstrict(
                        "textEnv",
                        "Too few columns specified in the {array} column argument.",
                        None,
                    )?;
                }
                parser.consume();
            }
            "\\end" => {
                end_row(parser)?;
                // Arrays terminate newlines with `\crcr` which consumes a `\cr` if
                // the last line is empty.  However, AMS environments keep the
                // empty row if it's the only one.
                // NOTE: Currently, `cell` is the last item added into `row`.
                if row_immut.len() == 1
                    && let ParseNode::Styling(styling) = cell
                    && styling.body.len() == 1
                    && let ParseNode::OrdGroup(ordgroup) = &styling.body[0]
                    && ordgroup.body.is_empty()
                    && (body.len() > 1 || !config.empty_single_row.unwrap_or(false))
                {
                    body.pop();
                }
                if h_lines_before_row.len() < body.len() + 1 {
                    h_lines_before_row.push(vec![]);
                }
                break;
            }
            "\\\\" => {
                parser.consume();
                let size = if parser.gullet.future_mut()?.text == " " {
                    None
                } else {
                    parser.parse_size_group(true)?
                };
                row_gaps.push(size.map(|s| s.value));
                end_row(parser)?;

                // check for \hline(s) following the row separator
                h_lines_before_row.push(get_hlines(parser)?);

                row = push_and_get_mut(&mut body, Vec::new());
                begin_row(parser)?;
            }
            _ => {
                return Err(ParseError::new(ParseErrorKind::ExpectedArrayDelimiter {
                    found: next,
                }));
            }
        }
    }

    // End cell group
    parser.gullet.end_group()?;
    // End array group defining \cr
    parser.gullet.end_group()?;

    Ok(ParseNodeArray {
        mode: parser.mode,
        loc: None,
        add_jot: config.add_jot,
        arraystretch,
        body,
        cols: config.cols,
        row_gaps,
        hskip_before_and_after: config.hskip_before_and_after,
        h_lines_before_row,
        col_separation_type: config.col_separation_type,
        tags,
        leqno: config.leqno,
        is_cd: None,
    })
}

/// Configuration for parsing array environments
#[derive(Debug, Clone, Default)]
pub struct ArrayParseConfig {
    pub hskip_before_and_after: Option<bool>,
    pub add_jot: Option<bool>,
    pub cols: Option<Vec<AlignSpec>>,
    pub arraystretch: Option<f64>,
    pub col_separation_type: Option<ColSeparationType>,
    pub auto_tag: Option<bool>,
    pub single_row: bool,
    pub empty_single_row: Option<bool>,
    pub max_num_cols: Option<usize>,
    pub leqno: Option<bool>,
}

/// Decides on a style for cells in an array according to whether the given
/// environment name starts with the letter 'd'.
fn d_cell_style(env_name: &str) -> &'static Style {
    if env_name.starts_with('d') {
        DISPLAY
    } else {
        TEXT
    }
}

// HTML and MathML builders will be implemented next
/// HTML builder for array nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Array(array_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Array,
        }));
    };

    let nr = array_node.body.len();
    let h_lines_before_row = &array_node.h_lines_before_row;
    let mut nc = 0;

    let mut body = Vec::with_capacity(nr);
    let mut hlines = Vec::new();

    let rule_thickness = options
        .font_metrics()
        .array_rule_width
        .max(options.min_rule_thickness);

    // Horizontal spacing
    let pt = 1.0 / options.font_metrics().pt_per_em;

    let arraycolsep = if array_node.col_separation_type == Some(ColSeparationType::Small) {
        // We're in a {smallmatrix}. Default column space is \thickspace,
        // i.e. 5/18em = 0.2778em, per amsmath.dtx for {smallmatrix}.
        // But that needs adjustment because LaTeX applies \scriptstyle to the
        // entire array, including the colspace, but this function applies
        // \scriptstyle only inside each element.
        let local_multiplier = options.having_style(SCRIPT).size_multiplier;
        0.2778 * (local_multiplier / options.size_multiplier)
    } else {
        // default value, i.e. \arraycolsep in article.cls
        5.0 * pt
    };

    // Vertical spacing
    let baselineskip = if array_node.col_separation_type == Some(ColSeparationType::CD) {
        ctx.calculate_size(
            &Measurement {
                number: 3.0,
                unit: "ex",
            },
            options,
        )?
    } else {
        12.0 * pt // see size10.clo
    };
    // Default \jot from ltmath.dtx
    // TODO(edemaine): allow overriding \jot via \setlength (#687)
    let jot = 3.0 * pt;
    let arrayskip = array_node.arraystretch * baselineskip;
    let arstrut_height = 0.7 * arrayskip; // \strutbox in ltfsstrc.dtx and
    let arstrut_depth = 0.3 * arrayskip; // \@arstrutbox in lttab.dtx

    let mut total_height = 0.0;

    // Set a position for \hline(s) at the top of the array, if any.
    set_hline_pos(&mut hlines, &mut total_height, &h_lines_before_row[0]);

    for r in 0..array_node.body.len() {
        let inrow = &array_node.body[r];
        let mut height = arstrut_height; // \@array adds an \@arstrut
        let mut depth = arstrut_depth; // to each tow (via the template)

        if nc < inrow.len() {
            nc = inrow.len();
        }

        let mut row_elements: Vec<Option<HtmlDomNode>> = Vec::with_capacity(inrow.len());
        for group in inrow {
            let elt = build_html::build_group(ctx, group, options, None)?;
            depth = depth.max(elt.depth());
            height = height.max(elt.height());
            row_elements.push(Some(elt));
        }

        let row_gap = array_node.row_gaps.get(r);
        let mut gap = 0.0;
        if let Some(row_gap) = row_gap
            && let Some(gap_val) = row_gap
        {
            gap = ctx.calculate_size(gap_val, options)?;
            if gap > 0.0 {
                // \@argarraycr
                gap += arstrut_depth;
                if depth < gap {
                    depth = gap; // \@xargarraycr
                }
                gap = 0.0;
            }
        }
        // In AMS multiline environments such as aligned and gathered, rows
        // correspond to lines that have additional \jot added to the
        // \baselineskip via \openup.
        if array_node.add_jot.unwrap_or(false) {
            depth += jot;
        }

        body.push(Outrow {
            elements: row_elements,
            height,
            depth,
            pos: total_height + height,
        });

        total_height += height;
        total_height += depth + gap; // \@yargarraycr

        // Set a position for \hline(s), if any.
        if r + 1 < h_lines_before_row.len() {
            set_hline_pos(&mut hlines, &mut total_height, &h_lines_before_row[r + 1]);
        }
    }

    let offset = total_height / 2.0 + options.font_metrics().axis_height;
    let col_descriptions = array_node.cols.as_deref().unwrap_or_default();
    let mut cols = Vec::new();
    let mut col_sep;
    // let mut col_descr_num;

    let mut tag_spans = Vec::new();
    if let Some(tags) = &array_node.tags
        && tags.iter().any(ParseNodeArrayTag::is_true)
    {
        // An environment with manual tags and/or automatic equation numbers.
        // Create node(s), the latter of which trigger CSS counter increment.
        for r in 0..nr {
            let rw = &body[r];
            let shift = rw.pos - offset;
            let tag = &tags[r];
            let mut tag_span = match tag {
                ParseNodeArrayTag::Bool(true) => make_span("eqn-num", vec![], Some(options), None),
                ParseNodeArrayTag::Bool(false) => {
                    make_span(ClassList::Empty, vec![], Some(options), None)
                }
                ParseNodeArrayTag::Nodes(nodes) => {
                    let tag_expr = build_html::build_expression(
                        ctx,
                        nodes,
                        options,
                        build_html::GroupType::True,
                        (None, None),
                    )?;
                    make_span(ClassList::Empty, tag_expr, Some(options), None)
                }
            };

            tag_span.depth = rw.depth;
            tag_span.height = rw.height;

            tag_spans.push(
                VListElemAndShift::builder()
                    .elem(tag_span.into())
                    .shift(shift)
                    .build(),
            );
        }
    }

    let mut c = 0;
    let mut col_descr_num = 0;
    while c < nc || col_descr_num < col_descriptions.len() {
        let mut first_separator = true;
        loop {
            let Some(separator) = col_descriptions
                .get(col_descr_num)
                .and_then(|spec| match spec {
                    AlignSpec::Separator { separator } => Some(separator.as_str()),
                    AlignSpec::Align { .. } => None,
                })
            else {
                break;
            };

            if !first_separator {
                col_sep = make_span("arraycolsep", vec![], None, None);
                col_sep.style.insert(
                    CssProperty::Width,
                    units::make_em(options.font_metrics().double_rule_sep),
                );
                cols.push(col_sep.into());
            }

            if separator == "|" || separator == ":" {
                let line_type = if separator == "|" { "solid" } else { "dashed" };
                let mut separator_span =
                    make_span("vertical-separator", vec![], Some(options), None);
                separator_span
                    .style
                    .insert(CssProperty::Height, units::make_em(total_height));
                separator_span.style.insert(
                    CssProperty::BorderRightWidth,
                    units::make_em(rule_thickness),
                );
                separator_span
                    .style
                    .insert(CssProperty::BorderRightStyle, line_type.to_owned());
                separator_span.style.insert(
                    CssProperty::Margin,
                    format!("0 {}", units::make_em(-rule_thickness / 2.0)),
                );
                let shift = total_height - offset;
                if shift != 0.0 {
                    separator_span
                        .style
                        .insert(CssProperty::VerticalAlign, units::make_em(-shift));
                }

                cols.push(separator_span.into());
            } else {
                return Err(ParseError::new(ParseErrorKind::InvalidSeparatorType {
                    separator: separator.to_owned(),
                }));
            }

            col_descr_num += 1;
            first_separator = false;
        }

        if c >= nc {
            c += 1;
            col_descr_num += 1;
            continue;
        }

        let col_descr = col_descriptions.get(col_descr_num);

        let mut sepwidth = if c > 0 || array_node.hskip_before_and_after.unwrap_or(false) {
            col_descr
                .and_then(|cd| match cd {
                    AlignSpec::Align { pregap, .. } => *pregap,
                    AlignSpec::Separator { .. } => None,
                })
                .unwrap_or(arraycolsep)
        } else {
            0.0
        };

        if sepwidth != 0.0 {
            col_sep = make_span("arraycolsep", vec![], None, None);
            col_sep
                .style
                .insert(CssProperty::Width, units::make_em(sepwidth));
            cols.push(col_sep.into());
        }

        let mut col_elements = Vec::new();
        for row in body.iter_mut().take(nr) {
            if let Some(slot) = row.elements.get_mut(c) {
                let Some(mut elem) = slot.take() else {
                    continue;
                };
                let shift = row.pos - offset;
                if let Some(height_mut) = elem.height_mut() {
                    *height_mut = row.height;
                }
                if let Some(depth_mut) = elem.depth_mut() {
                    *depth_mut = row.depth;
                }
                col_elements.push(VListElemAndShift::builder().elem(elem).shift(shift).build());
            }
        }

        let col_vlist = make_v_list(
            VListParam::IndividualShift {
                children: col_elements.into_iter().collect(),
            },
            options,
        )?;

        let col_align = col_descr
            .and_then(|cd| match cd {
                AlignSpec::Align { align, .. } => Some(align.clone()),
                AlignSpec::Separator { .. } => None,
            })
            .unwrap_or_else(|| "c".to_owned());

        let col_span = make_span(
            vec![Cow::Owned(format!("col-align-{col_align}"))],
            vec![col_vlist.into()],
            None,
            None,
        );
        cols.push(col_span.into());

        if c < nc - 1 || array_node.hskip_before_and_after.unwrap_or(false) {
            sepwidth = col_descr
                .and_then(|cd| match cd {
                    AlignSpec::Align { postgap, .. } => *postgap,
                    AlignSpec::Separator { .. } => None,
                })
                .unwrap_or(arraycolsep);

            if sepwidth != 0.0 {
                col_sep = make_span("arraycolsep", vec![], None, None);
                col_sep
                    .style
                    .insert(CssProperty::Width, units::make_em(sepwidth));
                cols.push(col_sep.into());
            }
        }

        c += 1;
        col_descr_num += 1;
    }

    let mut mtable = make_span("mtable", cols, None, None);

    // Add \hline(s), if any.
    if !hlines.is_empty() {
        let line = make_line_span("hline", options, Some(rule_thickness));
        let dashes = make_line_span("hdashline", options, Some(rule_thickness));
        let mut v_list_elems = vec![
            VListElemAndShift::builder()
                .elem(mtable.into())
                .shift(0.0)
                .build(),
        ];

        while let Some(hline) = hlines.pop() {
            let line_shift = hline.pos - offset;
            let line_elem = if hline.is_dashed {
                dashes.clone()
            } else {
                line.clone()
            };
            v_list_elems.push(
                VListElemAndShift::builder()
                    .elem(line_elem.into())
                    .shift(line_shift)
                    .build(),
            );
        }

        mtable = make_v_list(
            VListParam::IndividualShift {
                children: v_list_elems,
            },
            options,
        )?;
    }

    if tag_spans.is_empty() {
        Ok(make_span("mord", vec![mtable.into()], Some(options), None).into())
    } else {
        let eqn_num_col = make_v_list(
            VListParam::IndividualShift {
                children: tag_spans.into_iter().collect(),
            },
            options,
        )?;
        let tag_span = make_span("tag", vec![eqn_num_col.into()], Some(options), None);
        Ok(make_fragment(vec![mtable.into(), tag_span.into()]).into())
    }
}

/// Helper structure for row layout
#[derive(Debug, Clone)]
struct Outrow {
    // Equivalent to `[idx: number]: *` in Javascript
    elements: Vec<Option<HtmlDomNode>>,
    height: f64,
    depth: f64,
    pos: f64,
}

/// Helper structure for horizontal lines
#[derive(Debug, Clone)]
struct Hline {
    pos: f64,
    is_dashed: bool,
}

/// Set a position for \hline(s)
fn set_hline_pos(hlines: &mut Vec<Hline>, total_height: &mut f64, hlines_in_gap: &[bool]) {
    for (i, &is_dashed) in hlines_in_gap.iter().enumerate() {
        if i > 0 {
            *total_height += 0.25;
        }
        hlines.push(Hline {
            pos: *total_height,
            is_dashed,
        });
    }
}

// Environment definitions will be implemented next
// This is a large file, so I'll implement it in parts
/// MathML builder for array nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Array(array_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Array,
        }));
    };

    let mut tbl = Vec::new();
    let glue = MathNode::builder()
        .node_type(MathNodeType::Mtd)
        .classes("mtr-glue".into())
        .build();
    let tag = MathNode::builder()
        .node_type(MathNodeType::Mtd)
        .classes("mml-eqn-num".into())
        .build();

    for i in 0..array_node.body.len() {
        let rw = &array_node.body[i];
        let mut row = Vec::new();

        for group in rw {
            row.push(
                MathNode::builder()
                    .node_type(MathNodeType::Mtd)
                    .children(vec![build_mathml::build_group(ctx, group, options)?])
                    .build(),
            );
        }

        if let Some(tags) = &array_node.tags
            && tags[i].is_true()
        {
            row.insert(0, glue.clone());
            row.push(glue.clone());
            if array_node.leqno.unwrap_or(false) {
                row.insert(0, tag.clone());
            } else {
                row.push(tag.clone());
            }
        }

        tbl.push(
            MathNode::builder()
                .node_type(MathNodeType::Mtr)
                .children(row.into_iter().map(MathDomNode::Math).collect())
                .build(),
        );
    }

    let mut table = MathNode::builder()
        .node_type(MathNodeType::Mtable)
        .children(tbl.into_iter().map(MathDomNode::Math).collect())
        .build();

    // Set column alignment, row spacing, column spacing, and
    // array lines by setting attributes on the table element.

    // Set the row spacing. In MathML, we specify a gap distance.
    // We do not use rowGap[] because MathML automatically increases
    // cell height with the height/depth of the element content.

    // LaTeX \arraystretch multiplies the row baseline-to-baseline distance.
    // We simulate this by adding (arraystretch - 1)em to the gap. This
    // does a reasonable job of adjusting arrays containing 1 em tall content.

    // The 0.16 and 0.09 values are found empirically. They produce an array
    // similar to LaTeX and in which content does not interfere with \hlines.
    let gap = if array_node.arraystretch == 0.5 {
        // {smallmatrix}, {subarray}
        0.1
    } else {
        0.16 + array_node.arraystretch - 1.0
            + if array_node.add_jot.unwrap_or(false) {
                0.09
            } else {
                0.0
            }
    };
    table
        .attributes
        .insert("rowspacing".to_owned(), units::make_em(gap));

    // MathML table lines go only between cells.
    // To place a line on an edge we'll use <menclose>, if necessary.
    let mut menclose = String::new();
    let mut align = String::new();

    if let Some(cols) = &array_node.cols
        && !cols.is_empty()
    {
        // Find column alignment, column spacing, and  vertical lines.
        let mut column_lines = String::new();
        let mut prev_type_was_align = false;
        let mut i_start = 0;
        let i_end = cols.len();

        if let Some(first_col) = cols.first()
            && matches!(first_col, AlignSpec::Separator { .. })
        {
            menclose.push_str("top ");
            i_start = 1;
        }

        if let Some(last_col) = cols.last()
            && matches!(last_col, AlignSpec::Separator { .. })
        {
            menclose.push_str("bottom ");
        }

        for col in cols.iter().take(i_end).skip(i_start) {
            if let AlignSpec::Align {
                align: col_align, ..
            } = col
            {
                let mapped_align = match col_align.trim() {
                    "c" => "center ",
                    "l" => "left ",
                    "r" => "right ",
                    other => {
                        let mut tmp = other.to_owned();
                        tmp.push(' ');
                        align.push_str(&tmp);
                        if prev_type_was_align {
                            // columnLines += "none ";
                        }
                        prev_type_was_align = true;
                        continue;
                    }
                };
                align.push_str(mapped_align);

                if prev_type_was_align {
                    // columnLines += "none ";
                }
                prev_type_was_align = true;
            } else if let AlignSpec::Separator { separator } = col {
                // MathML accepts only single lines between cells.
                // So we read only the first of consecutive separators.
                if prev_type_was_align {
                    let line_type = if separator == "|" {
                        "solid "
                    } else {
                        "dashed "
                    };
                    column_lines += line_type;
                    prev_type_was_align = false;
                }
            }
        }

        // cdlongequal: path not found
        // !TODO
        // 'c' differs from 'center'
        table
            .attributes
            .insert("columnalign".to_owned(), align.trim().to_owned());

        // if /[sd]/.test(columnLines) {
        //     table.setAttribute("columnlines", columnLines.trim());
        // }
    }

    // Set column spacing.
    if array_node.col_separation_type == Some(ColSeparationType::Align) {
        if let Some(cols) = &array_node.cols {
            let mut spacing = String::new();
            for i in 1..cols.len() {
                spacing.push_str(if i % 2 == 1 { "0em " } else { "1em " });
            }
            table
                .attributes
                .insert("columnspacing".to_owned(), spacing.trim().to_owned());
        }
    } else if array_node.col_separation_type == Some(ColSeparationType::Alignat)
        || array_node.col_separation_type == Some(ColSeparationType::Gather)
    {
        table
            .attributes
            .insert("columnspacing".to_owned(), "0em".to_owned());
    } else if array_node.col_separation_type == Some(ColSeparationType::Small) {
        table
            .attributes
            .insert("columnspacing".to_owned(), "0.2778em".to_owned());
    } else if array_node.col_separation_type == Some(ColSeparationType::CD) {
        table
            .attributes
            .insert("columnspacing".to_owned(), "0.5em".to_owned());
    } else {
        table
            .attributes
            .insert("columnspacing".to_owned(), "1em".to_owned());
    }

    // Address \hline and \hdashline
    let mut row_lines = String::new();
    let hlines = &array_node.h_lines_before_row;

    menclose.push_str(if hlines.first().is_some_and(|h| !h.is_empty()) {
        "left "
    } else {
        ""
    });
    menclose.push_str(if hlines.last().is_some_and(|h| !h.is_empty()) {
        "right "
    } else {
        ""
    });

    for hline in hlines.iter().take(hlines.len().saturating_sub(1)).skip(1) {
        row_lines.push_str(if hline.is_empty() {
            "none "
        } else {
            // MathML accepts only a single line between rows. Read one element.
            if hline[0] { "dashed " } else { "solid " }
        });
    }

    if row_lines.contains('s') || row_lines.contains('d') {
        table
            .attributes
            .insert("rowlines".to_owned(), row_lines.trim().to_owned());
    }

    let mut result = MathDomNode::Math(table);

    if !menclose.trim().is_empty() {
        result = MathNode::builder()
            .node_type(MathNodeType::Menclose)
            .children(vec![result])
            .attributes(
                vec![("notation".to_owned(), menclose.trim().to_owned())]
                    .into_iter()
                    .collect(),
            )
            .build()
            .into();
    }

    if array_node.arraystretch < 1.0 {
        // A small array. Wrap in scriptstyle so row gap is not too large.
        result = MathNode::builder()
            .node_type(MathNodeType::Mstyle)
            .children(vec![result])
            .attributes(
                vec![("scriptlevel".to_owned(), "1".to_owned())]
                    .into_iter()
                    .collect(),
            )
            .build()
            .into();
    }

    Ok(result)
}

/// Handler for aligned environments (align, align*, aligned, alignat, alignat*,
/// alignedat, split)
const ALIGNED_HANDLER: EnvHandler = |context, args, _opt_args| {
    if !context.env_name.contains("ed") {
        validate_ams_environment_context(&context)?;
    }

    let cols = Vec::new();
    let separation_type = if context.env_name.contains("at") {
        ColSeparationType::Alignat
    } else {
        ColSeparationType::Align
    };
    let is_split = context.env_name == "split";

    let mut res = parse_array(
        context.parser,
        ArrayParseConfig {
            cols: Some(cols),
            add_jot: Some(true),
            auto_tag: if is_split {
                None
            } else {
                get_auto_tag(&context.env_name)
            },
            empty_single_row: Some(true),
            col_separation_type: Some(separation_type),
            max_num_cols: is_split.then_some(2),
            leqno: Some(context.parser.settings.leqno),
            ..Default::default()
        },
        DISPLAY,
    )?;

    // Determining number of columns.
    // 1. If the first argument is given, we use it as a number of columns, and
    //    makes sure that each row doesn't exceed that number.
    // 2. Otherwise, just count number of columns = maximum number of cells in each
    //    row ("aligned" mode -- isAligned will be true).
    //
    // At the same time, prepend empty group {} at beginning of every second
    // cell in each row (starting with second cell) so that operators become
    // binary.  This behavior is implemented in amsmath's \start@aligned.
    let mut num_maths = 0;
    let mut num_cols = 0;
    let empty_group = ParseNode::OrdGroup(ParseNodeOrdGroup {
        mode: context.mode,
        loc: None,
        body: vec![],
        semisimple: None,
    });

    if !args.is_empty()
        && let Some(ParseNode::OrdGroup(ord)) = args.first()
    {
        let mut num_str = String::new();
        for node in &ord.body {
            if let ParseNode::TextOrd(text) = node {
                num_str.push_str(text.text.as_str());
            }
        }
        num_maths = num_str
            .parse::<usize>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidNumberOfColumns))?;
        num_cols = num_maths * 2;
    }

    let is_aligned = num_cols == 0;

    for row in &mut res.body {
        for i in (1..row.len()).step_by(2) {
            // Modify ordgroup node within styling node
            if let ParseNode::Styling(styling) = &mut row[i]
                && let Some(ParseNode::OrdGroup(ordgroup)) = styling.body.first_mut()
            {
                ordgroup.body.insert(0, empty_group.clone());
            }
        }
        if !is_aligned {
            // Case 1
            let cur_maths = row.len() / 2;
            if num_maths < cur_maths {
                return Err(ParseError::new(ParseErrorKind::TooManyMathInRow {
                    expected: num_maths,
                    actual: cur_maths,
                }));
            }
        } else if num_cols < row.len() {
            // Case 2
            num_cols = row.len();
        }
    }

    // Adjusting alignment.
    // In aligned mode, we add one \qquad between columns;
    // otherwise we add nothing.
    let mut new_cols = Vec::new();
    for i in 0..num_cols {
        let mut align = "r";
        let mut pregap = 0.0;
        if i % 2 == 1 {
            align = "l";
        } else if i > 0 && is_aligned {
            // "aligned" mode.
            pregap = 1.0; // add one \quad
        }
        new_cols.push(AlignSpec::Align {
            align: align.to_owned(),
            pregap: Some(pregap),
            postgap: Some(0.0),
        });
    }
    res.cols = Some(new_cols);
    res.col_separation_type = Some(if is_aligned {
        ColSeparationType::Align
    } else {
        ColSeparationType::Alignat
    });

    Ok(ParseNode::Array(res))
};

/// Registers array environment in the KaTeX context
pub fn define_array(ctx: &mut KatexContext) {
    // Arrays are part of LaTeX, defined in lttab.dtx so its documentation
    // is part of the source2e.pdf file of LaTeX2e source documentation.
    // {darray} is an {array} environment where cells are set in \displaystyle,
    // as defined in nccmath.sty.
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec!["array".to_owned(), "darray".to_owned()],
        props: EnvProps {
            num_args: Some(1),
            ..Default::default()
        },
        handler: |context, args, _opt_args| {
            // Since no types are specified above, the two possibilities are
            // - The argument is wrapped in {} or [], in which case Parser's parseGroup()
            //   returns an "ordgroup" wrapping some symbol node.
            // - The argument is a bare symbol node.
            let sym_node = check_symbol_node_type(args.first());
            let colalign: Vec<AnyParseNode> = if sym_node.is_some() {
                vec![args[0].clone()]
            } else if let Some(ParseNode::OrdGroup(ord)) = args.first() {
                ord.body.clone()
            } else {
                return Err(ParseError::new(
                    ParseErrorKind::ExpectedOrdGroupOrSymbolNode,
                ));
            };

            let cols = colalign
                .into_iter()
                .map(|nde| {
                    let Some(ca) = nde.text() else {
                        return Err(ParseError::new(
                            ParseErrorKind::ExpectedColumnAlignmentCharacter,
                        ));
                    };

                    if "lcr|".contains(ca) {
                        if ca == "|" {
                            Ok(AlignSpec::Separator {
                                separator: "|".to_owned(),
                            })
                        } else {
                            Ok(AlignSpec::Align {
                                align: ca.to_owned(),
                                pregap: None,
                                postgap: None,
                            })
                        }
                    } else if ca == ":" {
                        Ok(AlignSpec::Separator {
                            separator: ":".to_owned(),
                        })
                    } else {
                        Err(ParseError::new(ParseErrorKind::UnknownColumnAlignment {
                            alignment: ca.to_owned(),
                        }))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            let res = parse_array(
                context.parser,
                ArrayParseConfig {
                    max_num_cols: Some(cols.len()),
                    cols: Some(cols),
                    hskip_before_and_after: Some(true), // \@preamble in lttab.dtx
                    ..Default::default()
                },
                d_cell_style(&context.env_name),
            )?;

            Ok(ParseNode::Array(res))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // The matrix environments of amsmath builds on the array environment
    // of LaTeX, which is discussed above.
    // The mathtools package adds starred versions of the same environments.
    // These have an optional argument to choose left|center|right justification.
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec![
            "matrix".to_owned(),
            "pmatrix".to_owned(),
            "bmatrix".to_owned(),
            "Bmatrix".to_owned(),
            "vmatrix".to_owned(),
            "Vmatrix".to_owned(),
            "matrix*".to_owned(),
            "pmatrix*".to_owned(),
            "bmatrix*".to_owned(),
            "Bmatrix*".to_owned(),
            "vmatrix*".to_owned(),
            "Vmatrix*".to_owned(),
        ],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: |context, _args, _opt_args| {
            let delimiters = match context.env_name.as_str().trim_end_matches('*') {
                "matrix" => None,
                "pmatrix" => Some(("(".to_owned(), ")".to_owned())),
                "bmatrix" => Some(("[".to_owned(), "]".to_owned())),
                "Bmatrix" => Some(("\\{".to_owned(), "\\}".to_owned())),
                "vmatrix" => Some(("|".to_owned(), "|".to_owned())),
                "Vmatrix" => Some(("\\Vert".to_owned(), "\\Vert".to_owned())),
                _ => unreachable!(),
            };

            // \hskip -\arraycolsep in amsmath
            let mut col_align = "c".to_owned();
            let payload = ArrayParseConfig {
                hskip_before_and_after: Some(false),
                cols: Some(vec![AlignSpec::Align {
                    align: col_align.clone(),
                    pregap: None,
                    postgap: None,
                }]),
                ..Default::default()
            };

            let mut payload = payload;
            if context.env_name.ends_with('*') {
                // It's one of the mathtools starred functions.
                // Parse the optional alignment argument.
                context.parser.gullet.consume_spaces()?;
                if context.parser.fetch()?.text == "[" {
                    context.parser.consume();
                    context.parser.gullet.consume_spaces()?;
                    context.parser.fetch()?.text.clone_into(&mut col_align);
                    if !["l", "c", "r"].contains(&col_align.as_str()) {
                        return Err(ParseError::new(ParseErrorKind::ExpectedAlignmentSpecifier));
                    }
                    context.parser.consume();
                    context.parser.gullet.consume_spaces()?;
                    let next = context.parser.fetch()?;
                    if next.text != "]" {
                        return Err(ParseError::new(ParseErrorKind::ExpectedClosingBracket {
                            found: next.text.to_owned_string(),
                        }));
                    }
                    context.parser.consume();
                    context.parser.consume();
                    payload.cols = Some(vec![AlignSpec::Align {
                        align: col_align.clone(),
                        pregap: None,
                        postgap: None,
                    }]);
                }
            }

            let mut res = parse_array(context.parser, payload, d_cell_style(&context.env_name))?;

            // Populate cols with the correct number of column alignment specs.
            let num_cols = res.body.iter().map(Vec::len).max().unwrap_or(0);
            res.cols = Some(
                repeat_n(
                    AlignSpec::Align {
                        align: col_align.clone(),
                        pregap: None,
                        postgap: None,
                    },
                    num_cols,
                )
                .collect(),
            );

            if let Some((left, right)) = delimiters {
                Ok(ParseNode::LeftRight(ParseNodeLeftRight {
                    mode: context.mode,
                    loc: None,
                    body: vec![ParseNode::Array(res)],
                    left,
                    right,
                    right_color: None,
                }))
            } else {
                Ok(ParseNode::Array(res))
            }
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // smallmatrix environment
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec!["smallmatrix".to_owned()],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: |context, _args, _opt_args| {
            let payload = ArrayParseConfig {
                arraystretch: Some(0.5),
                ..Default::default()
            };
            let mut res = parse_array(context.parser, payload, SCRIPT)?;
            res.col_separation_type = Some(ColSeparationType::Small);
            Ok(ParseNode::Array(res))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // subarray environment
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec!["subarray".to_owned()],
        props: EnvProps {
            num_args: Some(1),
            ..Default::default()
        },
        handler: |context, args, _opt_args| {
            // Parsing of {subarray} is similar to {array}
            let sym_node = check_symbol_node_type(args.first());
            let colalign: Vec<AnyParseNode> = if sym_node.is_some() {
                vec![args[0].clone()]
            } else if let Some(ParseNode::OrdGroup(ord)) = args.first() {
                ord.body.clone()
            } else {
                return Err(ParseError::new(
                    ParseErrorKind::ExpectedOrdGroupOrSymbolNode,
                ));
            };

            let cols = colalign
                .into_iter()
                .map(|nde| {
                    let Some(ca) = nde.text() else {
                        return Err(ParseError::new(
                            ParseErrorKind::ExpectedColumnAlignmentCharacter,
                        ));
                    };
                    // {subarray} only recognizes "l" & "c"
                    if "lc".contains(ca) {
                        Ok(AlignSpec::Align {
                            align: ca.to_owned(),
                            pregap: None,
                            postgap: None,
                        })
                    } else {
                        Err(ParseError::new(ParseErrorKind::UnknownColumnAlignment {
                            alignment: ca.to_owned(),
                        }))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            if cols.len() > 1 {
                return Err(ParseError::new(ParseErrorKind::SubarrayTooManyColumns {
                    subarray: "{subarray}",
                }));
            }

            let res = parse_array(
                context.parser,
                ArrayParseConfig {
                    cols: Some(cols),
                    hskip_before_and_after: Some(false),
                    arraystretch: Some(0.5),
                    ..Default::default()
                },
                SCRIPT,
            )?;

            if !res.body.is_empty() && res.body[0].len() > 1 {
                return Err(ParseError::new(ParseErrorKind::SubarrayTooManyColumns {
                    subarray: "{subarray}",
                }));
            }

            Ok(ParseNode::Array(res))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // cases environment
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec![
            "cases".to_owned(),
            "dcases".to_owned(),
            "rcases".to_owned(),
            "drcases".to_owned(),
        ],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: |context, _args, _opt_args| {
            let res = parse_array(
                context.parser,
                ArrayParseConfig {
                    arraystretch: Some(1.2),
                    cols: Some(vec![
                        AlignSpec::Align {
                            align: "l".to_owned(),
                            pregap: Some(0.0),
                            postgap: Some(1.0), // 1em quad
                        },
                        AlignSpec::Align {
                            align: "l".to_owned(),
                            pregap: Some(0.0),
                            postgap: Some(0.0),
                        },
                    ]),
                    ..Default::default()
                },
                d_cell_style(&context.env_name),
            )?;

            let (left, right) = if context.env_name.contains('r') {
                (".".to_owned(), "\\}".to_owned())
            } else {
                ("\\{".to_owned(), ".".to_owned())
            };

            Ok(ParseNode::LeftRight(ParseNodeLeftRight {
                mode: context.mode,
                loc: None,
                body: vec![ParseNode::Array(res)],
                left,
                right,
                right_color: None,
            }))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // gathered environment
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec![
            "gathered".to_owned(),
            "gather".to_owned(),
            "gather*".to_owned(),
        ],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: |context, _args, _opt_args| {
            if ["gather", "gather*"].contains(&context.env_name.as_str()) {
                validate_ams_environment_context(&context)?;
            }

            let res = parse_array(
                context.parser,
                ArrayParseConfig {
                    cols: Some(vec![AlignSpec::Align {
                        align: "c".to_owned(),
                        pregap: None,
                        postgap: None,
                    }]),
                    add_jot: Some(true),
                    col_separation_type: Some(ColSeparationType::Gather),
                    auto_tag: get_auto_tag(&context.env_name),
                    empty_single_row: Some(true),
                    leqno: Some(context.parser.settings.leqno),
                    ..Default::default()
                },
                DISPLAY,
            )?;

            Ok(ParseNode::Array(res))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // align, align*, aligned, split environments
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec![
            "align".to_owned(),
            "align*".to_owned(),
            "aligned".to_owned(),
            "split".to_owned(),
        ],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: ALIGNED_HANDLER,
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // alignat, alignat*, alignedat environments
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec![
            "alignat".to_owned(),
            "alignat*".to_owned(),
            "alignedat".to_owned(),
        ],
        props: EnvProps {
            num_args: Some(1),
            ..Default::default()
        },
        handler: ALIGNED_HANDLER,
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // equation environment
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec!["equation".to_owned(), "equation*".to_owned()],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: |context, _args, _opt_args| {
            validate_ams_environment_context(&context)?;

            let res = parse_array(
                context.parser,
                ArrayParseConfig {
                    auto_tag: get_auto_tag(&context.env_name),
                    empty_single_row: Some(true),
                    single_row: true,
                    max_num_cols: Some(1),
                    leqno: Some(context.parser.settings.leqno),
                    ..Default::default()
                },
                DISPLAY,
            )?;

            Ok(ParseNode::Array(res))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // CD environment
    ctx.define_environment(EnvDefSpec {
        node_type: NodeType::Array,
        names: vec!["CD".to_owned()],
        props: EnvProps {
            num_args: Some(0),
            ..Default::default()
        },
        handler: |context, _args, _opt_args| {
            validate_ams_environment_context(&context)?;
            let result = parse_cd(context.parser)?;
            Ok(ParseNode::from(result))
        },
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \nonumber and \notag macros should not be defined here

    // Catch \hline outside array environment
    ctx.define_function(FunctionDefSpec {
        node_type: None,
        names: &["\\hline", "\\hdashline"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            allowed_in_math: true,
            ..Default::default()
        },
        handler: Some(
            |context,
             _args: Vec<ParseNode>,
             _opt_args: Vec<Option<ParseNode>>|
             -> Result<ParseNode, ParseError> {
                Err(ParseError::new(ParseErrorKind::FunctionOnlyInArray {
                    func: context.func_name.to_owned(),
                }))
            },
        ),
        html_builder: None,
        mathml_builder: None,
    });
}
