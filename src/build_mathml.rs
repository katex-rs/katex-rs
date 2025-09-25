//! MathML rendering utilities for KaTeX
//!
//! This module provides functions to convert KaTeX parse trees into MathML
//! format, which is the W3C standard for representing mathematical expressions
//! in XML.

use crate::namespace::KeyMap;
use strum::IntoDiscriminant as _;

use crate::ParseError;
use crate::build_common::{FONT_MAP, make_span};
use crate::context::KatexContext;
use crate::dom_tree::{DomSpan, HtmlDomNode};
use crate::font_metrics::get_character_metrics;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::{FontShape, FontWeight, Options};
use crate::parser::parse_node::AnyParseNode;
use crate::symbols::{Symbols, is_ligature};
use crate::types::Mode;
use crate::types::ParseErrorKind;

/// Creates a MathML text node with optional symbol replacement
///
/// This function creates a MathML text node, applying symbol replacements based
/// on the current mode and rendering options. It handles special cases like
/// ligatures in typewriter fonts and Unicode Mathematical Alphanumeric Symbols.
///
/// # Parameters
/// * `text` - The text content to be placed in the MathML text node
/// * `mode` - The current rendering mode (Math or Text) affecting symbol
///   replacement
/// * `options` - Optional rendering options that may affect symbol replacement
///   behavior
/// * `symbols` - The symbol table containing replacement mappings
///
/// # Returns
/// A `TextNode` containing the processed text content.
///
/// # Symbol Replacement Logic
/// - Applies symbol replacements from the symbol table if available
/// - Skips replacement for ligatures in typewriter fonts (font family ending
///   with "tt")
/// - Preserves Unicode Mathematical Alphanumeric Symbols (U+1D400-U+1D7FF)
///   without replacement
/// - Handles special cases for exact compatibility with KaTeX JavaScript
///   implementation
///
/// # Behavior
/// - Returns the original text if no replacement is found or applicable
/// - Applies font-specific logic for ligature handling
/// - Ensures Unicode math symbols are not inadvertently replaced
#[must_use]
pub fn make_text(text: &str, mode: Mode, options: Option<&Options>, symbols: &Symbols) -> TextNode {
    let mut final_text = text.to_owned();

    // Apply symbol replacements if available (matching JS logic exactly)
    if let Some(char_info) = symbols.get(mode, text)
        && let Some(replace) = &char_info.replace
    {
        // Check if first character is not in the Unicode Mathematical Alphanumeric
        // Symbols block (U+1D400-U+1D7FF)
        let char_code = text.chars().next().unwrap_or('\0') as u32;
        if !(0x1D400..=0x1D7FF).contains(&char_code) {
            // Check for ligature + typewriter font condition
            let skip_replacement = options.is_some_and(|opts| {
                let font_family = &opts.font_family;
                let font = &opts.font;

                // Check if font family or font ends with "tt" (typewriter)
                let is_tt_font = (font_family.len() >= 6 && &font_family[4..6] == "tt")
                    || (font.len() >= 6 && &font[4..6] == "tt");

                // Skip replacement only for ligatures in typewriter fonts
                is_ligature(text) && is_tt_font
            });

            if !skip_replacement {
                final_text = replace.to_string();
            }
        }
    }

    TextNode { text: final_text }
}

/// Wraps nodes in an `<mrow>` element if there are multiple nodes
///
/// This function implements the MathML convention of grouping multiple elements
/// within an `<mrow>` container. Single elements are returned directly without
/// unnecessary wrapping for optimal MathML structure.
///
/// # Parameters
/// * `body` - Slice of MathML DOM nodes to be potentially wrapped
///
/// # Returns
/// - If `body` contains a single element: Returns that element directly
/// - If `body` contains multiple elements: Returns an `<mrow>` element
///   containing all elements
/// - If `body` is empty: Returns an empty `<mrow>` element
///
/// # MathML Convention
/// According to MathML specification, multiple elements should be grouped
/// within an `<mrow>` element to maintain proper structure and spacing. This
/// function automatically applies this convention while avoiding unnecessary
/// nesting for single elements.
///
/// # Use Cases
/// - Building complex mathematical expressions with multiple terms
/// - Ensuring proper MathML structure for operator precedence
/// - Grouping elements for spacing and layout control
#[must_use]
pub fn make_row(body: &[MathDomNode]) -> MathDomNode {
    if body.len() == 1 {
        body[0].clone()
    } else {
        MathDomNode::Math(MathNode {
            node_type: MathNodeType::Mrow,
            attributes: KeyMap::default(),
            children: body.to_vec(),
            classes: Vec::new(),
        })
    }
}

/// Determines the MathML mathvariant attribute for font styling
///
/// This function maps KaTeX font specifications to the appropriate MathML
/// `mathvariant` attribute values. MathML uses a standardized set of variant
/// names for font styling, which differ from KaTeX's internal font naming
/// conventions.
///
/// # Parameters
/// * `group` - The parse node containing text that may need font variant
///   determination
/// * `options` - Rendering options containing font family, shape, and weight
///   settings
/// * `symbols` - Symbol table for checking character metrics and replacements
///
/// # Returns
/// An `Option<&'static str>` containing the MathML mathvariant value:
/// - `Some("monospace")` - For typewriter fonts
/// - `Some("sans-serif")` - For sans-serif fonts
/// - `Some("bold")` - For bold weight
/// - `Some("italic")` - For italic shape
/// - `Some("bold-italic")` - For bold italic combination
/// - Various other MathML variant names for specific fonts
/// - `None` - When no variant should be applied (default math font)
///
/// # Font Mapping Logic
/// - Handles special cases like `\imath` and `\jmath` (always return `None`)
/// - Maps `\text...` font specifiers to MathML variants
/// - Considers font family, shape, and weight combinations
/// - Checks character metrics for font-specific variants
/// - Falls back to symbol replacement information when available
///
/// # MathML Specification Compliance
/// Follows the W3C MathML 3.0 specification for allowable `mathvariant` values
/// as defined at: <https://www.w3.org/TR/MathML3/chapter3.html#presm.commatt>
pub fn get_variant(
    ctx: &KatexContext,
    group: &AnyParseNode,
    options: &Options,
) -> Result<Option<&'static str>, ParseError> {
    // First, get the text and check for special cases that always return None
    let Some(text) = group.text() else {
        return Ok(None);
    };

    // Handle special cases FIRST - \imath and \jmath should never have variants
    if text == "\\imath" || text == "\\jmath" {
        return Ok(None);
    }

    // Handle \text... font specifiers
    // MathML has a limited list of allowable mathvariant specifiers; see
    // https://www.w3.org/TR/MathML3/chapter3.html#presm.commatt
    if options.font_family == "texttt" {
        return Ok(Some("monospace"));
    } else if options.font_family == "textsf" {
        return Ok(Some(match (&options.font_shape, &options.font_weight) {
            (FontShape::TextIt, FontWeight::TextBf) => "sans-serif-bold-italic",
            (FontShape::TextIt, _) => "sans-serif-italic",
            (_, FontWeight::TextBf) => "bold-sans-serif",
            _ => "sans-serif",
        }));
    } else if options.font_shape == FontShape::TextIt && options.font_weight == FontWeight::TextBf {
        return Ok(Some("bold-italic"));
    } else if options.font_shape == FontShape::TextIt {
        return Ok(Some("italic"));
    } else if options.font_weight == FontWeight::TextBf {
        return Ok(Some("bold"));
    }

    let font = &options.font;
    if font.is_empty() || font == "mathnormal" {
        return Ok(None);
    }

    let mode = group.mode();

    if let Some(result) = match font.as_str() {
        "mathit" => Some("italic"),
        "boldsymbol" => match group {
            AnyParseNode::TextOrd(_) => Some("bold"),
            _ => Some("bold-italic"),
        },
        "mathbf" => Some("bold"),
        "mathbb" => Some("double-struck"),
        "mathsfit" => Some("sans-serif-italic"),
        "mathfrak" => Some("fraktur"),
        "mathscr" | "mathcal" => {
            // MathML makes no distinction between script and calligraphic
            Some("script")
        }
        "mathsf" => Some("sans-serif"),
        "mathtt" => Some("monospace"),
        _ => None,
    } {
        return Ok(Some(result));
    }

    // Check for symbol replacement
    let final_text = if let Some(char_info) = ctx.symbols.get(mode, text)
        && let Some(replaced) = char_info.replace
    {
        replaced.to_string()
    } else {
        text.to_owned()
    };

    // Check if we have metrics for this character in the specified font
    if let Some(font_entry) = FONT_MAP.get(font)
        && let Some(final_char) = final_text.chars().next()
        && get_character_metrics(ctx, final_char, font_entry.font_name, mode)?.is_some()
    {
        return Ok(Some(font_entry.variant));
    }

    Ok(None)
}

/// Checks if a node represents number punctuation (dot or comma)
fn is_number_punctuation(group: Option<&MathNode>) -> bool {
    if let Some(node) = group {
        if node.node_type == MathNodeType::Mi && node.children.len() == 1 {
            if let Some(child) = node.children.first()
                && let MathDomNode::Text(text_node) = child
            {
                return text_node.text == ".";
            }
        } else if node.node_type == MathNodeType::Mo && node.children.len() == 1 {
            let has_separator = node
                .attributes
                .get("separator")
                .is_some_and(|s| s == "true");
            let lspace = node.attributes.get("lspace").is_some_and(|s| s == "0em");
            let rspace = node.attributes.get("rspace").is_some_and(|s| s == "0em");

            if has_separator
                && lspace
                && rspace
                && let Some(child) = node.children.first()
                && let MathDomNode::Text(text_node) = child
            {
                return text_node.text == ",";
            }
        }
    }
    false
}

/// Builds a list of MathML nodes from parse nodes with concatenation logic
///
/// This function converts a sequence of parse nodes into MathML DOM nodes,
/// applying various concatenation optimizations for better MathML output
/// structure.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing group builders and rendering state
/// * `expression` - Slice of parse nodes to be converted to MathML DOM nodes
/// * `options` - Rendering options including style, size, and font settings
/// * `is_ordgroup` - Whether this expression represents a ord group that
///   affects spacing
///
/// # Returns
/// A `Result` containing either a vector of MathML DOM nodes or a `ParseError`
/// if building fails.
///
/// # Concatenation Logic
/// The function applies several concatenation optimizations:
/// - Adjacent `<mtext>` elements with matching `mathvariant` attributes
/// - Adjacent `<mn>` elements (numbers)
/// - Numbers followed by number punctuation (dots, commas)
/// - Superscript/subscript operations on numbers or punctuation
/// - `\not` combining with operators (combining long solidus U+0338)
///
/// # Behavior
/// - Single-element expressions return the element directly
/// - Multi-element expressions may be concatenated based on the rules above
/// - Operators in ordgroups have their spacing suppressed (`lspace` and
///   `rspace` set to "0em")
pub fn build_expression(
    ctx: &KatexContext,
    expression: &[AnyParseNode],
    options: &Options,
    is_ordgroup: Option<bool>,
) -> Result<Vec<MathDomNode>, ParseError> {
    if expression.is_empty() {
        return Ok(Vec::new());
    }

    if expression.len() == 1 {
        let group = build_group(ctx, &expression[0], options)?;
        if let Some(math_node) = group.as_math_node()
            && is_ordgroup.unwrap_or(false)
            && math_node.node_type == MathNodeType::Mo
        {
            // Suppress spacing on operators in ordgroups
            let mut new_node = math_node.clone();
            new_node
                .attributes
                .insert("lspace".to_owned(), "0em".to_owned());
            new_node
                .attributes
                .insert("rspace".to_owned(), "0em".to_owned());
            return Ok(vec![MathDomNode::Math(new_node)]);
        }
        return Ok(vec![group]);
    }

    // Use MathDomNodeEnum internally for better performance
    let mut groups_enum: Vec<MathDomNode> = Vec::new();
    let mut last_group: Option<MathDomNode> = None;

    for node in expression {
        let group = build_group(ctx, node, options)?;

        if let (Some(last), Some(current)) = (&last_group, group.as_math_node())
            && let Some(last_math) = last.as_math_node()
        {
            // Concatenate adjacent <mtext> elements
            if current.node_type == MathNodeType::Mtext
                && last_math.node_type == MathNodeType::Mtext
            {
                let mathvariant_match = current.attributes.get("mathvariant")
                    == last_math.attributes.get("mathvariant");
                if mathvariant_match {
                    let mut new_last = last_math.clone();
                    new_last.children.extend(current.children.clone());
                    groups_enum.pop();
                    groups_enum.push(MathDomNode::Math(new_last.clone()));
                    last_group = Some(MathDomNode::Math(new_last));
                    continue;
                }
            }
            // Concatenate adjacent <mn> elements
            // Concatenate <mn> followed by number punctuation
            else if (is_number_punctuation(Some(current))
                || current.node_type == MathNodeType::Mn)
                && last_math.node_type == MathNodeType::Mn
            {
                let mut new_last = last_math.clone();
                new_last.children.extend(current.children.clone());
                groups_enum.pop();
                groups_enum.push(MathDomNode::Math(new_last.clone()));
                last_group = Some(MathDomNode::Math(new_last));
                continue;
            }
            // Concatenate number punctuation followed by <mn>
            else if current.node_type == MathNodeType::Mn
                && is_number_punctuation(Some(last_math))
            {
                let mut new_current = current.clone();
                new_current.children = last_math
                    .children
                    .iter()
                    .cloned()
                    .chain(current.children.iter().cloned())
                    .collect();
                groups_enum.pop();
                groups_enum.push(MathDomNode::Math(new_current.clone()));
                last_group = Some(MathDomNode::Math(new_current));
                continue;
            }
            // Handle msup/msub with preceding mn or punctuation
            else if (current.node_type == MathNodeType::Msup
                || current.node_type == MathNodeType::Msub)
                && !current.children.is_empty()
                && (last_math.node_type == MathNodeType::Mn
                    || is_number_punctuation(Some(last_math)))
            {
                if let Some(base) = current.children.first()
                    && let Some(base_math) = base.as_math_node()
                    && base_math.node_type == MathNodeType::Mn
                {
                    let mut new_base = base_math.clone();
                    new_base.children = last_math
                        .children
                        .iter()
                        .cloned()
                        .chain(base_math.children.iter().cloned())
                        .collect();
                    let mut new_current = current.clone();
                    new_current.children[0] = new_base.into();
                    groups_enum.pop();
                    groups_enum.push(MathDomNode::Math(new_current.clone()));
                    last_group = Some(MathDomNode::Math(new_current));
                    continue;
                }
            }
            // Handle \not combining with operators
            else if last_math.node_type == MathNodeType::Mi
                && last_math.children.len() == 1
                && let Some(last_child) = last_math.children.first()
                && let Some(text_node) = last_child.as_text_node()
                && text_node.text == "\u{0338}"
            {
                // Combining long solidus
                if (current.node_type == MathNodeType::Mo
                    || current.node_type == MathNodeType::Mi
                    || current.node_type == MathNodeType::Mn)
                    && let Some(child) = current.children.first()
                    && let Some(text_child) = child.as_text_node()
                    && !text_child.text.is_empty()
                    && let Some(first_char) = text_child.text.chars().next()
                {
                    let mut new_text = text_child.text.clone();
                    let first_char_len = first_char.len_utf8();
                    new_text.insert(first_char_len, '\u{0338}');
                    let mut new_child = text_child.clone();
                    new_child.text = new_text;
                    let mut new_current = current.clone();
                    new_current.children[0] = new_child.into();
                    groups_enum.pop();
                    groups_enum.push(MathDomNode::Math(new_current.clone()));
                    last_group = Some(MathDomNode::Math(new_current));
                    continue;
                }
            }
        }

        groups_enum.push(group.clone());
        last_group = Some(group);
    }

    Ok(groups_enum)
}

/// Builds a single MathML node from parse nodes, wrapped in mrow if multiple
///
/// This function converts a sequence of parse nodes into a single MathML DOM
/// node, automatically wrapping multiple elements in an `<mrow>` container as
/// required by MathML specification.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing group builders and rendering state
/// * `expression` - Slice of parse nodes to be converted to MathML
/// * `options` - Rendering options including style, size, and font settings
/// * `is_ordgroup` - Whether this expression represents a ord group that
///   affects spacing
///
/// # Returns
/// A `Result` containing either a single MathML DOM node or a `ParseError` if
/// building fails.
///
/// # Behavior
/// - Calls `build_expression` to convert parse nodes to MathML DOM nodes
/// - Applies `make_row` to ensure proper MathML structure:
///   - Single elements are returned directly (no unnecessary `<mrow>`)
///   - Multiple elements are wrapped in an `<mrow>` container
/// - Handles spacing and concatenation logic through the underlying
///   `build_expression` call
///
/// # Use Cases
/// - Building subexpressions that need to be treated as single units
/// - Ensuring proper MathML structure for complex expressions
/// - Creating properly grouped elements for operator precedence
pub fn build_expression_row(
    ctx: &KatexContext,
    expression: &[AnyParseNode],
    options: &Options,
    is_ordgroup: Option<bool>,
) -> Result<MathDomNode, ParseError> {
    let body = build_expression(ctx, expression, options, is_ordgroup)?;
    Ok(make_row(&body))
}

/// Builds a MathML node from a single parse node using the appropriate group
/// builder
///
/// This function is the central dispatcher for converting individual parse
/// nodes into MathML DOM nodes. It determines the appropriate builder function
/// based on the node type and delegates the actual conversion work.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing registered MathML group builders and
///   rendering state
/// * `group` - The parse node to be converted to MathML DOM
/// * `options` - Rendering options including style, size, and font settings
///
/// # Returns
/// A `Result` containing either the built MathML DOM node or a `ParseError` if
/// building fails.
///
/// # Behavior
/// - Determines the node type using `group.discriminant()`
/// - Looks up the appropriate builder function in `ctx.mathml_group_builders`
/// - Calls the registered builder with the provided options
/// - Returns the result from the specific builder function
///
/// # Error Handling
/// Returns `ParseError` if:
/// - No builder is registered for the given node type
/// - The registered builder function fails during execution
///
/// # Builder Registration
/// Builders are registered in the KaTeX context during initialization.
/// Each parse node type (e.g., fractions, symbols, operators) has its own
/// specialized builder function that handles the conversion to appropriate
/// MathML elements.
pub fn build_group(
    ctx: &KatexContext,
    group: &AnyParseNode,
    options: &Options,
) -> Result<MathDomNode, ParseError> {
    let group_type = group.discriminant();
    ctx.mathml_group_builders.get(&group_type).map_or_else(
        || {
            Err(ParseError::new(ParseErrorKind::UnknownGroupType {
                group_type,
            }))
        },
        |builder| builder(group, options, ctx),
    )
}

/// Main entry point for building MathML from a parse tree
///
/// This function converts a complete mathematical expression parse tree into
/// a properly structured MathML document wrapped in an HTML span for rendering.
/// It handles the full MathML document structure including semantics and
/// annotations.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing all necessary builders and rendering
///   state
/// * `tree` - The complete parse tree representing the mathematical expression
/// * `tex_expression` - The original TeX/LaTeX source string for annotation
/// * `options` - Rendering options including style, size, color, and display
///   settings
/// * `is_display_mode` - Whether to render in display mode (block) or inline
///   mode
/// * `for_mathml_only` - Whether this is for MathML-only output (affects
///   wrapper class)
///
/// # Returns
/// A `Result` containing either a `DomSpan` with the complete MathML structure
/// or a `ParseError` if building fails.
///
/// # Behavior
/// - Builds the expression using build_expression with root group settings
/// - Wraps content in proper MathML structure with namespace and display
///   attributes
/// - Adds semantic annotations with the original TeX source
/// - Handles both display and inline math modes
/// - Applies appropriate CSS wrapper classes for styling
///
/// # Error Handling
/// Returns `ParseError` if:
/// - Expression building fails
/// - MathML structure creation encounters issues
/// - DOM manipulation fails
pub fn build_mathml(
    ctx: &KatexContext,
    tree: &[AnyParseNode],
    tex_expression: &str,
    options: &Options,
    is_display_mode: bool,
    for_mathml_only: bool,
) -> Result<DomSpan, ParseError> {
    let expression = build_expression(ctx, tree, options, None)?;

    // Expression is already MathDomNodeEnum
    let expression_enum = expression;

    // Wrap in mrow if needed using MathDomNodeEnum
    let wrapper_enum = if expression_enum.len() == 1 {
        if let Some(math_node) = expression_enum[0].as_math_node() {
            if matches!(
                math_node.node_type,
                MathNodeType::Mrow | MathNodeType::Mtable
            ) {
                expression_enum[0].clone()
            } else {
                MathDomNode::Math(MathNode {
                    node_type: MathNodeType::Mrow,
                    attributes: KeyMap::default(),
                    children: expression_enum.clone(),
                    classes: Vec::new(),
                })
            }
        } else {
            MathDomNode::Math(MathNode {
                node_type: MathNodeType::Mrow,
                attributes: KeyMap::default(),
                children: expression_enum,
                classes: Vec::new(),
            })
        }
    } else {
        MathDomNode::Math(MathNode {
            node_type: MathNodeType::Mrow,
            attributes: KeyMap::default(),
            children: expression_enum,
            classes: Vec::new(),
        })
    };

    // Create annotation using MathDomNodeEnum
    let annotation_enum = MathDomNode::Math(MathNode {
        node_type: MathNodeType::Annotation,
        attributes: KeyMap::default(),
        children: vec![MathDomNode::Text(TextNode {
            text: tex_expression.to_owned(),
        })],
        classes: Vec::new(),
    });

    // Set encoding
    let annotation_with_encoding = if let MathDomNode::Math(mut node) = annotation_enum {
        node.attributes
            .insert("encoding".to_owned(), "application/x-tex".to_owned());
        MathDomNode::Math(node)
    } else {
        annotation_enum
    };

    // Create semantics using MathDomNodeEnum
    let semantics_enum = MathDomNode::Math(MathNode {
        node_type: MathNodeType::Semantics,
        attributes: KeyMap::default(),
        children: vec![wrapper_enum, annotation_with_encoding],
        classes: Vec::new(),
    });

    // Create math element using MathDomNodeEnum
    let mut math_enum = MathDomNode::Math(MathNode {
        node_type: MathNodeType::Math,
        attributes: KeyMap::default(),
        children: vec![semantics_enum],
        classes: Vec::new(),
    });

    // Set namespace and display mode
    if let MathDomNode::Math(ref mut math_node) = math_enum {
        math_node.attributes.insert(
            "xmlns".to_owned(),
            "http://www.w3.org/1998/Math/MathML".to_owned(),
        );

        if is_display_mode {
            math_node
                .attributes
                .insert("display".to_owned(), "block".to_owned());
        }
    }

    // Convert back to MathNode for HtmlNode::MathML
    let math_node = if let MathDomNode::Math(node) = math_enum {
        node
    } else {
        // Fallback
        MathNode::builder().node_type(MathNodeType::Math).build()
    };

    // Wrap in span for styling
    let wrapper_class = if for_mathml_only {
        "katex"
    } else {
        "katex-mathml"
    };

    Ok(make_span(
        vec![wrapper_class.to_owned()],
        vec![HtmlDomNode::MathML(math_node)],
        None,
        None,
    ))
}
