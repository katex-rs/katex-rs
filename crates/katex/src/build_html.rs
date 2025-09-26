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
use crate::{KatexContext, build_common};
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
pub fn make_null_delimiter(options: &Options, classes: Vec<String>) -> DomSpan {
    let mut combined_classes = classes;
    combined_classes.reserve(1 + options.base_sizing_classes().len());
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

fn container_ref_by_path<'a>(
    nodes: &'a Vec<HtmlDomNode>,
    path: &[usize],
) -> Option<&'a Vec<HtmlDomNode>> {
    if path.is_empty() {
        return Some(nodes);
    }

    let (last, rest) = path.split_last()?;
    let parent = container_ref_by_path(nodes, rest)?;
    let node = parent.get(*last)?;
    check_partial_group(node)
}

fn container_mut_by_path<'a>(
    nodes: &'a mut Vec<HtmlDomNode>,
    path: &[usize],
) -> Option<&'a mut Vec<HtmlDomNode>> {
    if path.is_empty() {
        return Some(nodes);
    }

    let (last, rest) = path.split_last()?;
    let parent = container_mut_by_path(nodes, rest)?;
    let node = parent.get_mut(*last)?;
    check_partial_group_mut(node)
}

fn container_len(nodes: &Vec<HtmlDomNode>, path: &[usize]) -> Option<usize> {
    container_ref_by_path(nodes, path).map(Vec::len)
}

fn node_ref_by_path<'a>(nodes: &'a Vec<HtmlDomNode>, path: &[usize]) -> Option<&'a HtmlDomNode> {
    let (last, rest) = path.split_last()?;
    let container = container_ref_by_path(nodes, rest)?;
    container.get(*last)
}

fn node_mut_by_path<'a>(
    nodes: &'a mut Vec<HtmlDomNode>,
    path: &[usize],
) -> Option<&'a mut HtmlDomNode> {
    let (last, rest) = path.split_last()?;
    let container = container_mut_by_path(nodes, rest)?;
    container.get_mut(*last)
}

fn node_mut_in_slice<'a>(
    slice: &'a mut [HtmlDomNode],
    path: &[usize],
) -> Option<&'a mut HtmlDomNode> {
    let (first, rest) = path.split_first()?;
    if rest.is_empty() {
        slice.get_mut(*first)
    } else {
        let children = check_partial_group_mut(slice.get_mut(*first)?)?;
        node_mut_by_path(children, rest)
    }
}

fn two_nodes_mut_in_container<'a>(
    container: &'a mut [HtmlDomNode],
    left_path: &[usize],
    right_path: &[usize],
) -> Option<(&'a mut HtmlDomNode, &'a mut HtmlDomNode)> {
    debug_assert!(!left_path.is_empty() && !right_path.is_empty());

    if left_path[0] == right_path[0] {
        let children = check_partial_group_mut(container.get_mut(left_path[0])?)?;
        return two_nodes_mut_by_path(children, &left_path[1..], &right_path[1..]);
    }

    if left_path[0] < right_path[0] {
        let split_at = right_path[0];
        if split_at > container.len() {
            return None;
        }
        let (left_slice, right_slice) = container.split_at_mut(split_at);
        let left_node = node_mut_in_slice(left_slice, left_path)?;
        let mut adjusted = right_path.to_vec();
        adjusted[0] = 0;
        let right_node = node_mut_in_slice(right_slice, &adjusted)?;
        Some((left_node, right_node))
    } else {
        let split_at = left_path[0];
        if split_at > container.len() {
            return None;
        }
        let (left_slice, right_slice) = container.split_at_mut(split_at);
        let mut adjusted = left_path.to_vec();
        adjusted[0] = 0;
        let right_node = node_mut_in_slice(right_slice, &adjusted)?;
        let left_node = node_mut_in_slice(left_slice, right_path)?;
        Some((right_node, left_node))
    }
}

fn two_nodes_mut_by_path<'a>(
    nodes: &'a mut Vec<HtmlDomNode>,
    left: &[usize],
    right: &[usize],
) -> Option<(&'a mut HtmlDomNode, &'a mut HtmlDomNode)> {
    debug_assert_ne!(left, right, "paths must reference distinct nodes");

    let prefix_len = left
        .iter()
        .zip(right.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let container_path = &left[..prefix_len];
    let container = container_mut_by_path(nodes, container_path)?;
    two_nodes_mut_in_container(container, &left[prefix_len..], &right[prefix_len..])
}

fn insert_into_container(
    nodes: &mut Vec<HtmlDomNode>,
    parent_path: &[usize],
    index: usize,
    node: HtmlDomNode,
) -> (Vec<usize>, usize) {
    if let Some(container) = container_mut_by_path(nodes, parent_path) {
        let insert_index = index.min(container.len());
        container.insert(insert_index, node);
        (parent_path.to_vec(), insert_index)
    } else {
        let insert_index = index.min(nodes.len());
        nodes.insert(insert_index, node);
        (Vec::new(), insert_index)
    }
}

fn seek_last_nonspace(nodes: &Vec<HtmlDomNode>, path: &[usize]) -> Option<NodePath> {
    let node = node_ref_by_path(nodes, path)?;
    if let Some(children) = check_partial_group(node) {
        for idx in (0..children.len()).rev() {
            let mut child_path = path.to_vec();
            child_path.push(idx);
            if let Some(result) = seek_last_nonspace(nodes, &child_path) {
                return Some(result);
            }
        }
        None
    } else if node.has_class("mspace") {
        None
    } else {
        Some(NodePath::new(path.to_vec()))
    }
}

fn find_prev_nonspace_path(nodes: &Vec<HtmlDomNode>, path: &[usize]) -> Option<NodePath> {
    if path.is_empty() {
        return None;
    }
    let (last, rest) = path.split_last()?;
    for idx in (0..*last).rev() {
        let mut candidate = rest.to_vec();
        candidate.push(idx);
        if let Some(result) = seek_last_nonspace(nodes, &candidate) {
            return Some(result);
        }
    }
    find_prev_nonspace_path(nodes, rest)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NodePath(Vec<usize>);

impl NodePath {
    const fn new(path: Vec<usize>) -> Self {
        Self(path)
    }

    #[inline]
    fn as_slice(&self) -> &[usize] {
        &self.0
    }

    fn adjust_for_insert(&mut self, parent_path: &[usize], index: usize) {
        if self.0.len() < parent_path.len() + 1 {
            return;
        }
        if &self.0[..parent_path.len()] == parent_path {
            let slot = parent_path.len();
            if self.0[slot] >= index {
                self.0[slot] += 1;
            }
        }
    }
}

enum PrevNodeState {
    Dummy,
    Owned(Box<HtmlDomNode>),
    Located(NodePath),
}

struct PrevTracker {
    state: PrevNodeState,
}

impl PrevTracker {
    const fn new() -> Self {
        Self {
            state: PrevNodeState::Dummy,
        }
    }

    fn with_prev_and_current(
        &mut self,
        root: &mut Vec<HtmlDomNode>,
        dummy_prev: &mut HtmlDomNode,
        current_path: &[usize],
        mut f: impl FnMut(&mut HtmlDomNode, &mut HtmlDomNode) -> Result<Option<HtmlDomNode>, ParseError>,
    ) -> Result<Option<HtmlDomNode>, ParseError> {
        match &mut self.state {
            PrevNodeState::Dummy => {
                let Some(current) = node_mut_by_path(root, current_path) else {
                    return Ok(None);
                };
                f(current, dummy_prev)
            }
            PrevNodeState::Owned(node) => {
                let Some(current) = node_mut_by_path(root, current_path) else {
                    return Ok(None);
                };
                f(current, node)
            }
            PrevNodeState::Located(_) => {
                let root_ref = &*root;
                if let Some(prev_path) = find_prev_nonspace_path(root_ref, current_path) {
                    let prev_slice = prev_path.as_slice();
                    let nodes = two_nodes_mut_by_path(root, current_path, prev_slice);
                    self.state = PrevNodeState::Located(prev_path);
                    if let Some((current, prev)) = nodes {
                        f(current, prev)
                    } else {
                        Ok(None)
                    }
                } else {
                    self.state = PrevNodeState::Dummy;
                    let Some(current) = node_mut_by_path(root, current_path) else {
                        return Ok(None);
                    };
                    f(current, dummy_prev)
                }
            }
        }
    }

    fn set_located(&mut self, path: NodePath) {
        self.state = PrevNodeState::Located(path);
    }

    fn set_owned_dummy(&mut self, node: HtmlDomNode) {
        self.state = PrevNodeState::Owned(Box::new(node));
    }

    fn prev_path(&self) -> Option<&[usize]> {
        match &self.state {
            PrevNodeState::Located(path) => Some(path.as_slice()),
            _ => None,
        }
    }

    fn adjust_for_insert(&mut self, parent_path: &[usize], index: usize) {
        if let PrevNodeState::Located(path) = &mut self.state {
            path.adjust_for_insert(parent_path, index);
        }
    }
}

/// Traverse non-space nodes, calling callback with current and previous node
fn traverse_non_space_nodes(
    ctx: &KatexContext,
    root: &mut Vec<HtmlDomNode>,
    callback: &mut impl FnMut(
        &KatexContext,
        &mut HtmlDomNode,
        &mut HtmlDomNode,
    ) -> Result<Option<HtmlDomNode>, ParseError>,
    prev: &mut PrevTracker,
    next: &mut Option<HtmlDomNode>,
    dummy_prev: &mut HtmlDomNode,
    is_root: bool,
) -> Result<(), ParseError> {
    #[derive(Clone, Default)]
    struct Frame {
        path: Vec<usize>,
        index: usize,
    }

    fn adjust_frame_paths(frames: &mut [Frame], parent_path: &[usize], index: usize) {
        for frame in frames.iter_mut() {
            if frame.path == parent_path {
                if frame.index >= index {
                    frame.index += 1;
                }
                continue;
            }
            if frame.path.len() < parent_path.len() + 1 {
                continue;
            }
            if frame.path[..parent_path.len()] == parent_path[..] {
                let slot = parent_path.len();
                if frame.path[slot] >= index {
                    frame.path[slot] += 1;
                }
            }
        }
    }

    let mut frames = vec![Frame::default()];

    let appended_next = next.take().is_some_and(|next_node| {
        root.push(next_node);
        true
    });

    while let Some(frame_index) = frames.len().checked_sub(1) {
        let Some(len) = container_len(root, &frames[frame_index].path) else {
            frames.pop();
            if frames.is_empty() {
                break;
            }
            continue;
        };
        if frames[frame_index].index >= len {
            frames.pop();
            if frames.is_empty() {
                break;
            }
            continue;
        }

        let current_index = frames[frame_index].index;
        let mut current_path = frames[frame_index].path.clone();
        current_path.push(current_index);
        let container_path_len = current_path.len().saturating_sub(1);

        let is_partial_group = {
            let Some(node_ref) = node_ref_by_path(root, &current_path) else {
                frames[frame_index].index = current_index + 1;
                continue;
            };
            check_partial_group(node_ref).is_some()
        };

        if is_partial_group {
            frames[frame_index].index = current_index + 1;
            frames.push(Frame {
                path: current_path,
                index: 0,
            });
            continue;
        }

        let nonspace = {
            let Some(node_ref) = node_ref_by_path(root, &current_path) else {
                frames[frame_index].index = current_index + 1;
                continue;
            };
            !node_ref.has_class("mspace")
        };

        let mut skip_inserted = 0usize;

        if nonspace {
            let result =
                prev.with_prev_and_current(root, dummy_prev, &current_path, |node, prev_node| {
                    callback(ctx, node, prev_node)
                })?;

            if let Some(new_node) = result {
                if let Some(prev_path) = prev.prev_path() {
                    if let Some((last, parent)) = prev_path.split_last() {
                        let inserted_pos = *last + 1;
                        let container_path_slice = &current_path[..container_path_len];
                        let (actual_path, actual_index) =
                            insert_into_container(root, parent, inserted_pos, new_node);
                        let same_container = actual_path.as_slice() == container_path_slice;
                        adjust_frame_paths(&mut frames, actual_path.as_slice(), actual_index);
                        prev.adjust_for_insert(actual_path.as_slice(), actual_index);
                        if same_container && actual_index <= current_index {
                            skip_inserted += 1;
                        }
                    } else {
                        let container_path_slice = &current_path[..container_path_len];
                        let (actual_path, actual_index) =
                            insert_into_container(root, container_path_slice, 0, new_node);
                        let same_container = actual_path.as_slice() == container_path_slice;
                        adjust_frame_paths(&mut frames, actual_path.as_slice(), actual_index);
                        prev.adjust_for_insert(actual_path.as_slice(), actual_index);
                        if same_container && actual_index <= current_index {
                            skip_inserted += 1;
                        }
                    }
                } else {
                    let container_path_slice = &current_path[..container_path_len];
                    let (actual_path, actual_index) =
                        insert_into_container(root, container_path_slice, 0, new_node);
                    let same_container = actual_path.as_slice() == container_path_slice;
                    adjust_frame_paths(&mut frames, actual_path.as_slice(), actual_index);
                    prev.adjust_for_insert(actual_path.as_slice(), actual_index);
                    if same_container && actual_index <= current_index {
                        skip_inserted += 1;
                    }
                }
            }
        }

        if skip_inserted > 0
            && let Some(last) = current_path.last_mut()
        {
            *last += skip_inserted;
        }

        if !nonspace && is_root {
            let is_newline = node_ref_by_path(root, &current_path)
                .is_some_and(|node_ref| node_ref.has_class("newline"));
            if is_newline {
                prev.set_owned_dummy(
                    build_common::make_span(vec!["leftmost".to_owned()], vec![], None, None).into(),
                );
            }
        }

        if nonspace {
            prev.set_located(NodePath::new(current_path));
        }

        frames[frame_index].index = current_index + 1 + skip_inserted;
    }

    if appended_next && let Some(next_node) = root.pop() {
        *next = Some(next_node);
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

    let mut prev_tracker = PrevTracker::new();
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
        &mut prev_tracker,
        &mut dummy_next,
        &mut dummy_prev,
        is_root,
    )?;

    // Insert spacings
    let mut prev_tracker = PrevTracker::new();
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
        &mut prev_tracker,
        &mut dummy_next,
        &mut dummy_prev,
        is_root,
    )?;

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
    let mut iter = expression.into_iter().peekable();
    while let Some(node) = iter.next() {
        let is_break_candidate =
            node.has_class("mbin") || node.has_class("mrel") || node.has_class("allowbreak");
        let is_newline = node.has_class("newline");

        parts.push(node);

        if is_break_candidate {
            // Put any post-operator glue on same line as operator.
            // Watch for \nobreak along the way, and stop at \newline.
            let mut nobreak = false;
            while let Some(next) =
                iter.next_if(|n| n.has_class("mspace") && !n.has_class("newline"))
            {
                if next.has_class("nobreak") {
                    nobreak = true;
                }
                parts.push(next);
            }
            // Don't allow break if \nobreak among the post-operator glue.
            if !nobreak {
                let mut chunk = Vec::with_capacity(parts.len());
                chunk.append(&mut parts);
                children.push(build_html_unbreakable(chunk, options));
            }
        } else if is_newline {
            // Write the line except the newline
            let newline = parts
                .pop()
                .ok_or_else(|| ParseError::new(ParseErrorKind::NewlineNodeNotFound))?;
            if !parts.is_empty() {
                let mut chunk = Vec::with_capacity(parts.len());
                chunk.append(&mut parts);
                children.push(build_html_unbreakable(chunk, options));
            }
            // Put the newline at the top level
            children.push(newline);
        }
    }

    if !parts.is_empty() {
        let mut chunk = Vec::with_capacity(parts.len());
        chunk.append(&mut parts);
        children.push(build_html_unbreakable(chunk, options));
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
