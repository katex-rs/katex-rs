//! HTML building utilities for KaTeX Rust implementation
//!
//! This module provides functions for building HTML DOM nodes from parse trees,
//! migrated from the JavaScript buildHTML.js file.

use crate::build_common::{make_span, try_combine_chars};
use crate::dom_tree::{DomSpan, HtmlDomNode};
use crate::options::Options;
use crate::parser::parse_node::AnyParseNode;
use crate::spacing_data::{SPACINGS, TIGHT_SPACINGS};
use crate::types::{CssProperty, ParseError, ParseErrorKind};
use crate::units::make_em;
use crate::utils::OwnedOrMut;
use crate::{KatexContext, build_common};
use alloc::collections::VecDeque;
use core::mem;
use core::ops::DerefMut as _;
use core::str::FromStr as _;
use phf::phf_set;
use strum::{AsRefStr, EnumString, IntoDiscriminant as _};

// Binary atoms (first class `mbin`) change into ordinary atoms (`mord`)
// depending on their surroundings. See TeXbook pg. 442-446, Rules 5 and 6,
// and the text before Rule 19.
const BIN_LEFT_CANCELLER: phf::Set<&str> =
    phf_set!("leftmost", "mbin", "mopen", "mrel", "mop", "mpunct");
const BIN_RIGHT_CANCELLER: phf::Set<&str> = phf_set!("rightmost", "mrel", "mclose", "mpunct");

/// DOM enum for atom classes
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum DomType {
    /// Ordinary atom (mord) - default class for most symbols
    Mord,
    /// Operator atom (mop) - for operators like sum, integral
    Mop,
    /// Binary operator atom (mbin) - for binary operators like +, -
    Mbin,
    /// Relation atom (mrel) - for relations like =, <, >
    Mrel,
    /// Opening delimiter atom (mopen) - for opening delimiters like (, [
    Mopen,
    /// Closing delimiter atom (mclose) - for closing delimiters like ), ]
    Mclose,
    /// Punctuation atom (mpunct) - for punctuation like comma, period
    Mpunct,
    /// Inner atom (minner) - for inner expressions
    Minner,
}

/// Enum for is_real_group parameter, matching JavaScript's boolean | "root"
///
/// This enum controls how expressions are treated during HTML building,
/// particularly regarding spacing calculations and binary operator
/// cancellation.
///
/// # Variants
/// * `False` - Partial group (e.g., created by `\color`, `\style`, or similar
///   commands). These groups don't affect spacing around them, allowing the
///   parent expression to handle spacing calculations. No binary operator
///   cancellation occurs.
/// * `True` - Real group with no atoms added on either side. Full spacing rules
///   and binary operator cancellation are applied.
/// * `Root` - Special case for root expressions. Treated as a real group but
///   may have additional special handling for top-level expressions.
///
/// # Usage Scenarios
/// - Use `False` for styling commands like `\color{red}{x}` where the color
///   span shouldn't interfere with surrounding spacing
/// - Use `True` for actual mathematical groupings like `{x + y}` where spacing
///   rules should be applied normally
/// - Use `Root` for the top-level expression in a mathematical formula
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupType {
    /// Partial group (e.g., created by \color) - allows parent to handle
    /// spacing
    False,
    /// Real group - no atoms added on either side, full spacing rules apply
    True,
    /// Root expression - special case for top-level expressions
    Root,
}

impl GroupType {
    /// Check if this represents a real group (true or root)
    #[must_use]
    pub const fn is_real(&self) -> bool {
        matches!(self, Self::True | Self::Root)
    }

    /// Check if this is the root case
    #[must_use]
    pub const fn is_root(&self) -> bool {
        matches!(self, Self::Root)
    }
}

impl DomType {
    /// Convert to string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mord => "mord",
            Self::Mop => "mop",
            Self::Mbin => "mbin",
            Self::Mrel => "mrel",
            Self::Mopen => "mopen",
            Self::Mclose => "mclose",
            Self::Mpunct => "mpunct",
            Self::Minner => "minner",
        }
    }
}

/// Side enum for getOutermostNode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Left side of the expression
    Left,
    /// Right side of the expression
    Right,
}

/// Create a null delimiter span
///
/// This function creates a special span element used as a placeholder for
/// missing delimiters. Null delimiters are invisible elements that maintain
/// proper spacing and alignment in mathematical expressions where delimiters
/// would normally be present but are omitted.
///
/// # Parameters
/// * `options` - Rendering options that affect sizing and styling
/// * `classes` - Additional CSS classes to apply to the null delimiter
///
/// # Returns
/// A `DomSpan` representing the null delimiter element with appropriate sizing
/// classes.
///
/// # Behavior
/// - Adds "nulldelimiter" class for CSS styling
/// - Includes base sizing classes from options for proper vertical alignment
/// - Creates an empty span that takes up space but has no visible content
/// - Used for maintaining layout consistency in cases like unmatched delimiters
#[must_use]
pub fn make_null_delimiter(options: &Options, classes: &[String]) -> DomSpan {
    let mut combined_classes =
        Vec::with_capacity(classes.len() + 1 + options.base_sizing_classes().len());
    combined_classes.extend_from_slice(classes);
    combined_classes.push(String::from("nulldelimiter"));
    combined_classes.extend(options.base_sizing_classes());
    make_span(combined_classes, vec![], None, None)
}

/// Check if given node is a partial group, i.e., does not affect spacing
/// around.
fn check_partial_group(node: &HtmlDomNode) -> Option<&Vec<HtmlDomNode>> {
    match node {
        HtmlDomNode::Fragment(fragment) => Some(&fragment.children),
        HtmlDomNode::Anchor(anchor) => Some(&anchor.children),
        HtmlDomNode::DomSpan(span) if span.classes.contains(&"enclosing".to_owned()) => {
            Some(&span.children)
        }
        _ => None,
    }
}

/// Check if given node is a partial group and get mutable children
fn check_partial_group_mut(node: &mut HtmlDomNode) -> Option<&mut Vec<HtmlDomNode>> {
    match node {
        HtmlDomNode::Fragment(fragment) => Some(&mut fragment.children),
        HtmlDomNode::Anchor(anchor) => Some(&mut anchor.children),
        HtmlDomNode::DomSpan(span) if span.classes.contains(&"enclosing".to_owned()) => {
            Some(&mut span.children)
        }
        _ => None,
    }
}

fn find_next_nonspace(nodes: &[HtmlDomNode], mut index: usize) -> Option<&HtmlDomNode> {
    while index < nodes.len() {
        if !nodes[index].has_class("mspace") {
            return Some(&nodes[index]);
        }
        index += 1;
    }
    None
}

fn find_last_nonspace_index(children: &[HtmlDomNode]) -> Option<usize> {
    children
        .iter()
        .rposition(|child| !child.has_class("mspace"))
}

fn fix_partial_group_spacing(
    ctx: &KatexContext,
    glue_options: &Options,
    nodes: &mut [HtmlDomNode],
) -> Result<(), ParseError> {
    let mut i = 0;
    while i < nodes.len() {
        let (left, right) = nodes.split_at_mut(i + 1);
        let current = &mut left[i];

        if let Some(children) = check_partial_group_mut(current) {
            fix_partial_group_spacing(ctx, glue_options, children)?;

            if let Some(last_idx) = find_last_nonspace_index(children) {
                let has_trailing_space = children
                    .get(last_idx + 1)
                    .is_some_and(|child| child.has_class("mspace"));
                if has_trailing_space {
                    i += 1;
                    continue;
                }

                let prev_type = get_type_of_dom_tree(&children[last_idx], Some(Side::Right));
                let next_node = find_next_nonspace(right, 0);
                if let (Some(prev_type), Some(next_node)) = (prev_type, next_node) {
                    let next_type = get_type_of_dom_tree(next_node, Some(Side::Left))
                        .or_else(|| get_type_of_dom_tree(next_node, None));
                    if let Some(next_type) = next_type {
                        let space = if next_node.has_class("mtight") {
                            TIGHT_SPACINGS
                                .get(prev_type.as_str())
                                .and_then(|inner| inner.get(next_type.as_str()))
                        } else {
                            SPACINGS
                                .get(prev_type.as_str())
                                .and_then(|inner| inner.get(next_type.as_str()))
                        };

                        if let Some(space) = space {
                            let glue = ctx.make_glue(space, glue_options)?;
                            children.insert(last_idx + 1, glue.into());
                        }
                    }
                }
            }
        }

        i += 1;
    }

    Ok(())
}

/// Return the outermost node of a domTree.
fn get_outermost_node(node: &HtmlDomNode, side: Side) -> &HtmlDomNode {
    if let Some(children) = check_partial_group(node)
        && !children.is_empty()
    {
        if side == Side::Right {
            return get_outermost_node(&children[children.len() - 1], Side::Right);
        }
        return get_outermost_node(&children[0], Side::Left);
    }
    node
}

/// Return math atom class (mclass) of a domTree
///
/// This function determines the mathematical atom type of a DOM node, which is
/// crucial for spacing calculations and binary operator cancellation in
/// mathematical expressions. The atom type determines how the element interacts
/// with surrounding elements.
///
/// # Parameters
/// * `node` - Reference to the HTML DOM node to analyze
/// * `side` - Optional side specification:
///   - `Some(Side::Left)`: Get the type of the leftmost outermost node
///   - `Some(Side::Right)`: Get the type of the rightmost outermost node
///   - `None`: Get the type of the node itself
///
/// # Returns
/// An `Option<DomType>` representing the mathematical atom classification:
/// - `Some(DomType::Mord)`: Ordinary symbol (default)
/// - `Some(DomType::Mop)`: Operator (e.g., sum, integral)
/// - `Some(DomType::Mbin)`: Binary operator (e.g., +, -)
/// - `Some(DomType::Mrel)`: Relation (e.g., =, <, >)
/// - `Some(DomType::Mopen)`: Opening delimiter (e.g., (, [)
/// - `Some(DomType::Mclose)`: Closing delimiter (e.g., ), ])
/// - `Some(DomType::Mpunct)`: Punctuation (e.g., comma, period)
/// - `Some(DomType::Minner)`: Inner expression
/// - `None`: Not a recognized math atom type
///
/// # Behavior
/// - Examines the first CSS class of DomSpan elements to determine type
/// - Handles partial groups by recursively examining outermost nodes
/// - Used extensively in spacing calculations and operator cancellation logic
#[must_use]
pub fn get_type_of_dom_tree(node: &HtmlDomNode, side: Option<Side>) -> Option<DomType> {
    let node = side.map_or(node, |side| get_outermost_node(node, side));
    let dom_type = DomType::from_str(node.classes().first()?);
    dom_type.ok()
}

/// Traverse non-space nodes, calling callback with current and previous node
fn traverse_non_space_nodes(
    ctx: &KatexContext,
    nodes: &mut Vec<HtmlDomNode>,
    callback: &mut impl FnMut(
        &KatexContext,
        &mut HtmlDomNode,
        &mut HtmlDomNode,
    ) -> Result<Option<HtmlDomNode>, ParseError>,
    prev_node: &mut HtmlDomNode,
    next_node: &mut Option<HtmlDomNode>,
    is_root: bool,
    insertions: Option<&mut VecDeque<HtmlDomNode>>,
) -> Result<(), ParseError> {
    let next_in_nodes = mem::take(next_node).is_some_and(|next| {
        nodes.push(next);
        true
    });

    let mut insertions = insertions.map_or_else(
        || OwnedOrMut::Owned {
            idx: 0,
            val: VecDeque::new(),
        },
        OwnedOrMut::Borrowed,
    );

    let mut prev_node = OwnedOrMut::Borrowed(prev_node);

    let mut i = 0;
    while i < nodes.len() {
        let node = &mut nodes[i];
        let partial_group = check_partial_group_mut(node);
        if let Some(children) = partial_group {
            traverse_non_space_nodes(
                ctx,
                children,
                callback,
                &mut prev_node,
                &mut None,
                is_root,
                Some(&mut *insertions),
            )?;
            i += 1;
            continue;
        }

        // Ignore explicit spaces (e.g., \;, \,) when determining what implicit
        // spacing should go between atoms of different classes
        let nonspace = !node.has_class("mspace");

        if nonspace {
            let result = callback(ctx, node, &mut prev_node)?;
            if let Some(new_node) = result {
                insertions.deref_mut().push_back(new_node);
            }
        }

        let to_be_prev = if nonspace {
            Some(OwnedOrMut::Owned {
                idx: i,
                val: node.clone(),
            })
        } else if is_root && node.has_class("newline") {
            // Treat like beginning of line
            Some(OwnedOrMut::Owned {
                idx: i,
                val: build_common::make_span(vec!["leftmost".to_owned()], vec![], None, None)
                    .into(),
            })
        } else {
            None
        };

        if let Some(to_be_prev) = to_be_prev {
            prev_node = to_be_prev;
        }

        if let OwnedOrMut::Owned { idx, val: arr } = &mut insertions {
            i += arr.len();
            nodes.splice((*idx + 1)..=(*idx), arr.drain(..));

            *idx = i;
        } else {
            insertions = OwnedOrMut::Owned {
                idx: i,
                val: VecDeque::new(),
            };
        }

        i += 1;
    }

    if let OwnedOrMut::Owned { idx, val: arr } = &mut insertions {
        nodes.splice((*idx + 1)..=(*idx), arr.drain(..));
    }

    if next_in_nodes {
        let next = nodes.pop();
        *next_node = next;
    }

    Ok(())
}

/// Take a list of nodes, build them in order, and return a list of the built
/// nodes.
///
/// documentFragments are flattened into their contents, so the returned list
/// contains no fragments. `is_real_group` indicates if `expression` is a real
/// group (no atoms will be added on either side), as opposed to a partial group
/// (e.g. one created by \color). `surrounding` is a tuple consisting of the
/// types of nodes that will be added to the left and right.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing group builders and other rendering
///   state
/// * `expression` - Slice of parse nodes to be converted to HTML DOM nodes
/// * `options` - Rendering options including style, size, and font settings
/// * `is_real_group` - Whether this expression represents a real group that
///   affects spacing:
///   - `GroupType::True`: Real group, atoms won't be added on either side
///   - `GroupType::False`: Partial group (e.g., created by \color), allows
///     parent to handle spacing
///   - `GroupType::Root`: Special case for root expressions, treated as real
///     group
/// * `surrounding` - Tuple of DOM types for left and right surrounding nodes,
///   used for:
///   - Binary operator cancellation (changing mbin to mord based on context)
///   - Spacing calculations between atoms of different classes
///   - Determining appropriate spacing rules from spacing tables
///
/// # Returns
/// A `Result` containing either a vector of HTML DOM nodes or a `ParseError` if
/// building fails.
///
/// # Behavior
/// - Flattens DocumentFragment children into the result list
/// - Combines consecutive symbol nodes for optimization
/// - Applies spacing rules if `is_real_group` is true
/// - Performs binary operator cancellation based on surrounding context
pub fn build_expression(
    ctx: &KatexContext,
    expression: &[AnyParseNode],
    options: &Options,
    is_real_group: GroupType,
    surrounding: (Option<DomType>, Option<DomType>),
) -> Result<Vec<HtmlDomNode>, ParseError> {
    // Parse expressions into groups
    let mut groups: Vec<HtmlDomNode> = Vec::new();

    for node in expression {
        let output = build_group(ctx, node, options, None)?;
        // Handle DocumentFragment flattening - match JS behavior
        if let HtmlDomNode::Fragment(fragment) = output {
            // Flatten DocumentFragment children into groups
            groups.extend(fragment.children);
        } else {
            groups.push(output);
        }
    }

    // Combine consecutive domTree.symbolNodes into a single symbolNode.
    try_combine_chars(&mut groups);

    // If `expression` is a partial group, let the parent handle spacings
    // to avoid processing groups multiple times.
    if !is_real_group.is_real() {
        return Ok(groups);
    }

    let glue_options = if expression.len() == 1 {
        if let AnyParseNode::Sizing(sizing) = &expression[0] {
            options.having_size(sizing.size)
        } else if let AnyParseNode::Styling(styling_node) = &expression[0] {
            // Apply styling to options using the appropriate style
            options.having_style(styling_node.style)
        } else {
            options.clone()
        }
    } else {
        options.clone()
    };

    // Dummy spans for determining spacings between surrounding atoms.
    // If `expression` has no atoms on the left or right, class "leftmost"
    // or "rightmost", respectively, is used to indicate it.
    let mut dummy_prev = make_span(
        vec![
            surrounding
                .0
                .as_ref()
                .map_or_else(|| "leftmost".to_owned(), |s| s.as_str().to_owned()),
        ],
        vec![],
        Some(options),
        None,
    )
    .into();
    let mut dummy_next = Some(
        make_span(
            vec![
                surrounding
                    .1
                    .as_ref()
                    .map_or_else(|| "rightmost".to_owned(), |s| s.as_str().to_owned()),
            ],
            vec![],
            Some(options),
            None,
        )
        .into(),
    );

    // Before determining what spaces to insert, perform bin cancellation.
    // Binary operators change to ordinary symbols in some contexts.
    let is_root = is_real_group.is_root();

    traverse_non_space_nodes(
        ctx,
        &mut groups,
        &mut |_ctx: &KatexContext, node: &mut HtmlDomNode, prev: &mut HtmlDomNode| {
            let Some(prev_type) = prev.classes().first() else {
                return Ok(None);
            };
            let Some(type_str) = node.classes().first() else {
                return Ok(None);
            };

            if prev_type == "mbin" && BIN_RIGHT_CANCELLER.contains(type_str) {
                // Change prev.classes[0] to "mord"
                if let Some(classes) = prev.classes_mut()
                    && !classes.is_empty()
                {
                    "mord".clone_into(&mut classes[0]);
                }
            } else if type_str == "mbin" && BIN_LEFT_CANCELLER.contains(prev_type) {
                // Change node.classes[0] to "mord"
                if let Some(classes) = node.classes_mut()
                    && !classes.is_empty()
                {
                    "mord".clone_into(&mut classes[0]);
                }
            }
            Ok(None)
        },
        &mut dummy_prev,
        &mut dummy_next,
        is_root,
        None,
    )?;

    // Insert spacings
    traverse_non_space_nodes(
        ctx,
        &mut groups,
        &mut |ctx: &KatexContext, node: &mut HtmlDomNode, prev: &mut HtmlDomNode| {
            let prev_type = get_type_of_dom_tree(prev, None);
            let type_opt = get_type_of_dom_tree(node, None);
            if let (Some(prev_type), Some(type_val)) = (prev_type, type_opt) {
                // 'mtight' indicates that the node is script or scriptscript style.
                let space = if node.has_class("mtight") {
                    TIGHT_SPACINGS
                        .get(prev_type.as_str())
                        .and_then(|inner| inner.get(type_val.as_str()))
                } else {
                    SPACINGS
                        .get(prev_type.as_str())
                        .and_then(|inner| inner.get(type_val.as_str()))
                };

                if let Some(space) = space {
                    // Insert glue (spacing) after the `prev`.
                    let glue = ctx.make_glue(space, &glue_options)?;
                    return Ok(Some(glue.into()));
                }
            }
            Ok(None)
        },
        &mut dummy_prev,
        &mut dummy_next,
        is_root,
        None,
    )?;

    fix_partial_group_spacing(ctx, &glue_options, &mut groups)?;

    Ok(groups)
}

/// Build a single parse node using the appropriate group builder
///
/// This function is the central dispatcher for converting individual parse
/// nodes into HTML DOM nodes. It determines the appropriate builder function
/// based on the node type and handles size/style interactions between parent
/// and child elements.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing registered group builders and
///   rendering state
/// * `group` - The parse node to be converted to HTML DOM
/// * `options` - Current rendering options (style, size, color, etc.)
/// * `base_options` - Optional base options for size calculations:
///   - Used to determine relative sizing when `options.size !=
///     base_options.size`
///   - If provided and sizes differ, applies scaling transformations to the
///     result
///   - `None` indicates no size adjustment is needed
///
/// # Returns
/// A `Result` containing either the built HTML DOM node or a `ParseError` if
/// building fails.
///
/// # Behavior
/// - Looks up the appropriate builder function in `ctx.html_group_builders`
///   based on node type
/// - Calls the registered builder with the provided options
/// - If `base_options` is provided and sizes differ:
///   - Wraps the result in a sizing span with appropriate CSS classes
///   - Scales height and depth by the size multiplier ratio
///   - Returns the wrapped node with adjusted dimensions
/// - Returns the built node directly if no size adjustment is needed
///
/// # Error Handling
/// Returns `ParseError` if:
/// - No builder is registered for the given node type
/// - The registered builder function fails
/// - Size calculation or DOM manipulation fails
pub fn build_group(
    ctx: &KatexContext,
    group: &AnyParseNode,
    options: &Options,
    base_options: Option<&Options>,
) -> Result<HtmlDomNode, ParseError> {
    // Get the group type string to lookup the builder
    let group_type = group.discriminant();

    // Try to find a group builder for this type
    let group_node = if let Some(builder) = ctx.html_group_builders.get(&group_type) {
        // Call the registered group builder
        builder(group, options, ctx)?
    } else {
        return Err(ParseError::new(ParseErrorKind::UnknownGroupType {
            group_type,
        }));
    };

    if let Some(base_options) = base_options
        && options.size != base_options.size
    {
        let mut group_node = make_span(
            options.sizing_classes(base_options),
            vec![group_node],
            Some(options),
            None,
        );
        let multiplier = options.size_multiplier / base_options.size_multiplier;
        group_node.height *= multiplier;
        group_node.depth *= multiplier;
        Ok(group_node.into())
    } else {
        Ok(group_node)
    }
}

/// Combine an array of HTML DOM nodes into an unbreakable HTML node of class
/// .base
fn build_html_unbreakable(children: Vec<HtmlDomNode>, options: &Options) -> HtmlDomNode {
    // Compute height and depth of this chunk.
    let mut body = make_span(vec!["base".to_owned()], children, Some(options), None);

    // Add strut, which ensures that the top of the HTML element falls at
    // the height of the expression, and the bottom of the HTML element
    // falls at the depth of the expression.
    let mut strut = make_span(vec!["strut".to_owned()], vec![], Some(options), None);

    strut
        .style
        .insert(CssProperty::Height, make_em(body.height + body.depth));
    if body.depth > 0.0 {
        strut
            .style
            .insert(CssProperty::VerticalAlign, make_em(-body.depth));
    }

    body.children.insert(0, strut.into());
    HtmlDomNode::DomSpan(body)
}

/// Build an entire parse tree into HTML DOM nodes
///
/// This is the main entry point for converting a complete mathematical
/// expression parse tree into HTML DOM representation. It handles the top-level
/// structure, line breaking, tagging, and final DOM assembly.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing all necessary builders and rendering
///   state
/// * `tree` - The complete parse tree to be converted to HTML
/// * `options` - Rendering options including style, size, color, and display
///   settings
///
/// # Returns
/// A `Result` containing either the root HTML DOM node or a `ParseError` if
/// building fails.
///
/// # Behavior
/// - Strips outer tag wrappers if present (e.g., from `\tag{}` commands)
/// - Builds the main expression using `build_expression` with root group
///   settings
/// - Handles equation numbering by extracting tag elements
/// - Implements line breaking logic based on TeX rules:
///   - Breaks after binary operators, relations, and `\allowbreak` commands
///   - Respects `\nobreak` commands to prevent unwanted breaks
///   - Creates unbreakable chunks between potential break points
/// - Processes tags and adjusts strut heights for proper vertical alignment
/// - Adds accessibility attributes (`aria-hidden="true"`)
/// - Wraps everything in a `katex-html` container span
///
/// # Line Breaking Rules
/// Follows TeXBook p.173 guidelines:
/// - Breaks after binary operators (+, -, ×, ÷) and relations (=, <, >, ≠)
/// - Only breaks at "outer level" (not inside {...} or \over constructs)
/// - Respects explicit break control commands
///
/// # Error Handling
/// Returns `ParseError` if:
/// - Expression building fails
/// - Tag processing encounters invalid structures
/// - DOM manipulation fails
pub fn build_html(
    ctx: &KatexContext,
    tree: &[AnyParseNode],
    options: &Options,
) -> Result<HtmlDomNode, ParseError> {
    // Strip off outer tag wrapper for processing below.
    let mut tag = None;
    let mut tree = tree;

    if tree.len() == 1
        && let AnyParseNode::Tag(tag_node) = &tree[0]
    {
        tag = Some(&tag_node.tag);
        tree = &tag_node.body;
    }

    // Build the expression contained in the tree
    let mut expression = build_expression(ctx, tree, options, GroupType::Root, (None, None))?;

    let eqn_num = if expression.len() == 2
        && let Some(second) = expression.get(1)
        && second.has_class("tag")
    {
        // An environment with automatic equation numbers, e.g. {gather}.
        expression.pop()
    } else {
        None
    };

    let mut children = Vec::new();

    // Create one base node for each chunk between potential line breaks.
    // The TeXBook [p.173] says "A formula will be broken only after a
    // relation symbol like $=$ or $<$ or $\rightarrow$, or after a binary
    // operation symbol like $+$ or $-$ or $\times$, where the relation or
    // binary operation is on the ``outer level'' of the formula (i.e., not
    // enclosed in {...} and not part of an \over construction)."
    let mut parts = Vec::new();
    let mut i = 0;
    while i < expression.len() {
        parts.push(expression[i].clone());
        if expression[i].has_class("mbin")
            || expression[i].has_class("mrel")
            || expression[i].has_class("allowbreak")
        {
            // Put any post-operator glue on same line as operator.
            // Watch for \nobreak along the way, and stop at \newline.
            let mut nobreak = false;
            while i < expression.len() - 1
                && expression[i + 1].has_class("mspace")
                && !expression[i + 1].has_class("newline")
            {
                i += 1;
                parts.push(expression[i].clone());
                if expression[i].has_class("nobreak") {
                    nobreak = true;
                }
            }
            // Don't allow break if \nobreak among the post-operator glue.
            if !nobreak {
                children.push(build_html_unbreakable(parts, options));
                parts = Vec::new();
            }
        } else if expression[i].has_class("newline") {
            // Write the line except the newline
            parts.pop();
            if !parts.is_empty() {
                children.push(build_html_unbreakable(parts, options));
                parts = Vec::new();
            }
            // Put the newline at the top level
            children.push(expression[i].clone());
        }
        i += 1;
    }

    if !parts.is_empty() {
        children.push(build_html_unbreakable(parts, options));
    }

    // Now, if there was a tag, build it too and append it as a final child.
    let tag_child_index = if let Some(tag_ref) = tag {
        let tag_html = build_expression(ctx, tag_ref, options, GroupType::True, (None, None))?;
        let mut unbreakable = build_html_unbreakable(tag_html, options);
        if let HtmlDomNode::DomSpan(span) = &mut unbreakable {
            span.classes = vec!["tag".to_owned()];
        }
        children.push(unbreakable);
        Some(children.len() - 1)
    } else {
        if let Some(eqn_num) = eqn_num {
            children.push(eqn_num);
        }
        None
    };

    let mut span = make_span(vec!["katex-html".to_owned()], children, Some(options), None);
    span.attributes
        .insert("aria-hidden".to_owned(), "true".to_owned());

    // Adjust the strut of the tag to be the maximum height of all children
    // (the height of the enclosing htmlNode) for proper vertical alignment.
    if let Some(index) = tag_child_index
        && let Some(tag_child) = span.children.get_mut(index)
        && let HtmlDomNode::DomSpan(tag_span) = tag_child
        && let Some(strut_node) = tag_span.children.first_mut()
        && let HtmlDomNode::DomSpan(strut_span) = strut_node
    {
        // Calculate the total height (height + depth of htmlNode)
        let total_height = span.height + span.depth;

        // Update strut height
        if total_height > 0.0 {
            strut_span
                .style
                .insert(CssProperty::Height, make_em(total_height));
        }

        // Update strut vertical alignment if depth > 0
        if span.depth > 0.0 {
            strut_span
                .style
                .insert(CssProperty::VerticalAlign, make_em(-span.depth));
        }
    }

    Ok(span.into())
}
