//! MathML tree node definitions for MathML rendering
//!
//! These objects store data about MathML nodes. This is the MathML equivalent
//! of the types in dom_tree.rs. Since MathML handles its own rendering, and
//! since we're mainly using MathML to improve accessibility, we don't manage
//! any of the styling state that the plain DOM nodes do.

use crate::ParseError;
#[cfg(feature = "wasm")]
use crate::dom_tree::create_class;
use crate::tree::{DocumentFragment, VirtualNode};
use crate::units::make_em;
use crate::utils::escape_into;
#[cfg(feature = "wasm")]
use crate::web_context::WebContext;
use crate::{namespace::KeyMap, types::CssStyle};
use bon::bon;
use core::fmt::{self, Debug, Write as _};
use strum::AsRefStr;
#[cfg(feature = "wasm")]
use wasm_bindgen::JsCast as _;
#[cfg(feature = "wasm")]
use web_sys;

fn map_fmt(result: fmt::Result) -> Result<(), ParseError> {
    result.map_err(ParseError::from)
}

/// MathML node types used in KaTeX
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum MathNodeType {
    /// `<math>` element
    Math,
    /// `<annotation>` element
    Annotation,
    /// `<semantics>` element
    Semantics,
    /// `<mtext>` element
    Mtext,
    /// `<mn>` element
    Mn,
    /// `<mo>` element
    Mo,
    /// `<mi>` element
    Mi,
    /// `<mspace>` element
    Mspace,
    /// `<mover>` element
    Mover,
    /// `<munder>` element
    Munder,
    /// `<munderover>` element
    Munderover,
    /// `<msup>` element
    Msup,
    /// `<msub>` element
    Msub,
    /// `<msubsup>` element
    Msubsup,
    /// `<mfrac>` element
    Mfrac,
    /// `<mroot>` element
    Mroot,
    /// `<msqrt>` element
    Msqrt,
    /// `<mtable>` element
    Mtable,
    /// `<mtr>` element
    Mtr,
    /// `<mtd>` element
    Mtd,
    /// `<mlabeledtr>` element
    Mlabeledtr,
    /// `<mrow>` element
    Mrow,
    /// `<menclose>` element
    Menclose,
    /// `<mstyle>` element
    Mstyle,
    /// `<mpadded>` element
    Mpadded,
    /// `<mphantom>` element
    Mphantom,
    /// `<mglyph>` element
    Mglyph,
}

/// Get the appropriate space character based on width
///
/// See <https://www.w3.org/TR/2000/WD-MathML2-20000328/chapter6.html>
/// for a table of space-like characters. We use Unicode representations
/// instead of &LongNames; as it's not clear how to make the latter via
/// document.createTextNode.
#[must_use]
pub fn get_space_character(width: f64) -> Option<String> {
    // Positive spaces
    if (0.05555..=0.05556).contains(&width) {
        Some("\u{200a}".to_owned()) // &VeryThinSpace;
    } else if (0.1666..=0.1667).contains(&width) {
        Some("\u{2009}".to_owned()) // &ThinSpace;
    } else if (0.2222..=0.2223).contains(&width) {
        Some("\u{2005}".to_owned()) // &MediumSpace;
    } else if (0.2777..=0.2778).contains(&width) {
        Some("\u{2005}\u{200a}".to_owned()) // &ThickSpace;
    }
    // Negative spaces - these include INVISIBLE SEPARATOR (U+2063) for proper negative spacing
    else if (-0.05556..=-0.05555).contains(&width) {
        Some("\u{200a}\u{2063}".to_owned()) // &NegativeVeryThinSpace;
    } else if (-0.1667..=-0.1666).contains(&width) {
        Some("\u{2009}\u{2063}".to_owned()) // &NegativeThinSpace;
    } else if (-0.2223..=-0.2222).contains(&width) {
        Some("\u{205f}\u{2063}".to_owned()) // &NegativeMediumSpace;
    } else if (-0.2778..=-0.2777).contains(&width) {
        Some("\u{2005}\u{2063}".to_owned()) // &NegativeThickSpace;
    } else {
        None
    }
}

// Clone implementation removed as we now use MathDomNodeEnum directly

/// MathML DOM node enum for type-safe node representation
#[derive(Clone)]
pub enum MathDomNode {
    /// MathML element node
    Math(MathNode),
    /// Text content node
    Text(TextNode),
    /// Space node
    Space(SpaceNode),
    /// Document fragment node
    Fragment(Box<MathDomFragment>),
}

/// Document fragment containing MathML DOM nodes
pub type MathDomFragment = DocumentFragment<MathDomNode>;

/// Make a MathDom Fragment
#[must_use]
pub fn make_fragment(children: Vec<MathDomNode>) -> MathDomFragment {
    MathDomFragment {
        children,
        classes: vec![],
        depth: 0.0,
        height: 0.0,
        max_font_size: 0.0,
        style: CssStyle::default(),
    }
}

/// General purpose MathML node of any type
#[derive(Clone)]
pub struct MathNode {
    /// The type of MathML node
    pub node_type: MathNodeType,
    /// Attributes of the MathML node
    pub attributes: KeyMap<String, String>,
    /// Child nodes of the MathML node
    pub children: Vec<MathDomNode>,
    /// CSS classes applied to the MathML node
    pub classes: Vec<String>,
}

impl Debug for MathNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MathNode")
            .field("node_type", &self.node_type)
            .field("attributes", &self.attributes)
            .field(
                "children",
                &format_args!("{} children", self.children.len()),
            )
            .field("classes", &self.classes)
            .finish()
    }
}

#[bon]
impl MathNode {
    /// Create a new MathNode with the given type
    #[builder]
    pub fn new(
        /// Node type
        node_type: MathNodeType,
        /// Node attributes
        attributes: Option<KeyMap<String, String>>,
        /// Child nodes
        children: Option<Vec<MathDomNode>>,
        /// CSS classes
        classes: Option<Vec<String>>,
    ) -> Self {
        Self {
            node_type,
            attributes: attributes.unwrap_or_default(),
            children: children.unwrap_or_default(),
            classes: classes.unwrap_or_default(),
        }
    }

    /// Create a new MathNode with the given type and children
    #[must_use]
    pub fn with_children(node_type: MathNodeType, children: Vec<MathDomNode>) -> Self {
        Self {
            node_type,
            attributes: KeyMap::default(),
            children,
            classes: Vec::new(),
        }
    }

    /// Add a child to this node
    pub fn add_child(&mut self, child: MathDomNode) {
        self.children.push(child);
    }

    /// Set an attribute on this node
    pub fn set_attribute<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.attributes.insert(key.into(), value.into());
    }

    /// Add a CSS class to this node
    pub fn add_class(&mut self, class: String) {
        self.classes.push(class);
    }

    fn to_text(&self) -> String {
        self.children.iter().map(MathDomNode::to_text).collect()
    }
}

impl VirtualNode for MathNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(write!(fmt, "<{}", self.node_type.as_ref()))?;

        if !self.classes.is_empty() {
            map_fmt(fmt.write_str(" class=\""))?;
            let mut first = true;
            for class in &self.classes {
                if !first {
                    map_fmt(fmt.write_char(' '))?;
                }
                first = false;
                map_fmt(escape_into(fmt, class))?;
            }
            map_fmt(fmt.write_char('"'))?;
        }

        for (key, value) in &self.attributes {
            map_fmt(write!(fmt, " {key}=\""))?;
            map_fmt(escape_into(fmt, value))?;
            map_fmt(fmt.write_char('"'))?;
        }

        map_fmt(fmt.write_char('>'))?;

        for child in &self.children {
            child.write_markup(fmt)?;
        }

        map_fmt(write!(fmt, "</{}>", self.node_type.as_ref()))?;
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx
            .document
            .create_element_ns(
                Some("http://www.w3.org/1998/Math/MathML"),
                self.node_type.as_ref(),
            )
            .unwrap();

        // Set attributes
        for (key, value) in &self.attributes {
            element.set_attribute(key, value).unwrap();
        }

        // Set classes
        if !self.classes.is_empty() {
            let class_str = create_class(&self.classes);
            element.set_attribute("class", &class_str).unwrap();
        }

        // Add children, combining consecutive TextNodes
        let mut i = 0;
        while i < self.children.len() {
            if let MathDomNode::Text(text_node) = &self.children[i] {
                // Check if next child is also a TextNode
                if i + 1 < self.children.len()
                    && let MathDomNode::Text(next_text_node) = &self.children[i + 1]
                {
                    // Combine them
                    let combined_text = format!("{}{}", text_node.text, next_text_node.text);
                    let text_node = ctx.document.create_text_node(&combined_text);
                    element.append_child(&text_node).unwrap();
                    i += 2; // Skip the next one
                    continue;
                }
            }
            // Normal case
            let child_node = self.children[i].to_node(ctx);
            element.append_child(&child_node).unwrap();
            i += 1;
        }

        element.dyn_into::<web_sys::Node>().unwrap()
    }
}

/// Text node for MathML content
#[derive(Debug, Clone)]
pub struct TextNode {
    /// The text content of the node
    pub text: String,
}

impl TextNode {
    fn to_text(&self) -> String {
        self.text.clone()
    }
}

impl VirtualNode for TextNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(escape_into(fmt, &self.text))?;
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        ctx.document
            .create_text_node(&self.text)
            .dyn_into::<web_sys::Node>()
            .unwrap()
    }
}

/// Space node for MathML, may render as `<mspace>` or as text
#[derive(Debug, Clone)]
pub struct SpaceNode {
    /// The width of the space in em units
    pub width: f64,
    /// Optional character to represent the space, defaults to a regular space
    pub character: Option<String>,
}

impl SpaceNode {
    /// Create a new SpaceNode with the given width
    #[must_use]
    pub fn new(width: f64) -> Self {
        let character = get_space_character(width);
        Self { width, character }
    }

    fn to_text(&self) -> String {
        self.character.clone().unwrap_or_else(|| " ".to_owned())
    }
}

impl VirtualNode for SpaceNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        if let Some(character) = &self.character {
            map_fmt(fmt.write_str("<mtext>"))?;
            map_fmt(escape_into(fmt, character))?;
            map_fmt(fmt.write_str("</mtext>"))?;
        } else {
            let width = make_em(self.width);
            map_fmt(fmt.write_str("<mspace width=\""))?;
            map_fmt(fmt.write_str(&width))?;
            map_fmt(fmt.write_str("\"/>"))?;
        }
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        self.character.as_ref().map_or_else(
            || {
                let element = ctx
                    .document
                    .create_element_ns(Some("http://www.w3.org/1998/Math/MathML"), "mspace")
                    .unwrap();
                element
                    .set_attribute("width", &make_em(self.width))
                    .unwrap();
                element.dyn_into::<web_sys::Node>().unwrap()
            },
            |character| {
                ctx.document
                    .create_text_node(character)
                    .dyn_into::<web_sys::Node>()
                    .unwrap()
            },
        )
    }
}

impl MathDomNode {
    /// Convert the MathDomNode to plain text
    pub fn to_text(&self) -> String {
        match self {
            Self::Math(node) => node.to_text(),
            Self::Text(node) => node.to_text(),
            Self::Space(node) => node.to_text(),
            Self::Fragment(fragment) => fragment.children.iter().map(Self::to_text).collect(),
        }
    }
}

impl VirtualNode for MathDomNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        match self {
            Self::Math(node) => node.write_markup(fmt),
            Self::Text(node) => node.write_markup(fmt),
            Self::Space(node) => node.write_markup(fmt),
            Self::Fragment(fragment) => fragment.write_markup(fmt),
        }
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        match self {
            Self::Math(node) => node.to_node(ctx),
            Self::Text(node) => node.to_node(ctx),
            Self::Space(node) => node.to_node(ctx),
            Self::Fragment(fragment) => fragment.to_node(ctx),
        }
    }
}

impl MathDomNode {
    /// Type-safe access to MathNode variant
    #[must_use]
    pub const fn as_math_node(&self) -> Option<&MathNode> {
        match self {
            Self::Math(node) => Some(node),
            _ => None,
        }
    }

    /// Type-safe access to TextNode variant
    #[must_use]
    pub const fn as_text_node(&self) -> Option<&TextNode> {
        match self {
            Self::Text(node) => Some(node),
            _ => None,
        }
    }

    /// Type-safe access to SpaceNode variant
    #[must_use]
    pub const fn as_space_node(&self) -> Option<&SpaceNode> {
        match self {
            Self::Space(node) => Some(node),
            _ => None,
        }
    }

    /// Mutable type-safe access to MathNode variant
    pub const fn as_math_node_mut(&mut self) -> Option<&mut MathNode> {
        match self {
            Self::Math(node) => Some(node),
            _ => None,
        }
    }

    /// Mutable type-safe access to TextNode variant
    pub const fn as_text_node_mut(&mut self) -> Option<&mut TextNode> {
        match self {
            Self::Text(node) => Some(node),
            _ => None,
        }
    }

    /// Mutable type-safe access to SpaceNode variant
    pub const fn as_space_node_mut(&mut self) -> Option<&mut SpaceNode> {
        match self {
            Self::Space(node) => Some(node),
            _ => None,
        }
    }

    /// Type-safe access to Fragment variant
    #[must_use]
    pub const fn as_fragment(&self) -> Option<&MathDomFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    /// Mutable type-safe access to Fragment variant
    pub const fn as_fragment_mut(&mut self) -> Option<&mut MathDomFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }
}

// From/Into trait implementations for compatibility
impl From<MathNode> for MathDomNode {
    fn from(node: MathNode) -> Self {
        Self::Math(node)
    }
}

impl From<TextNode> for MathDomNode {
    fn from(node: TextNode) -> Self {
        Self::Text(node)
    }
}

impl From<SpaceNode> for MathDomNode {
    fn from(node: SpaceNode) -> Self {
        Self::Space(node)
    }
}

impl From<MathDomFragment> for MathDomNode {
    fn from(fragment: MathDomFragment) -> Self {
        Self::Fragment(Box::new(fragment))
    }
}
