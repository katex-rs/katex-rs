//! Stretchy wide elements rendered from SVG files and CSS tricks
//!
//! This module provides support for building stretchy wide elements that are
//! rendered using SVG files and CSS overflow techniques. It includes functions
//! for creating SVG spans, enclosing spans, and MathML nodes for stretchy
//! symbols.

use alloc::borrow::Cow;

use crate::ParseError;
use crate::build_common::{make_span, make_svg_span};
use crate::dom_tree::{DomSpan, HtmlDomNode, LineNode, PathNode, SvgChildNode, SvgNode};
use crate::mathml_tree::{MathNode, MathNodeType, TextNode};
use crate::namespace::KeyMap;
use crate::options::Options;
use crate::parser::parse_node::AnyParseNode;
use crate::types::ClassList;
use crate::types::CssProperty;
use crate::types::ParseErrorKind;
use crate::units::make_em;
use phf::{phf_map, phf_set};

/// Code point mapping for stretchy symbols
pub const STRETCHY_CODE_POINT: phf::Map<&'static str, &'static str> = phf_map! {
    "widehat" => "^",
    "widecheck" => "\u{2c7}",
    "widetilde" => "~",
    "utilde" => "~",
    "overleftarrow" => "\u{2190}",
    "underleftarrow" => "\u{2190}",
    "xleftarrow" => "\u{2190}",
    "overrightarrow" => "\u{2192}",
    "underrightarrow" => "\u{2192}",
    "xrightarrow" => "\u{2192}",
    "underbrace" => "\u{23df}",
    "overbrace" => "\u{23de}",
    "overgroup" => "\u{23e0}",
    "undergroup" => "\u{23e1}",
    "overleftrightarrow" => "\u{2194}",
    "underleftrightarrow" => "\u{2194}",
    "xleftrightarrow" => "\u{2194}",
    "Overrightarrow" => "\u{21d2}",
    "xRightarrow" => "\u{21d2}",
    "overleftharpoon" => "\u{21bc}",
    "xleftharpoonup" => "\u{21bc}",
    "overrightharpoon" => "\u{21c0}",
    "xrightharpoonup" => "\u{21c0}",
    "xLeftarrow" => "\u{21d0}",
    "xLeftrightarrow" => "\u{21d4}",
    "xhookleftarrow" => "\u{21a9}",
    "xhookrightarrow" => "\u{21aa}",
    "xmapsto" => "\u{21a6}",
    "xrightharpoondown" => "\u{21c1}",
    "xleftharpoondown" => "\u{21bd}",
    "xrightleftharpoons" => "\u{21cc}",
    "xleftrightharpoons" => "\u{21cb}",
    "xtwoheadleftarrow" => "\u{219e}",
    "xtwoheadrightarrow" => "\u{21a0}",
    "xlongequal" => "=",
    "xtofrom" => "\u{21c4}",
    "xrightleftarrows" => "\u{21c4}",
    "xrightequilibrium" => "\u{21cc}",
    "xleftequilibrium" => "\u{21cb}",
    "\\cdrightarrow" => "\u{2192}",
    "\\cdleftarrow" => "\u{2190}",
    "\\cdlongequal" => "=",
};

/// Data structure for image information
#[derive(Debug, Clone)]
pub struct ImageData {
    /// SVG path names or path arrays
    pub paths: &'static [&'static str],
    /// Minimum width in em units
    pub min_width: f64,
    /// Height in em units
    pub height: f64,
    /// Optional alignment string
    pub align: Option<&'static str>,
}

impl ImageData {
    /// Create a new [`ImageData`] instance
    #[must_use]
    pub const fn new(
        paths: &'static [&'static str],
        min_width: f64,
        height: f64,
        align: Option<&'static str>,
    ) -> Self {
        Self {
            paths,
            min_width,
            height,
            align,
        }
    }
}

const IMAGES_DATA: phf::Map<&'static str, ImageData> = phf_map! {
    "overrightarrow" => ImageData::new(&["rightarrow"], 0.888, 522.0, Some("xMaxYMin")),
    "overleftarrow" => ImageData::new(&["leftarrow"], 0.888, 522.0, Some("xMinYMin")),
    "underrightarrow" => ImageData::new(&["rightarrow"], 0.888, 522.0, Some("xMaxYMin")),
    "underleftarrow" => ImageData::new(&["leftarrow"], 0.888, 522.0, Some("xMinYMin")),
    "xrightarrow" => ImageData::new(&["rightarrow"], 1.469, 522.0, Some("xMaxYMin")),
    "\\cdrightarrow" => ImageData::new(&["rightarrow"], 3.0, 522.0, Some("xMaxYMin")),
    "xleftarrow" => ImageData::new(&["leftarrow"], 1.469, 522.0, Some("xMinYMin")),
    "\\cdleftarrow" => ImageData::new(&["leftarrow"], 3.0, 522.0, Some("xMinYMin")),
    "Overrightarrow" => ImageData::new(&["doublerightarrow"], 0.888, 560.0, Some("xMaxYMin")),
    "xRightarrow" => ImageData::new(&["doublerightarrow"], 1.526, 560.0, Some("xMaxYMin")),
    "xLeftarrow" => ImageData::new(&["doubleleftarrow"], 1.526, 560.0, Some("xMinYMin")),
    "overleftharpoon" => ImageData::new(&["leftharpoon"], 0.888, 522.0, Some("xMinYMin")),
    "xleftharpoonup" => ImageData::new(&["leftharpoon"], 0.888, 522.0, Some("xMinYMin")),
    "xleftharpoondown" => ImageData::new(&["leftharpoondown"], 0.888, 522.0, Some("xMinYMin")),
    "overrightharpoon" => ImageData::new(&["rightharpoon"], 0.888, 522.0, Some("xMaxYMin")),
    "xrightharpoonup" => ImageData::new(&["rightharpoon"], 0.888, 522.0, Some("xMaxYMin")),
    "xrightharpoondown" => ImageData::new(&["rightharpoondown"], 0.888, 522.0, Some("xMaxYMin")),
    "xlongequal" => ImageData::new(&["longequal"], 0.888, 334.0, Some("xMinYMin")),
    "\\cdlongequal" => ImageData::new(&["longequal"], 3.0, 334.0, Some("xMinYMin")),
    "xtwoheadleftarrow" => ImageData::new(&["twoheadleftarrow"], 0.888, 334.0, Some("xMinYMin")),
    "xtwoheadrightarrow" => ImageData::new(&["twoheadrightarrow"], 0.888, 334.0, Some("xMaxYMin")),
    "overleftrightarrow" => ImageData::new(&["leftarrow", "rightarrow"], 0.888, 522.0, None),
    "overbrace" => ImageData::new(&["leftbrace", "midbrace", "rightbrace"], 1.6, 548.0, None),
    "underbrace" => ImageData::new(&["leftbraceunder", "midbraceunder", "rightbraceunder"], 1.6, 548.0, None),
    "underleftrightarrow" => ImageData::new(&["leftarrow", "rightarrow"], 0.888, 522.0, None),
    "xleftrightarrow" => ImageData::new(&["leftarrow", "rightarrow"], 1.75, 522.0, None),
    "xLeftrightarrow" => ImageData::new(&["doubleleftarrow", "doublerightarrow"], 1.75, 560.0, None),
    "xrightleftharpoons" => ImageData::new(&["leftharpoondownplus", "rightharpoonplus"], 1.75, 716.0, None),
    "xleftrightharpoons" => ImageData::new(&["leftharpoonplus", "rightharpoondownplus"], 1.75, 716.0, None),
    "xhookleftarrow" => ImageData::new(&["leftarrow", "righthook"], 1.08, 522.0, None),
    "xhookrightarrow" => ImageData::new(&["lefthook", "rightarrow"], 1.08, 522.0, None),
    "overlinesegment" => ImageData::new(&["leftlinesegment", "rightlinesegment"], 0.888, 522.0, None),
    "underlinesegment" => ImageData::new(&["leftlinesegment", "rightlinesegment"], 0.888, 522.0, None),
    "overgroup" => ImageData::new(&["leftgroup", "rightgroup"], 0.888, 342.0, None),
    "undergroup" => ImageData::new(&["leftgroupunder", "rightgroupunder"], 0.888, 342.0, None),
    "xmapsto" => ImageData::new(&["leftmapsto", "rightarrow"], 1.5, 522.0, None),
    "xtofrom" => ImageData::new(&["leftToFrom", "rightToFrom"], 1.75, 528.0, None),
    "xrightleftarrows" => ImageData::new(&["baraboveleftarrow", "rightarrowabovebar"], 1.75, 901.0, None),
    "xrightequilibrium" => ImageData::new(&["baraboveshortleftharpoon", "rightharpoonaboveshortbar"], 1.75, 716.0, None),
    "xleftequilibrium" => ImageData::new(&["shortbaraboveleftharpoon", "shortrightharpoonabovebar"], 1.75, 716.0, None),
};

/// Calculate the length of an ordgroup parse node
const fn group_length(arg: &AnyParseNode) -> usize {
    if let AnyParseNode::OrdGroup(ordgroup) = arg {
        ordgroup.body.len()
    } else {
        1
    }
}

const ACCENT_STRETCHY: phf::Set<&'static str> = phf_set! {
    "widehat", "widecheck", "widetilde", "utilde"
};

const ACCENT_STRETCHY_OVER: phf::Set<&'static str> = phf_set! {
    "widehat", "widecheck"
};

/// Create an SVG span for stretchy elements
pub fn svg_span(group: &AnyParseNode, options: &Options) -> Result<HtmlDomNode, ParseError> {
    // Extract the label from the group
    let Some(label) = group.label() else {
        return Err(ParseError::new(
            ParseErrorKind::UnsupportedGroupTypeForSvgSpan,
        ));
    };

    let Some(label) = label.strip_prefix('\\') else {
        return Err(ParseError::new(ParseErrorKind::LabelMissingBackslashPrefix));
    };

    if ACCENT_STRETCHY.contains(label) {
        // Handle accent-style stretchy elements
        let grp_base = match group {
            AnyParseNode::Accent(acc) => &acc.base,
            AnyParseNode::AccentUnder(acc_under) => &acc_under.base,
            _ => {
                return Err(ParseError::new(ParseErrorKind::InvalidGroupTypeForAccent));
            }
        };

        let num_chars = group_length(grp_base) as f64;
        let (view_box_width, view_box_height, height_val, path_name) = if num_chars > 5.0 {
            if ACCENT_STRETCHY_OVER.contains(label) {
                (2364.0, 420.0, 0.42, format!("{label}4"))
            } else {
                (2340.0, 312.0, 0.34, "tilde4".to_owned())
            }
        } else {
            let img_index = [1, 1, 2, 2, 3, 3][num_chars as usize];
            if ACCENT_STRETCHY_OVER.contains(label) {
                let widths = [0.0, 1062.0, 2364.0, 2364.0, 2364.0];
                let heights = [0.0, 239.0, 300.0, 360.0, 420.0];
                let h_vals = [0.0, 0.24, 0.3, 0.3, 0.36, 0.42];
                (
                    widths[img_index],
                    heights[img_index],
                    h_vals[img_index],
                    format!("{label}{img_index}"),
                )
            } else {
                let widths = [0.0, 600.0, 1033.0, 2339.0, 2340.0];
                let heights = [0.0, 260.0, 286.0, 306.0, 312.0];
                let h_vals = [0.0, 0.26, 0.286, 0.3, 0.306, 0.34];
                (
                    widths[img_index],
                    heights[img_index],
                    h_vals[img_index],
                    format!("tilde{img_index}"),
                )
            }
        };

        let path = PathNode {
            path_name,
            alternate: None,
        };

        let mut svg_node = SvgNode::builder()
            .children(vec![SvgChildNode::Path(path)])
            .build();
        svg_node.attributes.extend([
            ("width".to_owned(), "100%".to_owned()),
            ("height".to_owned(), make_em(height_val)),
            (
                "viewBox".to_owned(),
                format!("0 0 {view_box_width} {view_box_height}"),
            ),
            ("preserveAspectRatio".to_owned(), "none".to_owned()),
        ]);
        let mut span = make_svg_span(vec![], vec![svg_node], options);

        span.height = height_val;
        span.style.insert(CssProperty::Height, make_em(height_val));

        Ok(span.into())
    } else {
        // Handle other stretchy elements
        let data = IMAGES_DATA.get(label).ok_or_else(|| {
            ParseError::new(ParseErrorKind::UnknownStretchyElement {
                label: label.to_owned(),
            })
        })?;

        let mut spans: Vec<HtmlDomNode> = Vec::new();
        let height_val = data.height / 1000.0;
        let view_box_width = 400000.0;

        let (width_classes, aligns): (&[&str], &[&str]) = match data.paths.len() {
            1 => {
                let align = data.align.unwrap_or("xMinYMin");
                (&["hide-tail"], &[align])
            }
            2 => (
                &["halfarrow-left", "halfarrow-right"],
                &["xMinYMin", "xMaxYMin"],
            ),
            3 => (
                &["brace-left", "brace-center", "brace-right"],
                &["xMinYMin", "xMidYMin", "xMaxYMin"],
            ),
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::UnsupportedStretchyPathCount {
                        count: data.paths.len(),
                    },
                ));
            }
        };

        for (i, (width_class, align)) in width_classes.iter().zip(aligns.iter()).enumerate() {
            let path = PathNode {
                path_name: data.paths[i].to_owned(),
                alternate: None,
            };

            let mut svg_node = SvgNode::builder()
                .children(vec![SvgChildNode::Path(path)])
                .build();

            svg_node.attributes.extend([
                ("width".to_owned(), "400em".to_owned()),
                ("height".to_owned(), make_em(height_val)),
                (
                    "viewBox".to_owned(),
                    format!("0 0 {} {}", view_box_width, data.height),
                ),
                ("preserveAspectRatio".to_owned(), format!("{align} slice")),
            ]);

            let span = make_span(
                ClassList::Static(width_class),
                vec![HtmlDomNode::SvgNode(svg_node)],
                Some(options),
                None,
            );

            let mut span = span;
            if data.paths.len() == 1 {
                // For single path, return directly
                span.height = height_val;
                span.style.insert(CssProperty::Height, make_em(height_val));
                if data.min_width > 0.0 {
                    span.style
                        .insert(CssProperty::MinWidth, make_em(data.min_width));
                }
                return Ok(span.into());
            }

            // For multiple paths, collect spans
            span.height = height_val;
            span.style.insert(CssProperty::Height, make_em(height_val));
            spans.push(span.into());
        }

        // For multiple paths, create a stretchy span containing all spans
        let mut span = make_span("stretchy", spans, Some(options), None);
        span.height = height_val;
        span.style.insert(CssProperty::Height, make_em(height_val));
        if data.min_width > 0.0 {
            span.style
                .insert(CssProperty::MinWidth, make_em(data.min_width));
        }

        Ok(span.into())
    }
}

/// Create an enclosing span for elements like cancel, fbox, etc.
pub fn enclose_span(
    inner: &HtmlDomNode,
    label: &str,
    top_pad: f64,
    bottom_pad: f64,
    options: &Options,
) -> DomSpan {
    let total_height = inner.height() + inner.depth() + top_pad + bottom_pad;

    let is_box_like = label.contains("fbox") || label.contains("color");
    if is_box_like || label == "angl" {
        let classes = vec![Cow::Borrowed("stretchy"), Cow::Owned(label.to_owned())];
        let mut span = make_span(classes, vec![], Some(options), None);

        if label == "fbox"
            && let Some(color) = options.get_color()
        {
            span.style.insert(CssProperty::BorderColor, color);
        }

        span.style
            .insert(CssProperty::Height, make_em(total_height));
        span.height = total_height;
        span
    } else {
        // Handle cancel, bcancel, xcancel
        let mut lines = Vec::new();

        if label == "bcancel" || label == "xcancel" {
            lines.push(LineNode {
                attributes: [
                    ("x1".to_owned(), "0".to_owned()),
                    ("y1".to_owned(), "0".to_owned()),
                    ("x2".to_owned(), "100%".to_owned()),
                    ("y2".to_owned(), "100%".to_owned()),
                    ("stroke-width".to_owned(), "0.046em".to_owned()),
                ]
                .iter()
                .cloned()
                .collect(),
            });
        }

        if label == "cancel" || label == "xcancel" {
            lines.push(LineNode {
                attributes: [
                    ("x1".to_owned(), "0".to_owned()),
                    ("y1".to_owned(), "100%".to_owned()),
                    ("x2".to_owned(), "100%".to_owned()),
                    ("y2".to_owned(), "0".to_owned()),
                    ("stroke-width".to_owned(), "0.046em".to_owned()),
                ]
                .iter()
                .cloned()
                .collect(),
            });
        }

        let svg_attributes = [
            ("width".to_owned(), "100%".to_owned()),
            ("height".to_owned(), make_em(total_height)),
        ]
        .iter()
        .cloned()
        .collect();

        let svg_node = SvgNode::builder()
            .children(lines.into_iter().map(SvgChildNode::Line).collect())
            .attributes(svg_attributes)
            .build();

        let mut span = make_svg_span(vec![], vec![svg_node], options);
        span.style
            .insert(CssProperty::Height, make_em(total_height));
        span.height = total_height;
        span
    }
}

/// Create a MathML node for stretchy elements
#[must_use]
pub fn math_ml_node(label: &str) -> MathNode {
    let code_point = STRETCHY_CODE_POINT
        .get(label.trim_start_matches('\\'))
        .unwrap_or(&" ");

    let text_node = TextNode {
        text: (*code_point).to_owned(),
    };

    let mut node = MathNode {
        node_type: MathNodeType::Mo,
        attributes: KeyMap::default(),
        children: vec![text_node.into()],
        classes: ClassList::Empty,
    };

    node.attributes
        .insert("stretchy".to_owned(), "true".to_owned());
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom_tree::Span;
    use crate::parser::parse_node::ParseNodeMathOrd;
    use crate::types::Mode;

    #[test]
    fn test_stretchy_code_point() {
        assert_eq!(STRETCHY_CODE_POINT.get("widehat"), Some(&"^"));
        assert_eq!(STRETCHY_CODE_POINT.get("overleftarrow"), Some(&"\u{2190}"));
        assert_eq!(STRETCHY_CODE_POINT.get("nonexistent"), None);
    }

    #[test]
    fn test_get_katex_images_data() {
        let data = IMAGES_DATA;
        assert!(data.contains_key("overrightarrow"));
        assert!(data.contains_key("overbrace"));

        let overrightarrow = data.get("overrightarrow").unwrap();
        assert_eq!(overrightarrow.min_width, 0.888f64);
        assert_eq!(overrightarrow.height, 522f64);
        assert_eq!(overrightarrow.align, Some("xMaxYMin"));
    }

    #[test]
    fn test_group_length() {
        // Test with a simple non-ordgroup
        let simple_node = AnyParseNode::MathOrd(ParseNodeMathOrd {
            mode: Mode::Math,
            loc: None,
            text: "x".into(),
        });
        assert_eq!(group_length(&simple_node), 1);
    }

    #[test]
    fn test_svg_span_basic_functionality() {
        use crate::options::Options;
        use crate::style;

        let options = Options::builder()
            .style(style::TEXT)
            .phantom(false)
            .max_size(1_000_000.0)
            .min_rule_thickness(0.04)
            .build();

        // Test that the function exists and can be called
        // We'll use a simple test that should work with our current implementation
        let simple_node = AnyParseNode::MathOrd(ParseNodeMathOrd {
            mode: Mode::Math,
            loc: None,
            text: "x".into(),
        });

        // This should not panic and should return an error for unsupported node type
        let result = svg_span(&simple_node, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_math_ml_node() {
        let node = math_ml_node("widehat");
        assert_eq!(node.node_type, MathNodeType::Mo);
        assert_eq!(node.attributes.get("stretchy"), Some(&"true".to_owned()));
        assert_eq!(node.children.len(), 1);
    }

    #[test]
    fn test_enclose_span() {
        use crate::options::Options;
        use crate::style;

        let options = Options::builder()
            .style(style::TEXT)
            .phantom(false)
            .max_size(1_000_000.0)
            .min_rule_thickness(0.04)
            .build();

        let inner: Span<HtmlDomNode> = Span::builder()
            .children(vec![])
            .height(1.0)
            .depth(0.5)
            .build(None);

        let result = enclose_span(&HtmlDomNode::DomSpan(inner), "cancel", 0.1, 0.1, &options);
        assert!(result.height > 0.0);
        assert!(result.style.contains_key(CssProperty::Height));
    }
}
