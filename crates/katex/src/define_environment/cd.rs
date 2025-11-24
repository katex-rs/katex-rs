//! CD (Commutative Diagram) environment implementation
//!
//! This module provides the Rust implementation of KaTeX's CD environment,
//! which is used to create commutative diagrams with arrows and labels.

use crate::build_html::build_group;
use crate::macros::MacroDefinition;
use crate::mathml_tree::{MathNode, MathNodeType};
use crate::parser::parse_node::NodeType::{CdLabel, CdLabelParent};
use crate::parser::parse_node::{
    ParseNodeAtom, ParseNodeCdLabel, ParseNodeCdLabelParent, ParseNodeTextOrd,
};
use crate::types::CssProperty;
use crate::units::make_em;
use crate::{build_mathml, wrap_fragment};
use crate::{
    define_function::{FunctionDefSpec, FunctionPropSpec},
    macros::MacroContextInterface as _,
    parser::{
        Parser,
        parse_node::{
            AlignSpec, AnyParseNode, ColSeparationType, ParseNode, ParseNodeArray,
            ParseNodeOrdGroup, ParseNodeStyling,
        },
    },
    style::DISPLAY,
    symbols::Atom,
    types::{BreakToken, Mode, ParseError, ParseErrorKind, TokenText},
    utils::push_and_get_mut,
};
use phf::phf_map;

/// Arrow function name mapping for CD environment
const CD_ARROW_FUNCTION_NAMES: phf::Map<&str, &str> = phf_map!(
    ">" => "\\\\cdrightarrow",
    "<" => "\\\\cdleftarrow",
    "=" => "\\\\cdlongequal",
    "A" => "\\uparrow",
    "V" => "\\downarrow",
    "|" => "\\Vert",
    "." => "no arrow",
);

/// Create an empty cell for CD environment
const fn new_cell() -> ParseNode {
    ParseNode::Styling(ParseNodeStyling {
        mode: Mode::Math,
        loc: None,
        style: DISPLAY,
        body: vec![],
    })
}

/// Check if a node is the start of an arrow (@)
fn is_start_of_arrow(node: &AnyParseNode) -> bool {
    matches!(node, AnyParseNode::TextOrd(text_ord) if text_ord.text == "@")
}

/// Check if a node is a label end character
fn is_label_end(node: &AnyParseNode, end_char: &str) -> bool {
    match node {
        AnyParseNode::MathOrd(math_ord) => math_ord.text == end_char,
        AnyParseNode::Atom(atom) => atom.text == end_char,
        _ => false,
    }
}

/// Create an arrow node with labels
fn cd_arrow(
    arrow_char: &str,
    labels: &[ParseNode],
    parser: &mut Parser,
) -> Result<AnyParseNode, ParseError> {
    let func_name = CD_ARROW_FUNCTION_NAMES
        .get(arrow_char)
        .unwrap_or(&"no arrow");

    assert!(labels.len() >= 2);

    match func_name {
        &"\\\\cdrightarrow" | &"\\\\cdleftarrow" => parser.call_function(
            func_name,
            vec![labels[0].clone()],
            vec![Some(labels[1].clone())],
            None,
            None,
        ),
        &"\\uparrow" | &"\\downarrow" => {
            let left_label =
                parser.call_function("\\\\cdleft", vec![labels[0].clone()], vec![], None, None)?;
            let bare_arrow = AnyParseNode::Atom(ParseNodeAtom {
                family: Atom::Rel,
                mode: Mode::Math,
                loc: None,
                text: TokenText::from(*func_name),
            });
            let sized_arrow =
                parser.call_function("\\Big", vec![bare_arrow], vec![], None, None)?;
            let right_label =
                parser.call_function("\\\\cdright", vec![labels[1].clone()], vec![], None, None)?;
            let arrow_group = AnyParseNode::OrdGroup(ParseNodeOrdGroup {
                mode: Mode::Math,
                loc: None,
                body: vec![left_label, sized_arrow, right_label],
                semisimple: None,
            });
            parser.call_function("\\\\cdparent", vec![arrow_group], vec![], None, None)
        }
        &"\\\\cdlongequal" => parser.call_function("\\\\cdlongequal", vec![], vec![], None, None),
        &"\\Vert" => {
            let arrow = AnyParseNode::TextOrd(ParseNodeTextOrd {
                mode: Mode::Math,
                loc: None,
                text: TokenText::from("\\Vert"),
            });
            parser.call_function("\\Big", vec![arrow], vec![], None, None)
        }
        _ => Ok(AnyParseNode::TextOrd(ParseNodeTextOrd {
            mode: Mode::Math,
            loc: None,
            text: TokenText::from(" "),
        })),
    }
}

/// Parse CD environment content
pub fn parse_cd(parser: &mut Parser) -> Result<AnyParseNode, ParseError> {
    let mut parsed_rows: Vec<Vec<AnyParseNode>> = Vec::new();

    parser.gullet.begin_group();
    parser.gullet.macros_mut().set(
        "\\cr",
        Some(MacroDefinition::String("\\\\\\relax".to_owned())),
        false,
    );
    parser.gullet.begin_group();

    loop {
        let row_nodes = parser.parse_expression(false, Some(&BreakToken::DoubleBackslash))?;
        parsed_rows.push(row_nodes);
        parser.gullet.end_group()?;
        parser.gullet.begin_group();

        let next_text = parser.fetch()?.text.to_owned_string();
        if next_text == "&" || next_text == "\\\\" {
            parser.consume();
        } else if next_text == "\\end" {
            if parsed_rows.last().is_some_and(Vec::is_empty) {
                parsed_rows.pop();
            }
            break;
        } else {
            return Err(ParseError::new(ParseErrorKind::ExpectedCdDelimiter {
                found: next_text,
            }));
        }
    }

    let mut body = vec![Vec::new()];
    let mut row = &mut body[0];

    // Process parsed rows into cells and arrows
    for (i, row_nodes) in parsed_rows.iter().enumerate() {
        // Start a new row
        let mut cell = new_cell();
        let mut j = 0;
        while j < row_nodes.len() {
            let node = &row_nodes[j];
            if is_start_of_arrow(node) {
                // Parse arrow
                row.push(cell);

                // Get arrow character
                j += 1;
                if j >= row_nodes.len() {
                    return Err(ParseError::new(
                        ParseErrorKind::MissingArrowCharacterAfterAt,
                    ));
                }
                let Some(arrow_char) = row_nodes[j].text() else {
                    return Err(ParseError::new(ParseErrorKind::InvalidArrowCharacter));
                };

                // Create labels
                let mut labels = [
                    AnyParseNode::OrdGroup(ParseNodeOrdGroup {
                        mode: Mode::Math,
                        loc: None,
                        body: vec![],
                        semisimple: None,
                    }),
                    AnyParseNode::OrdGroup(ParseNodeOrdGroup {
                        mode: Mode::Math,
                        loc: None,
                        body: vec![],
                        semisimple: None,
                    }),
                ];

                // Process labels based on arrow type
                if "=|.".contains(arrow_char) {
                    // No labels
                } else if "<>AV".contains(arrow_char) {
                    // Parse labels
                    for label in labels.iter_mut().take(2) {
                        let mut in_label = true;
                        let mut k = j + 1;
                        while k < row_nodes.len() {
                            if is_label_end(&row_nodes[k], arrow_char) {
                                in_label = false;
                                j = k;
                                break;
                            }
                            if is_start_of_arrow(&row_nodes[k]) {
                                return Err(ParseError::new(ParseErrorKind::MissingCdArrowChar {
                                    arrow: arrow_char.to_owned(),
                                }));
                            }
                            if let AnyParseNode::OrdGroup(ord_group) = label {
                                ord_group.body.push(row_nodes[k].clone());
                            }
                            k += 1;
                        }
                        if in_label {
                            return Err(ParseError::new(ParseErrorKind::MissingCdArrowChar {
                                arrow: arrow_char.to_owned(),
                            }));
                        }
                    }
                } else {
                    return Err(ParseError::new(ParseErrorKind::InvalidCdArrowSpecifier {
                        found: arrow_char.to_owned(),
                    }));
                }

                // Create arrow
                let arrow = cd_arrow(
                    arrow_char,
                    &labels
                        .iter()
                        .map(|l| ParseNode::from(l.clone()))
                        .collect::<Vec<_>>(),
                    parser,
                )?;

                // Wrap arrow in styling
                let wrapped_arrow = ParseNode::Styling(ParseNodeStyling {
                    mode: Mode::Math,
                    loc: None,
                    style: DISPLAY,
                    body: vec![ParseNode::from(arrow)],
                });
                row.push(wrapped_arrow);

                // Create new empty cell
                cell = new_cell();
            } else {
                // If not an arrow, add to cell
                if let ParseNode::Styling(styling) = &mut cell {
                    styling.body.push(node.clone());
                }
            }

            j += 1;
        }

        if i % 2 == 0 {
            // Even rows: cell, arrow, cell, arrow, ... cell
            row.push(cell);
        } else {
            // Odd rows: vert arrow, empty cell, ... vert arrow
            if !row.is_empty() {
                row.remove(0); // Remove empty cell at beginning
            }
        }

        row = push_and_get_mut(&mut body, Vec::new());
    }

    // End row group
    parser.gullet.end_group()?;
    // End array group defining \\
    parser.gullet.end_group()?;

    // Define column separation
    let cols = vec![
        AlignSpec::Align {
            align: "c".to_owned(),
            pregap: Some(0.25),
            postgap: Some(0.25),
        };
        body.first().map_or(0, Vec::len)
    ];

    let body_len = body.len();

    Ok(AnyParseNode::Array(ParseNodeArray {
        mode: Mode::Math,
        loc: None,
        col_separation_type: Some(ColSeparationType::CD),
        hskip_before_and_after: None,
        add_jot: Some(true),
        cols: Some(cols),
        arraystretch: 1.0,
        body,
        row_gaps: vec![None; body_len],
        h_lines_before_row: vec![vec![]; body_len + 1],
        tags: None,
        leqno: None,
        is_cd: Some(true),
    }))
}

/// Define CD-related functions in the KaTeX context
pub fn define_cd(ctx: &mut crate::KatexContext) {
    // Define \\cdleft, \\cdright function
    ctx.define_function(FunctionDefSpec {
        node_type: Some(CdLabel),
        names: &["\\\\cdleft", "\\\\cdright"],
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            Ok(ParseNode::CdLabel(ParseNodeCdLabel {
                mode: context.parser.mode,
                loc: None,
                side: context.func_name[4..].to_owned(),
                label: Box::new(args[0].clone()),
            }))
        }),
        html_builder: Some(|node, options, ctx| {
            let ParseNode::CdLabel(group) = node else {
                return Err(ParseError::new(ParseErrorKind::InvalidNodeTypeForBuilder {
                    builder: "cdlabel_html_builder",
                }));
            };
            let new_options = options.having_style(options.style.sup());
            let mut label = wrap_fragment(
                build_group(ctx, &group.label, &new_options, Some(options))?,
                options,
            );
            if let Some(classes) = label.classes_mut() {
                classes.push(format!("cd-label-{}", group.side));
            }
            let depth = label.depth();
            if let Some(style) = label.style_mut() {
                style.insert(CssProperty::Bottom, make_em(0.8 - depth));
            }
            if let Some(height) = label.height_mut() {
                *height = 0.0;
            }
            if let Some(depth) = label.depth_mut() {
                *depth = 0.0;
            }
            Ok(label)
        }),
        mathml_builder: Some(|node, options, ctx| {
            let ParseNode::CdLabel(group) = node else {
                return Err(ParseError::new(ParseErrorKind::InvalidNodeTypeForBuilder {
                    builder: "cdlabel_mathml_builder",
                }));
            };
            let label = MathNode::builder()
                .node_type(MathNodeType::Mrow)
                .children(vec![build_mathml::build_group(ctx, &group.label, options)?])
                .build();
            let mut label = MathNode::builder()
                .node_type(MathNodeType::Mpadded)
                .children(vec![label.into()])
                .build();
            if group.side == "left" {
                label.set_attribute("width", "-1width");
            }
            label.set_attribute("voffset", "0.7em");
            let mut label = MathNode::builder()
                .node_type(MathNodeType::Mstyle)
                .children(vec![label.into()])
                .build();
            label.set_attribute("displaystyle", "false");
            label.set_attribute("scriptlevel", "1");
            Ok(label.into())
        }),
    });

    // Define \\cdparent function
    ctx.define_function(FunctionDefSpec {
        node_type: Some(CdLabelParent),
        names: &["\\\\cdparent"],
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            Ok(ParseNode::CdLabelParent(ParseNodeCdLabelParent {
                mode: context.parser.mode,
                loc: None,
                fragment: Box::new(args[0].clone()),
            }))
        }),
        html_builder: Some(|node, options, ctx| {
            let ParseNode::CdLabelParent(group) = node else {
                return Err(ParseError::new(ParseErrorKind::InvalidNodeTypeForBuilder {
                    builder: "cdparent_html_builder",
                }));
            };
            let mut parent =
                wrap_fragment(build_group(ctx, &group.fragment, options, None)?, options);
            if let Some(classes) = parent.classes_mut() {
                classes.push("cd-vert-arrow");
            }
            Ok(parent)
        }),
        mathml_builder: Some(|node, options, ctx| {
            let ParseNode::CdLabelParent(group) = node else {
                return Err(ParseError::new(ParseErrorKind::InvalidNodeTypeForBuilder {
                    builder: "cdparent_html_builder",
                }));
            };
            Ok(MathNode::builder()
                .node_type(MathNodeType::Mrow)
                .children(vec![build_mathml::build_group(
                    ctx,
                    &group.fragment,
                    options,
                )?])
                .build()
                .into())
        }),
    });
}
