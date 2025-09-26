//! Build common utilities for KaTeX DOM construction
//!
//! This module provides general functions for constructing DOM tree nodes in
//! KaTeX's math rendering process. It includes utilities for creating symbols
//! and other DOM elements with proper styling and metrics.

use crate::ParseError;
use crate::context::KatexContext;
use crate::dom_tree::{
    Anchor, DomSpan, HtmlDomFragment, HtmlDomNode, Span, SvgNode, SymbolNode, create_class,
};
use crate::font_metrics::get_character_metrics;
use crate::font_metrics_data::CharacterMetrics;
use crate::namespace::KeyMap;
use crate::options::{FontShape, FontWeight, Options};
use crate::parser::parse_node::AnyParseNode;
use crate::spacing_data::Measurement;
use crate::symbols::{Font, Mode, is_ligature};
use crate::tree::DocumentFragment;
use crate::types::{CssProperty, CssStyle};
use crate::units::make_em;
use crate::wide_character::get_wide_character_font;
use bon::bon;
use phf::phf_map;

/// Font mapping for TeX font commands to font names and variants
pub const FONT_MAP: phf::Map<&str, FontMapEntry> = phf_map! {
    "mathbf" => FontMapEntry {
        variant: "bold",
        font_name: "Main-Bold",
    },
    "mathrm" => FontMapEntry {
        variant: "normal",
        font_name: "Main-Regular",
    },
    "textit" => FontMapEntry {
        variant: "italic",
        font_name: "Main-Italic",
    },
    "mathit" => FontMapEntry {
        variant: "italic",
        font_name: "Main-Italic",
    },
    "mathnormal" => FontMapEntry {
        variant: "italic",
        font_name: "Math-Italic",
    },
    "mathbb" => FontMapEntry {
        variant: "double-struck",
        font_name: "AMS-Regular",
    },
    "mathcal" => FontMapEntry {
        variant: "script",
        font_name: "Caligraphic-Regular",
    },
    "mathscr" => FontMapEntry {
        variant: "script",
        font_name: "Script-Regular",
    },
    "mathfrak" => FontMapEntry {
        variant: "fraktur",
        font_name: "Fraktur-Regular",
    },
    "mathsf" => FontMapEntry {
        variant: "sans-serif",
        font_name: "SansSerif-Regular",
    },
    "mathsfit" => FontMapEntry {
        variant: "sans-serif-italic",
        font_name: "SansSerif-Italic",
    },
    "mathtt" => FontMapEntry {
        variant: "monospace",
        font_name: "Typewriter-Regular",
    },
    "boldsymbol" => FontMapEntry {
        variant: "bold-italic",
        font_name: "Math-BoldItalic",
    },
};

/// Font map entry structure
#[derive(Debug, Clone)]
pub struct FontMapEntry {
    /// MathML mathvariant attribute value
    pub variant: &'static str,
    /// Font name for font metrics lookup
    pub font_name: &'static str,
}

/// Result of symbol lookup
#[derive(Debug, Clone)]
pub struct SymbolLookup {
    /// The symbol value (possibly replaced)
    pub value: char,
    /// Character metrics if available
    pub metrics: Option<CharacterMetrics>,
}

/// Element for vertical list
#[derive(Debug, bon::Builder)]
pub struct VListElem {
    /// The HTML DOM node element
    pub elem: HtmlDomNode,
    /// Optional shift amount
    pub shift: Option<f64>,
    /// Optional left margin
    pub margin_left: Option<String>,
    /// Optional right margin
    pub margin_right: Option<String>,
    /// Optional wrapper classes
    pub wrapper_classes: Option<Vec<String>>,
    /// Optional wrapper style
    pub wrapper_style: Option<CssStyle>,
}

/// Kern element for vertical list
#[derive(Debug, Clone)]
pub struct VListKern {
    /// Size of the kern
    pub size: f64,
}

impl From<f64> for VListKern {
    fn from(size: f64) -> Self {
        Self { size }
    }
}

/// Child element in vertical list (either elem or kern)
#[derive(Debug)]
pub enum VListChild {
    /// Element child
    Elem(Box<VListElem>),
    /// Kern child
    Kern(VListKern),
}

impl From<VListElem> for VListChild {
    fn from(elem: VListElem) -> Self {
        Self::Elem(Box::new(elem))
    }
}

/// Element with shift for individual shift positioning
#[derive(Debug)]
pub struct VListElemAndShift {
    /// The HTML DOM node element
    pub elem: HtmlDomNode,
    /// Shift amount
    pub shift: f64,
    /// Optional left margin
    pub margin_left: Option<String>,
    /// Optional right margin
    pub margin_right: Option<String>,
    /// Optional wrapper classes
    pub wrapper_classes: Option<Vec<String>>,
    /// Optional wrapper style
    pub wrapper_style: Option<CssStyle>,
}

#[bon]
impl VListElemAndShift {
    /// Creates a new VListElemAndShift
    #[builder]
    pub const fn new(
        /// Element for vertical list
        elem: HtmlDomNode,
        /// Shift amount
        shift: f64,
        /// Optional left margin
        margin_left: Option<String>,
        /// Optional right margin
        margin_right: Option<String>,
        /// Optional wrapper classes
        wrapper_classes: Option<Vec<String>>,
        /// Optional wrapper style
        wrapper_style: Option<CssStyle>,
    ) -> Self {
        Self {
            elem,
            shift,
            margin_left,
            margin_right,
            wrapper_classes,
            wrapper_style,
        }
    }
}

/// Parameters for make_v_list function
#[derive(Debug)]
pub enum VListParam {
    /// Individual shift positioning
    IndividualShift {
        /// The children elements with individual shifts
        children: Vec<VListElemAndShift>,
    },
    /// Top positioning
    Top {
        /// The position data for top alignment
        position_data: f64,
        /// The child elements
        children: Vec<VListChild>,
    },
    /// Bottom positioning
    Bottom {
        /// The position data for bottom alignment
        position_data: f64,
        /// The child elements
        children: Vec<VListChild>,
    },
    /// Shift positioning
    Shift {
        /// The position data for shift alignment
        position_data: f64,
        /// The child elements
        children: Vec<VListChild>,
    },
    /// First baseline positioning
    FirstBaseline {
        /// The child elements
        children: Vec<VListChild>,
    },
}

/// Result of get_v_list_children_and_depth function
#[derive(Debug)]
pub struct VListChildrenAndDepth {
    /// The processed children
    pub children: Vec<VListChild>,
    /// The calculated depth
    pub depth: f64,
}

/// Compute height, depth, and max_font_size of an element based on its children
fn size_element_from_children_dom(node: &mut DomSpan) {
    let mut height = 0.0f64;
    let mut depth = 0.0f64;
    let mut max_font_size = 0.0f64;

    for child in &node.children {
        height = height.max(child.height());
        depth = depth.max(child.depth());
        max_font_size = max_font_size.max(child.max_font_size());
    }

    // Update the node's values
    node.height = height;
    node.depth = depth;
    node.max_font_size = max_font_size;
}

/// Create a span with given classes and options
#[must_use]
pub fn make_span(
    classes: Vec<String>,
    children: Vec<HtmlDomNode>,
    options: Option<&Options>,
    style: Option<CssStyle>,
) -> DomSpan {
    let mut node = Span::builder()
        .children(children)
        .classes(classes)
        .maybe_style(style)
        .build(options);

    // Compute height, depth, and max_font_size from children
    size_element_from_children_dom(&mut node);

    node
}

/// Makes a vertical list by stacking elements and kerns on top of each other
/// with complete KaTeX logic including pstrut creation, table layout, and
/// styling
pub fn make_v_list(params: VListParam, _options: &Options) -> Result<DomSpan, ParseError> {
    let VListChildrenAndDepth { children, depth } = get_v_list_children_and_depth(params)?;

    // Create pstrut: a phantom strut taller than any list item
    // This ensures precise baseline positioning without font ascent/line-height
    // interference
    let mut pstrut_size = 0.0f64;
    for child in &children {
        if let VListChild::Elem(elem) = child {
            pstrut_size = pstrut_size
                .max(elem.elem.max_font_size())
                .max(elem.elem.height());
        }
    }
    pstrut_size += 2.0; // Add buffer for safety
    let mut pstrut = make_span(vec!["pstrut".to_owned()], vec![], None, None);
    pstrut
        .style
        .insert(CssProperty::Height, make_em(pstrut_size));

    // Process children and calculate positioning
    let mut real_children: Vec<HtmlDomNode> = Vec::new();
    let mut min_pos = depth;
    let mut max_pos = depth;
    let mut curr_pos = depth;

    for child in children {
        match child {
            VListChild::Kern(kern) => {
                curr_pos += kern.size;
            }
            VListChild::Elem(child) => {
                let elem = child.elem;
                let classes = child.wrapper_classes.unwrap_or_default();
                let style = child.wrapper_style.unwrap_or_default();

                let elem_height = elem.height();
                let elem_depth = elem.depth();

                // Create wrapper span with pstrut and element
                let mut child_wrap = make_span(
                    classes,
                    vec![pstrut.clone().into(), elem],
                    None,
                    Some(style),
                );

                // Set top positioning for vertical alignment
                child_wrap.style.insert(
                    CssProperty::Top,
                    make_em(-pstrut_size - curr_pos - elem_depth),
                );

                // Apply margins if specified
                if let Some(margin_left) = &child.margin_left {
                    child_wrap
                        .style
                        .insert(CssProperty::MarginLeft, margin_left.clone());
                }
                if let Some(margin_right) = &child.margin_right {
                    child_wrap
                        .style
                        .insert(CssProperty::MarginRight, margin_right.clone());
                }

                real_children.push(child_wrap.into());
                curr_pos += elem_height + elem_depth;
            }
        }
        min_pos = min_pos.min(curr_pos);
        max_pos = max_pos.max(curr_pos);
    }

    // Create the main vlist cell with vertical-align: bottom
    let mut vlist = make_span(vec!["vlist".to_owned()], real_children, None, None);
    vlist.style.insert(CssProperty::Height, make_em(max_pos));

    // Handle depth with multiple rows when needed
    let rows: Vec<HtmlDomNode> = if min_pos < 0.0 {
        // Create depth row with proper height
        let empty_span = make_span(vec![], vec![], None, None);
        let mut depth_strut = make_span(
            vec!["vlist".to_owned()],
            vec![empty_span.into()],
            None,
            None,
        );
        depth_strut
            .style
            .insert(CssProperty::Height, make_em(-min_pos));

        // Create top row with zero-width space for Safari compatibility
        let top_strut = make_span(
            vec!["vlist-s".to_owned()],
            vec![HtmlDomNode::Symbol(
                SymbolNode::builder().text("\u{200b}").build(),
            )],
            None,
            None,
        );

        vec![
            make_span(
                vec!["vlist-r".to_owned()],
                vec![vlist.into(), top_strut.into()],
                None,
                None,
            )
            .into(),
            make_span(
                vec!["vlist-r".to_owned()],
                vec![depth_strut.into()],
                None,
                None,
            )
            .into(),
        ]
    } else {
        vec![make_span(vec!["vlist-r".to_owned()], vec![vlist.into()], None, None).into()]
    };

    let mut vtable_classes = vec!["vlist-t".to_owned()];

    // Add vlist-t2 class if depth row is present
    if rows.len() == 2 {
        vtable_classes.push("vlist-t2".to_owned());
    }

    // Create the table structure with vlist-t class
    let mut vtable = make_span(vtable_classes, rows, None, None);

    // Set final height and depth on the table
    vtable.height = max_pos;
    vtable.depth = -min_pos;

    Ok(vtable)
}

/// Computes the updated children list and the overall depth for vertical lists.
///
/// This helper function for make_v_list makes it easier to enforce type safety
/// by allowing early exits (returns) in the logic.
pub fn get_v_list_children_and_depth(
    params: VListParam,
) -> Result<VListChildrenAndDepth, ParseError> {
    match params {
        VListParam::IndividualShift {
            children: old_children,
        } => {
            let mut children: Vec<VListChild> = Vec::new();

            // Add the first child
            let first_child = &old_children[0];
            let first_elem = VListElem {
                elem: first_child.elem.clone(),
                shift: Some(first_child.shift),
                margin_left: first_child.margin_left.clone(),
                margin_right: first_child.margin_right.clone(),
                wrapper_classes: first_child.wrapper_classes.clone(),
                wrapper_style: first_child.wrapper_style.clone(),
            };
            children.push(first_elem.into());

            // Calculate initial depth
            let depth = -old_children[0].shift - old_children[0].elem.depth();
            let mut curr_pos = depth;

            // Add in kerns to the list of children to get each element to be
            // shifted to the correct specified shift
            for i in 1..old_children.len() {
                let child = &old_children[i];
                let diff = -child.shift - curr_pos - child.elem.depth();
                let size =
                    diff - (old_children[i - 1].elem.height() + old_children[i - 1].elem.depth());

                curr_pos += diff;

                children.push(VListChild::Kern(VListKern { size }));

                let elem = VListElem {
                    elem: child.elem.clone(),
                    shift: Some(child.shift),
                    margin_left: child.margin_left.clone(),
                    margin_right: child.margin_right.clone(),
                    wrapper_classes: child.wrapper_classes.clone(),
                    wrapper_style: child.wrapper_style.clone(),
                };
                children.push(elem.into());
            }

            Ok(VListChildrenAndDepth { children, depth })
        }
        VListParam::Top {
            position_data,
            children: vlist_children,
        } => {
            // We always start at the bottom, so calculate the bottom by adding up
            // all the sizes
            let mut bottom = position_data;
            for child in &vlist_children {
                bottom -= match child {
                    VListChild::Kern(kern) => kern.size,
                    VListChild::Elem(elem) => elem.elem.height() + elem.elem.depth(),
                };
            }
            let depth = bottom;
            Ok(VListChildrenAndDepth {
                children: vlist_children,
                depth,
            })
        }
        VListParam::Bottom {
            position_data,
            children: vlist_children,
        } => {
            let depth = -position_data;
            Ok(VListChildrenAndDepth {
                children: vlist_children,
                depth,
            })
        }
        VListParam::Shift {
            position_data,
            children: vlist_children,
        } => {
            // Find the first Elem child to calculate depth
            let first_elem = vlist_children.iter().find_map(|child| {
                if let VListChild::Elem(elem) = child {
                    Some(elem)
                } else {
                    None
                }
            });

            let depth = first_elem
                .map_or_else(|| -position_data, |elem| -elem.elem.depth() - position_data);

            Ok(VListChildrenAndDepth {
                children: vlist_children,
                depth,
            })
        }
        VListParam::FirstBaseline {
            children: vlist_children,
        } => {
            // Find the first Elem child to calculate depth
            let first_elem = vlist_children.iter().find_map(|child| {
                if let VListChild::Elem(elem) = child {
                    Some(elem)
                } else {
                    None
                }
            });

            let depth = first_elem.map_or(0.0, |elem| -elem.elem.depth());

            Ok(VListChildrenAndDepth {
                children: vlist_children,
                depth,
            })
        }
    }
}

/// Looks up the given symbol in font metrics, after applying any symbol
/// replacements
pub fn lookup_symbol(
    ctx: &KatexContext,
    value: &str,
    font_name: &str,
    mode: Mode,
) -> Result<Option<SymbolLookup>, ParseError> {
    let query = if let Some(char_info) = ctx.symbols.get(mode, value)
        && let Some(replaced) = char_info.replace
    {
        replaced
    } else {
        value
            .chars()
            .next()
            .ok_or_else(|| ParseError::new("Empty string passed to lookup_symbol"))?
    };

    let metrics = get_character_metrics(ctx, query, font_name, mode)?;
    Ok(Some(SymbolLookup {
        value: query,
        metrics: metrics.copied(),
    }))
}

/// Makes a symbol node after translation via the list of symbols
pub fn make_symbol(
    ctx: &KatexContext,
    value: &str,
    font_name: &str,
    mode: Mode,
    options: Option<&Options>,
    classes: Option<&[String]>,
) -> Result<SymbolNode, ParseError> {
    let (metrics, value) = lookup_symbol(ctx, value, font_name, mode)?.map_or_else(
        || (None, value.to_owned()),
        |lookup| (lookup.metrics, lookup.value.to_string()),
    );

    let (height, depth, italic, skew, width) = metrics.map_or((0.0, 0.0, 0.0, 0.0, 0.0), |m| {
        let italic = if mode == Mode::Text || options.as_ref().is_some_and(|o| o.font == "mathit") {
            0.0
        } else {
            m.italic
        };

        (m.height, m.depth, italic, m.skew, m.width)
    });

    let mut classes_vec = classes.unwrap_or(&[]).to_vec();
    let mut style = CssStyle::default();

    if let Some(options) = options {
        if options.style.is_tight() {
            classes_vec.push("mtight".to_owned());
        }
        if let Some(color) = options.get_color() {
            style.insert(CssProperty::Color, color);
        }
    }

    let mut symbol = SymbolNode::builder()
        .text(&value)
        .height(height)
        .depth(depth)
        .italic(italic)
        .skew(skew)
        .width(width)
        .classes(classes_vec)
        .style(style)
        .build();

    if let Some(options) = options {
        symbol.max_font_size = options.size_multiplier;
    }
    Ok(symbol)
}

/// Makes a symbol in Main-Regular or AMS-Regular for operators
pub fn mathsym(
    ctx: &KatexContext,
    value: &str,
    mode: Mode,
    options: &Options,
    classes: Option<&[String]>,
) -> Result<SymbolNode, ParseError> {
    if options.font == "boldsymbol"
        && lookup_symbol(ctx, value, "Main-Bold", mode)?
            .is_some_and(|lookup| lookup.metrics.is_some())
    {
        let mut combined_classes = classes.unwrap_or(&[]).to_vec();
        combined_classes.push("mathbf".to_owned());
        make_symbol(
            ctx,
            value,
            "Main-Bold",
            mode,
            Some(options),
            Some(&combined_classes),
        )
    } else if value == "\\"
        || ctx
            .symbols
            .get(mode, value)
            .is_some_and(|info| matches!(info.font, Font::Main))
    {
        make_symbol(ctx, value, "Main-Regular", mode, Some(options), classes)
    } else {
        let mut combined_classes = classes.unwrap_or(&[]).to_vec();
        combined_classes.push("amsrm".to_owned());
        make_symbol(
            ctx,
            value,
            "AMS-Regular",
            mode,
            Some(options),
            Some(&combined_classes),
        )
    }
}

/// Takes font options, and returns the appropriate font lookup name
#[must_use]
pub fn retrieve_text_font_name(
    font_family: &str,
    font_weight: &FontWeight,
    font_shape: &FontShape,
) -> String {
    let base_font_name = match font_family {
        "amsrm" => "AMS",
        "textrm" => "Main",
        "textsf" => "SansSerif",
        "texttt" => "Typewriter",
        _ => font_family,
    };

    let font_styles_name = match (font_weight, font_shape) {
        (FontWeight::TextBf, FontShape::TextIt) => "BoldItalic",
        (FontWeight::TextBf, _) => "Bold",
        (_, FontShape::TextIt) => "Italic",
        _ => "Regular",
    };

    format!("{base_font_name}-{font_styles_name}")
}

/// Checks if two Symbol nodes can be combined
fn can_combine(prev: &HtmlDomNode, next: &HtmlDomNode) -> bool {
    // Extract Symbol data from both nodes
    let (prev_skew, prev_max_font_size, prev_classes, prev_style) = match prev {
        HtmlDomNode::Symbol(symbol_struct) => (
            symbol_struct.skew,
            symbol_struct.max_font_size,
            &symbol_struct.classes,
            &symbol_struct.style,
        ),
        _ => return false,
    };

    let (next_skew, next_max_font_size, next_classes, next_style) = match next {
        HtmlDomNode::Symbol(symbol_struct) => (
            symbol_struct.skew,
            symbol_struct.max_font_size,
            &symbol_struct.classes,
            &symbol_struct.style,
        ),
        _ => return false,
    };

    // Check if classes are identical
    if create_class(prev_classes) != create_class(next_classes) {
        return false;
    }

    // Check skew equality
    if prev_skew != next_skew {
        return false;
    }

    // Check max_font_size equality
    if prev_max_font_size != next_max_font_size {
        return false;
    }

    // If prev and next both are just "mbin"s or "mord"s we don't combine them
    // so that the proper spacing can be preserved.
    if prev_classes.len() == 1 {
        let cls = &prev_classes[0];
        if cls == "mbin" || cls == "mord" {
            return false;
        }
    }

    // Check that all styles in prev are equal in next
    next_style == prev_style
}

/// Combine consecutive Symbol nodes that meet the criteria
pub fn try_combine_chars(chars: &mut Vec<HtmlDomNode>) {
    if chars.is_empty() {
        return;
    }
    let mut i = 0;
    while i < chars.len() - 1 {
        let prev = &chars[i];
        let next = &chars[i + 1];

        if can_combine(prev, next) {
            // Extract data from next before modifying
            let (next_text, next_height, next_depth, next_italic) = match next {
                HtmlDomNode::Symbol(symbol_struct) => (
                    symbol_struct.text.clone(),
                    symbol_struct.height,
                    symbol_struct.depth,
                    symbol_struct.italic,
                ),
                _ => unreachable!(), // can_combine already checked this
            };

            // Modify prev
            if let HtmlDomNode::Symbol(symbol_struct) = &mut chars[i] {
                symbol_struct.text.push_str(&next_text);
                symbol_struct.height = symbol_struct.height.max(next_height);
                symbol_struct.depth = symbol_struct.depth.max(next_depth);
                symbol_struct.italic = next_italic; // Use the last character's italic correction
            }

            // Remove the next element
            chars.remove(i + 1);

            // Decrement index to check the same position again
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
}

impl KatexContext {
    /// Create a glue (spacing) node with proper unit conversion
    ///
    /// This method creates a spacing element that matches KaTeX's behavior,
    /// properly converting measurements to CSS ems using the context's
    /// calculate_size method.
    pub fn make_glue<T>(
        &self,
        measurement: &Measurement<T>,
        options: &Options,
    ) -> Result<DomSpan, ParseError>
    where
        T: AsRef<str>,
    {
        // Calculate the size using proper unit conversion
        let mut rule = make_span(vec!["mspace".to_owned()], vec![], Some(options), None);
        let size = self.calculate_size(measurement, options)?;
        rule.style.insert(CssProperty::MarginRight, make_em(size));
        Ok(rule)
    }
}

/// Makes either a mathord or textord in the correct font and color.
/// Corresponds to the JavaScript `makeOrd` function.
pub fn make_ord(
    ctx: &KatexContext,
    node: &AnyParseNode,
    options: &Options,
) -> Result<HtmlDomNode, ParseError> {
    // Extract mode and text from the node
    let (mode, text, ord_type) = match node {
        AnyParseNode::MathOrd(math_ord) => (math_ord.mode, &math_ord.text, Mode::Math),
        AnyParseNode::TextOrd(text_ord) => (text_ord.mode, &text_ord.text, Mode::Text),
        // Spacing use TextOrd type by default
        AnyParseNode::Spacing(spacing) => (spacing.mode, &spacing.text, Mode::Text),
        _ => {
            return Err(ParseError::new(
                "make_ord: expected MathOrd, TextOrd or Spacing node",
            ));
        }
    };

    let classes = vec!["mord".to_owned()];

    // Math mode or Old font (i.e. \rm)
    let is_font = mode == Mode::Math || (mode == Mode::Text && !options.font.is_empty());
    let font_or_family = if is_font {
        if options.font.is_empty() {
            None
        } else {
            Some(&options.font)
        }
    } else if options.font_family.is_empty() {
        None
    } else {
        Some(&options.font_family)
    };

    // Handle wide characters
    let mut utf16_iter = text.encode_utf16();
    if let Some(code_unit) = utf16_iter.next()
        && code_unit == 0xD835
        && let Ok((font_name, font_class)) = get_wide_character_font(text, mode)
        && !font_name.is_empty()
    {
        let mut combined_classes = classes;
        if !font_class.is_empty() {
            combined_classes.push(font_class.to_owned());
        }
        return Ok(make_symbol(
            ctx,
            text,
            font_name,
            mode,
            Some(options),
            Some(&combined_classes),
        )?
        .into());
    }

    // Handle font selection
    if let Some(font_or_family) = font_or_family {
        let (font_name, font_classes) = if font_or_family == "boldsymbol" {
            // Special handling for boldsymbol
            let font_data = bold_symbol(ctx, text, mode, options, &classes, ord_type)?;
            (font_data.font_name, vec![font_data.font_class])
        } else if is_font {
            // Font command like \mathbf
            let font_name: &str = FONT_MAP
                .get(font_or_family)
                .map_or(font_or_family, |entry| entry.font_name);
            (font_name.to_owned(), vec![font_or_family.clone()])
        } else {
            // Font family like \textrm
            let font_name =
                retrieve_text_font_name(font_or_family, &options.font_weight, &options.font_shape);
            (
                font_name,
                vec![
                    font_or_family.clone(),
                    options.font_weight.to_string(),
                    options.font_shape.as_str().to_owned(),
                ],
            )
        };

        if lookup_symbol(ctx, text, &font_name, mode)?
            .is_some_and(|lookup| lookup.metrics.is_some())
        {
            let mut combined_classes = classes;
            combined_classes.extend(font_classes);
            return Ok(make_symbol(
                ctx,
                text,
                &font_name,
                mode,
                Some(options),
                Some(&combined_classes),
            )?
            .into());
        }

        // Handle ligature decomposition for monospace fonts
        if font_name.starts_with("Typewriter") && is_ligature(text) {
            let mut base_classes = classes;
            base_classes.extend(font_classes);

            let mut parts = Vec::new();
            for ch in text.chars() {
                let char_str = ch.to_string();
                let symbol = make_symbol(
                    ctx,
                    &char_str,
                    &font_name,
                    mode,
                    Some(options),
                    Some(&base_classes),
                )?;
                parts.push(symbol.into());
            }
            return Ok(make_fragment(&parts).into());
        }
    }

    // Default font handling
    match ord_type {
        Mode::Math => {
            let mut combined_classes = classes;
            combined_classes.push("mathnormal".to_owned());
            Ok(make_symbol(
                ctx,
                text,
                "Math-Italic",
                mode,
                Some(options),
                Some(&combined_classes),
            )?
            .into())
        }
        Mode::Text => {
            // Check symbol table for font information
            let symbol_info = ctx.symbols.get(mode, text);
            if let Some(info) = symbol_info {
                match &info.font {
                    Font::Ams => {
                        let font_name = retrieve_text_font_name(
                            "amsrm",
                            &options.font_weight,
                            &options.font_shape,
                        );
                        let mut combined_classes = classes;
                        combined_classes.push("amsrm".to_owned());
                        combined_classes.push(options.font_weight.to_string());
                        combined_classes.push(options.font_shape.as_str().to_owned());
                        Ok(make_symbol(
                            ctx,
                            text,
                            &font_name,
                            mode,
                            Some(options),
                            Some(&combined_classes),
                        )?
                        .into())
                    }
                    Font::Main => {
                        let font_name = retrieve_text_font_name(
                            "textrm",
                            &options.font_weight,
                            &options.font_shape,
                        );
                        let mut combined_classes = classes;
                        combined_classes.push(options.font_weight.to_string());
                        combined_classes.push(options.font_shape.as_str().to_owned());
                        Ok(make_symbol(
                            ctx,
                            text,
                            &font_name,
                            mode,
                            Some(options),
                            Some(&combined_classes),
                        )?
                        .into())
                    }
                    Font::Custom(font) => {
                        let font_name = retrieve_text_font_name(
                            font,
                            &options.font_weight,
                            &options.font_shape,
                        );
                        let mut combined_classes = classes;
                        combined_classes.push(options.font_weight.to_string());
                        combined_classes.push(options.font_shape.as_str().to_owned());
                        Ok(make_symbol(
                            ctx,
                            text,
                            &font_name,
                            mode,
                            Some(options),
                            Some(&combined_classes),
                        )?
                        .into())
                    }
                }
            } else {
                // Default to main font
                let font_name =
                    retrieve_text_font_name("textrm", &options.font_weight, &options.font_shape);
                let mut combined_classes = classes;
                combined_classes.push(options.font_weight.to_string());
                combined_classes.push(options.font_shape.as_str().to_owned());
                Ok(make_symbol(
                    ctx,
                    text,
                    &font_name,
                    mode,
                    Some(options),
                    Some(&combined_classes),
                )?
                .into())
            }
        }
    }
}

/// Create an SVG span with given classes and SvgNode
pub fn make_svg_span(classes: Vec<String>, svg_node: Vec<SvgNode>, options: &Options) -> DomSpan {
    Span::builder()
        .children(svg_node.into_iter().map(HtmlDomNode::SvgNode).collect())
        .classes(classes)
        .build(Some(options))
}

/// W/H of vec
pub const VEC_SVG_DATA: (f64, f64) = (0.471, 0.714);
/// W/H of oiint1
pub const OIINT_SIZE1: (f64, f64) = (0.957, 0.499);
/// W/H of oiint2
pub const OIINT_SIZE2: (f64, f64) = (1.472, 0.659);
/// W/H of oiiint1
pub const OIIINT_SIZE1: (f64, f64) = (1.304, 0.499);
/// W/H of oiiint2
pub const OIIINT_SIZE2: (f64, f64) = (1.98, 0.659);

/// SVG data for static SVG elements
///
/// Equivalent to KaTeX buildCommon.js svgData
/// However, the values slightly differ because they are based on phf_map!
/// Values are `path` => (width, height)
const SVG_DATA: phf::Map<&'static str, (f64, f64)> = phf_map! {
    // values from the font glyph
    "vec" => VEC_SVG_DATA,
    // oval to overlay the integrand
    "oiintSize1" => OIINT_SIZE1,
    "oiintSize2" => OIINT_SIZE2,
    "oiiintSize1" => OIIINT_SIZE1,
    "oiiintSize2" => OIIINT_SIZE2,
};

/// Creates a static SVG span for elements like vec
pub fn static_svg(path_name: &str, options: &Options) -> Result<DomSpan, ParseError> {
    if let Some((width, height)) = SVG_DATA.get(path_name) {
        use crate::build_common::make_svg_span;
        use crate::dom_tree::{PathNode, SvgChildNode};

        let path = PathNode {
            path_name: path_name.to_owned(),
            alternate: None,
        };
        let svg_attributes = [
            ("width".to_owned(), make_em(*width)),
            ("height".to_owned(), make_em(*height)),
            // Override CSS rule `.katex svg { width: 100% }`
            ("style".to_owned(), format!("width:{}", make_em(*width))),
            (
                "viewBox".to_owned(),
                format!("0 0 {} {}", 1000.0 * width, 1000.0 * height),
            ),
            ("preserveAspectRatio".to_owned(), "xMinYMin".to_owned()),
        ]
        .iter()
        .cloned()
        .collect();

        let svg_node = SvgNode::builder()
            .children(vec![SvgChildNode::Path(path)])
            .attributes(svg_attributes)
            .build();
        let mut span = make_svg_span(vec!["overlay".to_owned()], vec![svg_node], options);
        span.height = *height;
        span.style.insert(CssProperty::Height, make_em(*height));
        span.style.insert(CssProperty::Width, make_em(*width));
        Ok(span)
    } else {
        // Fallback
        use crate::build_common::make_span;
        Ok(make_span(vec![], vec![], Some(options), None))
    }
}

/// Result of boldsymbol font selection
#[derive(Debug)]
struct FontData {
    font_name: String,
    font_class: String,
}

/// Determines which font to use for boldsymbol
fn bold_symbol(
    ctx: &KatexContext,
    text: &str,
    mode: Mode,
    _options: &Options,
    _classes: &[String],
    ord_type: Mode,
) -> Result<FontData, ParseError> {
    if ord_type != Mode::Text
        && lookup_symbol(ctx, text, "Math-BoldItalic", mode)?
            .is_some_and(|lookup| lookup.metrics.is_some())
    {
        Ok(FontData {
            font_name: "Math-BoldItalic".to_owned(),
            font_class: "boldsymbol".to_owned(),
        })
    } else {
        // Some glyphs do not exist in Math-BoldItalic so we need to use
        // Main-Bold instead.
        Ok(FontData {
            font_name: "Main-Bold".to_owned(),
            font_class: "mathbf".to_owned(),
        })
    }
}

/// Trait for elements that have size properties (height, depth, maxFontSize)
trait HasSizeProperties {
    fn set_height(&mut self, height: f64);
    fn set_depth(&mut self, depth: f64);
    fn set_max_font_size(&mut self, max_font_size: f64);
}

impl HasSizeProperties for DomSpan {
    fn set_height(&mut self, height: f64) {
        self.height = height;
    }

    fn set_depth(&mut self, depth: f64) {
        self.depth = depth;
    }

    fn set_max_font_size(&mut self, max_font_size: f64) {
        self.max_font_size = max_font_size;
    }
}

impl HasSizeProperties for Anchor {
    fn set_height(&mut self, height: f64) {
        self.height = height;
    }

    fn set_depth(&mut self, depth: f64) {
        self.depth = depth;
    }

    fn set_max_font_size(&mut self, max_font_size: f64) {
        self.max_font_size = max_font_size;
    }
}

impl HasSizeProperties for DocumentFragment<HtmlDomNode> {
    fn set_height(&mut self, height: f64) {
        self.height = height;
    }

    fn set_depth(&mut self, depth: f64) {
        self.depth = depth;
    }

    fn set_max_font_size(&mut self, max_font_size: f64) {
        self.max_font_size = max_font_size;
    }
}

/// Makes a line span with the given className, options, and thickness
///
/// Creates a span element designed to represent a horizontal line (rule) in
/// mathematical expressions. The line's height is determined by the thickness
/// parameter or falls back to the font's default rule thickness, with a minimum
/// of the minimum rule thickness.
///
/// This function corresponds to the JavaScript `makeLineSpan` function in
/// buildCommon.js.
///
/// # Arguments
/// * `class_name` - CSS class name to apply to the line span
/// * `options` - KaTeX options containing font metrics and styling information
/// * `thickness` - Optional thickness of the line in em units. If None, uses
///   default rule thickness
///
/// # Returns
/// A DomSpan configured as a horizontal line element
#[must_use]
pub fn make_line_span(class_name: &str, options: &Options, thickness: Option<f64>) -> DomSpan {
    let mut line = make_span(vec![class_name.to_owned()], vec![], Some(options), None);

    // Calculate line height: use thickness, or default rule thickness, with minimum
    // threshold
    let default_thickness = options.font_metrics().default_rule_thickness;
    let line_thickness = thickness.unwrap_or(default_thickness);
    line.height = line_thickness.max(options.min_rule_thickness);

    // Set border-bottom-width style to create the visual line
    line.style
        .insert(CssProperty::BorderBottomWidth, make_em(line.height));

    // Set maximum font size to 1.0 for consistent sizing
    line.max_font_size = 1.0;

    line
}

/// Makes an anchor element with the given href, classes, children, and options
///
/// Creates an HTML anchor (`<a>`) element that can contain other DOM nodes as
/// children. The anchor's size properties (height, depth, maxFontSize) are
/// automatically calculated based on its children.
///
/// This function corresponds to the JavaScript `makeAnchor` function in
/// buildCommon.js.
///
/// # Arguments
/// * `href` - URL or reference for the anchor's href attribute
/// * `classes` - CSS classes to apply to the anchor
/// * `children` - Child DOM nodes to be contained within the anchor
/// * `options` - KaTeX options for styling and configuration
///
/// # Returns
/// An Anchor element properly sized based on its children
#[must_use]
pub fn make_anchor(
    href: &str,
    classes: &[String],
    children: &[HtmlDomNode],
    options: &Options,
) -> Anchor {
    // Create attributes map with href
    let mut attributes = KeyMap::default();
    attributes.insert("href".to_owned(), href.to_owned());

    let mut anchor = Anchor::builder()
        .children(children.to_owned())
        .attributes(attributes)
        .classes(classes.to_vec())
        .height(0.0)
        .depth(0.0)
        .max_font_size(options.size_multiplier)
        .build(Some(options));

    // Calculate size properties based on children
    size_element_from_children(&mut anchor, children);

    anchor
}

/// Makes a document fragment with the given list of children
///
/// Creates an HTML document fragment that can contain multiple DOM nodes
/// without creating a wrapper element. The fragment's size properties are
/// automatically calculated based on its children.
///
/// This function corresponds to the JavaScript `makeFragment` function in
/// buildCommon.js.
///
/// # Arguments
/// * `children` - Child DOM nodes to be contained within the fragment
///
/// # Returns
/// An HtmlDocumentFragment properly sized based on its children
#[must_use]
pub fn make_fragment(children: &[HtmlDomNode]) -> HtmlDomFragment {
    let mut fragment = DocumentFragment::new(children.to_owned());

    // Calculate size properties based on children
    size_element_from_children(&mut fragment, children);

    fragment
}

/// Wraps a group in a span if it's a document fragment
///
/// If the input node is a DocumentFragment, wraps it in a span to allow classes
/// and styles to be applied. Otherwise, returns the node unchanged.
///
/// This function corresponds to the JavaScript `wrapFragment` function in
/// buildCommon.js.
///
/// # Arguments
/// * `group` - The DOM node that might need wrapping
/// * `options` - KaTeX options for styling the wrapper span if needed
///
/// # Returns
/// Either the original node (if not a fragment) or a span containing the
/// fragment
#[must_use]
pub fn wrap_fragment(group: HtmlDomNode, options: &Options) -> HtmlDomNode {
    match group {
        HtmlDomNode::Fragment(fragment) => {
            // Wrap the fragment in a span
            let span = make_span(
                vec![],
                vec![HtmlDomNode::Fragment(fragment)],
                Some(options),
                None,
            );
            HtmlDomNode::DomSpan(span)
        }
        _ => group, // Return the node unchanged if it's not a fragment
    }
}

/// Calculate the height, depth, and maxFontSize of an element based on its
/// children
///
/// This helper function corresponds to the JavaScript `sizeElementFromChildren`
/// function and updates the size properties of spans, anchors, and fragments
/// based on their children.
fn size_element_from_children<T>(elem: &mut T, children: &[HtmlDomNode])
where
    T: HasSizeProperties,
{
    let mut height = 0.0;
    let mut depth = 0.0;
    let mut max_font_size = 0.0;

    for child in children {
        if child.height() > height {
            height = child.height();
        }
        if child.depth() > depth {
            depth = child.depth();
        }
        if child.max_font_size() > max_font_size {
            max_font_size = child.max_font_size();
        }
    }

    elem.set_height(height);
    elem.set_depth(depth);
    elem.set_max_font_size(max_font_size);
}
