//! DOM tree node definitions for HTML rendering
//!
//! These objects store the data about the DOM nodes we create, as well as some
//! extra data. They can then be transformed into real DOM nodes with the
//! `to_node` function or HTML markup using `to_markup`.

use core::fmt::{self, Write as _};

use crate::ParseError;
use crate::namespace::KeyMap;
use crate::types::ParseErrorKind;
#[cfg(feature = "wasm")]
use crate::web_context::WebContext;
use bon::bon;
use phf::phf_map;
#[cfg(feature = "wasm")]
use web_sys;

use crate::mathml_tree::MathNode;
use crate::options::Options;
use crate::svg_geometry::PATH_MAP;
use crate::tree::{DocumentFragment, VirtualNode};
use crate::types::{CssProperty, CssStyle};
use crate::unicode::script_from_codepoint;
use crate::units::make_em;
use crate::utils::escape_into;

/// Span wrapping other DOM nodes with generic child type
#[derive(Debug, Clone, PartialEq)]
pub struct Span<T> {
    /// Child nodes contained within this span
    pub children: Vec<T>,
    /// HTML attributes for this span element
    pub attributes: KeyMap<String, String>,
    /// CSS classes applied to this span
    pub classes: Vec<String>,
    /// Height of this span element
    pub height: f64,
    /// Depth of this span element
    pub depth: f64,
    /// Optional width of this span element
    pub width: Option<f64>,
    /// Maximum font size used in this span
    pub max_font_size: f64,
    /// Inline CSS style object
    pub style: CssStyle,

    /// For `src/functions/delimsizing.rs` only
    pub is_middle: Option<(String, Options)>,
    /// For `src/functions/op.rs` and `src/functions/supsub.rs` only
    pub italic: Option<f64>,
}

#[bon]
/// Builder for creating a new Span
impl<T> Span<T> {
    #[builder]
    /// Create a new Span with builder
    #[expect(clippy::option_option)]
    pub fn new(
        /// Options for building the span
        #[builder(finish_fn)]
        options: Option<&Options>,
        /// Child nodes contained within this span
        children: Vec<T>,
        /// Attributes for this span element
        attributes: Option<KeyMap<String, String>>,
        /// Classes applied to this span
        classes: Option<Vec<String>>,
        /// Height of this span element
        height: Option<f64>,
        /// Depth of this span element
        depth: Option<f64>,
        /// Optional width of this span element
        width: Option<Option<f64>>,
        /// Maximum font size used in this span
        max_font_size: Option<f64>,
        /// Inline CSS style object
        style: Option<CssStyle>,
        /// `is_middle` tuple
        is_middle: Option<(String, Options)>,
    ) -> Self {
        let mut span = Self {
            children,
            attributes: attributes.unwrap_or_default(),
            classes: classes.unwrap_or_default(),
            height: height.unwrap_or_default(),
            depth: depth.unwrap_or_default(),
            width: width.unwrap_or(None),
            max_font_size: max_font_size.unwrap_or_default(),
            style: style.unwrap_or_default(),
            is_middle,
            italic: None,
        };

        if let Some(options) = options {
            init_node(&mut span.classes, &mut span.style, options);
        }

        span
    }
}

/// Anchor element with hyperlink
#[derive(Debug, Clone)]
pub struct Anchor {
    /// Child nodes contained within this anchor
    pub children: Vec<HtmlDomNode>,
    /// HTML attributes for this anchor element
    pub attributes: KeyMap<String, String>,
    /// CSS classes applied to this anchor
    pub classes: Vec<String>,
    /// Height of this anchor element
    pub height: f64,
    /// Depth of this anchor element
    pub depth: f64,
    /// Maximum font size used in this anchor
    pub max_font_size: f64,
    /// Inline CSS style object
    pub style: CssStyle,
}

impl From<Anchor> for HtmlDomNode {
    fn from(anchor: Anchor) -> Self {
        Self::Anchor(anchor)
    }
}

#[bon]
impl Anchor {
    #[builder]
    /// Create a new Anchor element with builder
    pub fn new(
        /// Options for building the anchor
        #[builder(finish_fn)]
        options: Option<&Options>,
        /// Child nodes contained within this anchor
        children: Option<Vec<HtmlDomNode>>,
        /// HTML attributes for this anchor element
        attributes: Option<KeyMap<String, String>>,
        /// Classes applied to this anchor
        classes: Option<Vec<String>>,
        /// Height of this anchor element
        height: Option<f64>,
        /// Depth of this anchor element
        depth: Option<f64>,
        /// Maximum font size used in this anchor
        max_font_size: Option<f64>,
        /// Inline CSS style object
        style: Option<CssStyle>,
    ) -> Self {
        let mut anchor = Self {
            children: children.unwrap_or_default(),
            attributes: attributes.unwrap_or_default(),
            classes: classes.unwrap_or_default(),
            height: height.unwrap_or_default(),
            depth: depth.unwrap_or_default(),
            max_font_size: max_font_size.unwrap_or_default(),
            style: style.unwrap_or_default(),
        };

        if let Some(options) = options {
            init_node(&mut anchor.classes, &mut anchor.style, options);
        }

        anchor
    }
}

impl Anchor {
    /// Create a new Anchor (Going to be deprecated)
    #[must_use]
    pub const fn new(
        children: Vec<HtmlDomNode>,
        attributes: KeyMap<String, String>,
        classes: Vec<String>,
        height: f64,
        depth: f64,
        max_font_size: f64,
        style: CssStyle,
    ) -> Self {
        Self {
            children,
            attributes,
            classes,
            height,
            depth,
            max_font_size,
            style,
        }
    }
}

/// Image embed element
#[derive(Debug, Clone)]
pub struct Img {
    /// Source URL of the image
    pub src: String,
    /// Alternative text for the image
    pub alt: String,
    /// CSS classes applied to this image
    pub classes: Vec<String>,
    /// Height of this image element
    pub height: f64,
    /// Depth of this image element
    pub depth: f64,
    /// Maximum font size used in this image
    pub max_font_size: f64,
    /// Inline CSS style object
    pub style: CssStyle,
}

impl Img {
    /// Create a new Img
    #[must_use]
    pub fn new(
        src: String,
        alt: String,
        height: f64,
        depth: f64,
        max_font_size: f64,
        style: CssStyle,
    ) -> Self {
        Self {
            src,
            alt,
            classes: vec!["mord".to_owned()],
            height,
            depth,
            max_font_size,
            style,
        }
    }
}

/// Symbol node containing information about a single symbol
#[derive(Debug, Clone)]
pub struct SymbolNode {
    /// The text content of this symbol
    pub text: String,
    /// Height of this symbol
    pub height: f64,
    /// Depth of this symbol
    pub depth: f64,
    /// Italic correction value
    pub italic: f64,
    /// Skew correction value
    pub skew: f64,
    /// Width of this symbol
    pub width: f64,
    /// Maximum font size used in this symbol
    pub max_font_size: f64,
    /// CSS classes applied to this symbol
    pub classes: Vec<String>,
    /// Inline CSS style object
    pub style: CssStyle,
}

impl From<SymbolNode> for HtmlDomNode {
    fn from(symbol: SymbolNode) -> Self {
        Self::Symbol(symbol)
    }
}

const I_COMBINATIONS: phf::Map<&str, &str> = phf_map! {
    "\u{ee}" => "\u{0131}\u{0302}",
    "\u{ef}" => "\u{0131}\u{0308}",
    "\u{ed}" => "\u{0131}\u{0301}",
    "\u{ec}" => "\u{0131}\u{0300}",
};

#[bon]
impl SymbolNode {
    /// Create a new Symbol
    #[builder]
    pub fn new(
        /// Symbol text for the node
        text: &str,
        /// Height of the symbol
        height: Option<f64>,
        /// Depth of the symbol
        depth: Option<f64>,
        /// Italic correction value
        italic: Option<f64>,
        /// Skew correction value
        skew: Option<f64>,
        /// Width of the symbol
        width: Option<f64>,
        /// Maximum font size used in this symbol
        max_font_size: Option<f64>,
        /// Classes applied to this symbol
        classes: Option<Vec<String>>,
        /// Inline CSS style object
        style: Option<CssStyle>,
    ) -> Self {
        let mut classes = classes.unwrap_or_default();

        // Mark text from non-Latin scripts with specific classes so that we
        // can specify which fonts to use. This allows us to render these
        // characters with a serif font in situations where the browser would
        // either default to a sans serif or render a placeholder character.
        // We use CSS class names like cjk_fallback, hangul_fallback and
        // brahmic_fallback. See ./unicodeScripts.js for the set of possible
        // script names
        if let Some(first_ch) = text.chars().next()
            && let Some(script) = script_from_codepoint(first_ch as u32)
        {
            classes.push(format!("{script}_fallback"));
        }

        // Handle iCombinations for special characters
        let text = I_COMBINATIONS
            .get(text)
            .map_or_else(|| text.to_owned(), ToString::to_string);

        Self {
            text,
            height: height.unwrap_or_default(),
            depth: depth.unwrap_or_default(),
            italic: italic.unwrap_or_default(),
            skew: skew.unwrap_or_default(),
            width: width.unwrap_or_default(),
            max_font_size: max_font_size.unwrap_or_default(),
            classes,
            style: style.unwrap_or_default(),
        }
    }
}

/// Span wrapping other DOM nodes
pub type DomSpan = Span<HtmlDomNode>;

/// Recursive HTML DOM node enum with tuple variants
#[derive(Debug, Clone)]
pub enum HtmlDomNode {
    /// Span wrapping other DOM nodes
    DomSpan(Span<HtmlDomNode>),
    /// Anchor (`<a>`) element with hyperlink
    Anchor(Anchor),
    /// Image embed (`<img>`) element
    Img(Img),
    /// Symbol node containing information about a single symbol
    Symbol(SymbolNode),
    /// SVG node for rendering stretchy wide elements
    SvgNode(SvgNode),
    /// MathML node for mathematical expressions
    MathML(MathNode),
    /// Document fragment containing HTML DOM nodes
    Fragment(HtmlDomFragment),
}

impl From<Span<Self>> for HtmlDomNode {
    fn from(span: Span<Self>) -> Self {
        Self::DomSpan(span)
    }
}

/// SVG child node types
#[derive(Debug, Clone)]
pub enum SvgChildNode {
    /// Path element
    Path(PathNode),
    /// Line element
    Line(LineNode),
}

impl SvgChildNode {
    /// Convert this SVG child node into HTML markup string
    pub fn to_markup(&self) -> Result<String, ParseError> {
        match self {
            Self::Path(path_node) => path_node.to_markup(),
            Self::Line(line_node) => line_node.to_markup(),
        }
    }

    /// Convert this SVG child node into a DOM node representation
    #[cfg(feature = "wasm")]
    #[must_use]
    pub fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        match self {
            Self::Path(path_node) => path_node.to_node(ctx),
            Self::Line(line_node) => line_node.to_node(ctx),
        }
    }
}

/// Document fragment containing HTML DOM nodes
pub type HtmlDomFragment = DocumentFragment<HtmlDomNode>;

impl From<HtmlDomFragment> for HtmlDomNode {
    fn from(fragment: HtmlDomFragment) -> Self {
        Self::Fragment(fragment)
    }
}

/// SVG node for rendering stretchy wide elements
#[derive(Debug, Clone)]
pub struct SvgNode {
    /// Child nodes contained within this SVG
    pub children: Vec<SvgChildNode>,
    /// HTML attributes for this SVG element
    pub attributes: KeyMap<String, String>,
}

#[bon]
impl SvgNode {
    /// Create a new SvgNode
    #[builder]
    pub fn new(
        /// Children for the SVG node
        children: Vec<SvgChildNode>,
        /// Attributes for the SVG node
        attributes: Option<KeyMap<String, String>>,
    ) -> Self {
        Self {
            children,
            attributes: attributes.unwrap_or_default(),
        }
    }
}

/// Create an HTML className based on a list of classes. In addition to joining
/// with spaces, we also remove empty classes.
pub fn create_class(classes: &[String]) -> String {
    classes
        .iter()
        .filter(|cls| !cls.is_empty())
        .map(String::as_str)
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Initialize a DOM node with common properties according to KaTeX.js initNode
/// implementation
#[inline]
fn init_node(classes: &mut Vec<String>, style: &mut CssStyle, options: &Options) {
    if options.style.is_tight() {
        classes.push("mtight".to_owned());
    }
    if let Some(color) = options.get_color() {
        style.insert(CssProperty::Color, color);
    }
}

/// Convert into an HTML node
#[cfg(feature = "wasm")]
#[must_use]
pub fn to_node(node: &HtmlDomNode, ctx: &WebContext) -> web_sys::Node {
    node.to_node(ctx)
}

/// Convert into an HTML markup string
pub fn to_markup(node: &HtmlDomNode) -> Result<String, ParseError> {
    node.to_markup()
}

fn fmt_error() -> ParseError {
    ParseError::new(ParseErrorKind::Message("failed to write markup"))
}

fn map_fmt(result: fmt::Result) -> Result<(), ParseError> {
    result.map_err(|_| fmt_error())
}

fn write_node_class<W: fmt::Write>(writer: &mut W, classes: &[String]) -> fmt::Result {
    if classes.is_empty() {
        return Ok(());
    }

    writer.write_str(" class=\"")?;
    escape_into(writer, &create_class(classes))?;
    writer.write_char('"')
}

fn write_node_style<W: fmt::Write>(writer: &mut W, style: &CssStyle) -> fmt::Result {
    if style.is_empty() {
        return Ok(());
    }

    writer.write_str(" style=\"")?;
    for (key, value) in style {
        writer.write_str(key.as_ref())?;
        writer.write_char(':')?;
        escape_into(writer, value)?;
        writer.write_char(';')?;
    }
    writer.write_char('"')
}

#[cfg(feature = "wasm")]
fn class_to_node(element: &web_sys::Element, classes: &[String]) {
    if !classes.is_empty() {
        let class_attr = create_class(classes);
        element.set_attribute("class", &class_attr).unwrap();
    }
}

#[cfg(feature = "wasm")]
fn style_to_node(element: &web_sys::Element, style: &CssStyle) {
    if !style.is_empty() {
        let styles = style.iter().fold(String::new(), |mut s, (key, value)| {
            let _ = write!(s, "{key}:{value};");
            s
        });
        element.set_attribute("style", &styles).unwrap();
    }
}

/// Implement VirtualNode for `Span<T>`
impl<T: VirtualNode> VirtualNode for Span<T> {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(fmt.write_str("<span"))?;
        map_fmt(write_node_class(fmt, &self.classes))?;
        map_fmt(write_node_style(fmt, &self.style))?;
        node_attributes_to_markup(fmt, &self.attributes)?;
        map_fmt(fmt.write_char('>'))?;

        for child in &self.children {
            child.write_markup(fmt)?;
        }

        map_fmt(fmt.write_str("</span>"))?;
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx.document.create_element("span").unwrap();

        // Add classes
        class_to_node(&element, &self.classes);

        // Add styles
        style_to_node(&element, &self.style);

        // Add attributes
        node_attributes_to_node(&element, &self.attributes);

        // Add children
        for child in &self.children {
            let child_node = child.to_node(ctx);
            element.append_child(&child_node).unwrap();
        }

        element.dyn_into::<web_sys::Node>().unwrap()
    }
}

/// Implement VirtualNode for Anchor
impl VirtualNode for Anchor {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(fmt.write_str("<a"))?;
        map_fmt(write_node_class(fmt, &self.classes))?;
        map_fmt(write_node_style(fmt, &self.style))?;
        node_attributes_to_markup(fmt, &self.attributes)?;
        map_fmt(fmt.write_char('>'))?;

        for child in &self.children {
            child.write_markup(fmt)?;
        }

        map_fmt(fmt.write_str("</a>"))?;
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx.document.create_element("a").unwrap();

        // Add classes
        class_to_node(&element, &self.classes);

        // Add styles
        style_to_node(&element, &self.style);

        // Add attributes
        node_attributes_to_node(&element, &self.attributes);

        // Add children
        for child in &self.children {
            let child_node = child.to_node(ctx);
            element.append_child(&child_node).unwrap();
        }

        element.dyn_into::<web_sys::Node>().unwrap()
    }
}

/// Implement VirtualNode for Img
impl VirtualNode for Img {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(fmt.write_str("<img src=\""))?;
        map_fmt(escape_into(fmt, &self.src))?;
        map_fmt(fmt.write_str("\" alt=\""))?;
        map_fmt(escape_into(fmt, &self.alt))?;
        map_fmt(fmt.write_char('"'))?;
        map_fmt(write_node_class(fmt, &self.classes))?;
        map_fmt(write_node_style(fmt, &self.style))?;
        map_fmt(fmt.write_str("/"))?;
        map_fmt(fmt.write_char('>'))?;
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx.document.create_element("img").unwrap();

        element.set_attribute("src", &self.src).unwrap();
        element.set_attribute("alt", &self.alt).unwrap();

        // Add classes
        class_to_node(&element, &self.classes);

        // Add styles
        style_to_node(&element, &self.style);

        element.dyn_into::<web_sys::Node>().unwrap()
    }
}

fn write_symbol_style<W: fmt::Write>(writer: &mut W, italic: f64, style: &CssStyle) -> fmt::Result {
    if italic <= 0.0 && style.is_empty() {
        return Ok(());
    }

    writer.write_str(" style=\"")?;
    if italic > 0.0 {
        writer.write_str("margin-right:")?;
        writer.write_str(&make_em(italic))?;
        writer.write_char(';')?;
    }
    for (key, value) in style {
        writer.write_str(key.as_ref())?;
        writer.write_char(':')?;
        escape_into(writer, value)?;
        writer.write_char(';')?;
    }
    writer.write_char('"')
}

#[cfg(feature = "wasm")]
fn symbol_node_style_str(italic: f64, style: &CssStyle) -> String {
    use crate::utils::escape;
    let mut styles = String::new();
    if italic > 0.0 {
        let _ = write!(styles, "margin-right:{};", make_em(italic));
    }
    for (key, value) in style {
        let _ = write!(styles, "{key}:{value};");
    }
    escape(&styles)
}

/// Implement VirtualNode for Symbol
impl VirtualNode for SymbolNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        let needs_span = self.italic > 0.0 || !self.classes.is_empty() || !self.style.is_empty();

        if needs_span {
            map_fmt(fmt.write_str("<span"))?;
            map_fmt(write_node_class(fmt, &self.classes))?;
            map_fmt(write_symbol_style(fmt, self.italic, &self.style))?;
            map_fmt(fmt.write_char('>'))?;
            map_fmt(escape_into(fmt, &self.text))?;
            map_fmt(fmt.write_str("</span>"))?;
        } else {
            map_fmt(escape_into(fmt, &self.text))?;
        }

        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let needs_span = self.italic > 0.0 || !self.classes.is_empty() || !self.style.is_empty();

        if needs_span {
            let element = ctx.document.create_element("span").unwrap();

            // Add classes
            if !self.classes.is_empty() {
                let class_attr = create_class(&self.classes);
                element.set_attribute("class", &class_attr).unwrap();
            }

            // Add styles
            let styles = symbol_node_style_str(self.italic, &self.style);
            if !styles.is_empty() {
                element.set_attribute("style", &styles).unwrap();
            }

            // Add text content
            let text_node = ctx.document.create_text_node(&self.text);
            element.append_child(&text_node).unwrap();

            element.dyn_into::<web_sys::Node>().unwrap()
        } else {
            // Just return a text node
            ctx.document
                .create_text_node(&self.text)
                .dyn_into::<web_sys::Node>()
                .unwrap()
        }
    }
}

/// Implement VirtualNode for SvgNode
impl VirtualNode for SvgNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(fmt.write_str("<svg xmlns=\"http://www.w3.org/2000/svg\""))?;
        node_attributes_to_markup(fmt, &self.attributes)?;
        map_fmt(fmt.write_char('>'))?;

        for child in &self.children {
            match child {
                SvgChildNode::Path(path) => path.write_markup(fmt)?,
                SvgChildNode::Line(line) => line.write_markup(fmt)?,
            }
        }

        map_fmt(fmt.write_str("</svg>"))?;
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx
            .document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
            .unwrap();

        // Add attributes
        node_attributes_to_node(&element, &self.attributes);

        // Add children
        for child in &self.children {
            let child_node = child.to_node(ctx);
            element.append_child(&child_node).unwrap();
        }

        element.dyn_into::<web_sys::Node>().unwrap()
    }
}

/// Implement VirtualNode for HtmlDomNode
impl VirtualNode for HtmlDomNode {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        match self {
            Self::DomSpan(span) => span.write_markup(fmt),
            Self::Anchor(anchor) => anchor.write_markup(fmt),
            Self::Img(img) => img.write_markup(fmt),
            Self::Symbol(symbol) => symbol.write_markup(fmt),
            Self::SvgNode(svg_node) => svg_node.write_markup(fmt),
            Self::MathML(math_node) => math_node.write_markup(fmt),
            Self::Fragment(fragment) => fragment.write_markup(fmt),
        }
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        match self {
            Self::DomSpan(span) => span.to_node(ctx),
            Self::Anchor(anchor) => anchor.to_node(ctx),
            Self::Img(img) => img.to_node(ctx),
            Self::Symbol(symbol) => symbol.to_node(ctx),
            Self::SvgNode(svg_node) => svg_node.to_node(ctx),
            Self::MathML(math_node) => math_node.to_node(ctx),
            Self::Fragment(fragment) => fragment.to_node(ctx),
        }
    }
}

/// Helper methods for HtmlDomNode to maintain API compatibility
///
/// These methods provide a unified interface for accessing properties of
/// different HTML DOM node types. Not all node types support all properties -
/// for example, SVG and MathML nodes don't have traditional CSS classes or
/// dimensions.
impl HtmlDomNode {
    /// Get the CSS classes applied to this node
    ///
    /// Returns an empty slice for node types that don't support CSS classes
    /// (SvgNode, MathML). For other node types, returns their class list.
    #[must_use]
    pub fn classes(&self) -> &[String] {
        match self {
            Self::DomSpan(span) => &span.classes,
            Self::Anchor(anchor) => &anchor.classes,
            Self::Img(img) => &img.classes,
            Self::Symbol(symbol) => &symbol.classes,
            Self::SvgNode(_) | Self::MathML { .. } => &[],
            Self::Fragment(fragment) => &fragment.classes,
        }
    }

    /// Try to mutate the CSS classes applied to this node
    ///
    /// Returns `Some(&mut Vec<String>)` for node types that support mutable CSS
    /// classes, or `None` for node types that don't support CSS classes
    /// (SvgNode, MathML).
    pub const fn classes_mut(&mut self) -> Option<&mut Vec<String>> {
        match self {
            Self::DomSpan(span) => Some(&mut span.classes),
            Self::Anchor(anchor) => Some(&mut anchor.classes),
            Self::Img(img) => Some(&mut img.classes),
            Self::Symbol(symbol) => Some(&mut symbol.classes),
            Self::SvgNode(_) | Self::MathML { .. } => None,
            Self::Fragment(fragment) => Some(&mut fragment.classes),
        }
    }

    /// Get the height of this node
    ///
    /// Returns the vertical height above the baseline in em units.
    /// For node types that don't have a defined height (SvgNode, MathML),
    /// returns 0.0.
    #[must_use]
    pub const fn height(&self) -> f64 {
        match self {
            Self::DomSpan(span) => span.height,
            Self::Anchor(anchor) => anchor.height,
            Self::Img(img) => img.height,
            Self::Symbol(symbol) => symbol.height,
            Self::SvgNode(_) | Self::MathML { .. } => 0.0,
            Self::Fragment(fragment) => fragment.height,
        }
    }

    /// Try to set the height of this node
    ///
    /// Returns `Some(&mut f64)` for node types that support mutable height,
    /// or `None` for node types that don't have a defined height (SvgNode,
    /// MathML).
    pub const fn height_mut(&mut self) -> Option<&mut f64> {
        match self {
            Self::DomSpan(span) => Some(&mut span.height),
            Self::Anchor(anchor) => Some(&mut anchor.height),
            Self::Img(img) => Some(&mut img.height),
            Self::Symbol(symbol) => Some(&mut symbol.height),
            Self::SvgNode(_) | Self::MathML { .. } => None,
            Self::Fragment(fragment) => Some(&mut fragment.height),
        }
    }

    /// Get the depth of this node
    ///
    /// Returns the vertical depth below the baseline in em units.
    /// For node types that don't have a defined depth (SvgNode, MathML),
    /// returns 0.0.
    #[must_use]
    pub const fn depth(&self) -> f64 {
        match self {
            Self::DomSpan(span) => span.depth,
            Self::Anchor(anchor) => anchor.depth,
            Self::Img(img) => img.depth,
            Self::Symbol(symbol) => symbol.depth,
            Self::SvgNode(_) | Self::MathML { .. } => 0.0,
            Self::Fragment(fragment) => fragment.depth,
        }
    }

    /// Try to set the depth of this node
    ///
    /// Returns `Some(&mut f64)` for node types that support mutable depth,
    /// or `None` for node types that don't have a defined depth (SvgNode,
    /// MathML).
    pub const fn depth_mut(&mut self) -> Option<&mut f64> {
        match self {
            Self::DomSpan(span) => Some(&mut span.depth),
            Self::Anchor(anchor) => Some(&mut anchor.depth),
            Self::Img(img) => Some(&mut img.depth),
            Self::Symbol(symbol) => Some(&mut symbol.depth),
            Self::SvgNode(_) | Self::MathML { .. } => None,
            Self::Fragment(fragment) => Some(&mut fragment.depth),
        }
    }

    /// Get the maximum font size used in this node
    ///
    /// Returns the largest font size used within this node and its children in
    /// em units. For node types that don't have a defined max_font_size
    /// (SvgNode, MathML), returns 0.0.
    #[must_use]
    pub const fn max_font_size(&self) -> f64 {
        match self {
            Self::DomSpan(span) => span.max_font_size,
            Self::Anchor(anchor) => anchor.max_font_size,
            Self::Img(img) => img.max_font_size,
            Self::Symbol(symbol) => symbol.max_font_size,
            Self::SvgNode(_) | Self::MathML { .. } => 0.0,
            Self::Fragment(fragment) => fragment.max_font_size,
        }
    }

    /// Try to set the maximum font size of this node
    ///
    /// Returns `Some(&mut f64)` for node types that support mutable
    /// max_font_size, or `None` for node types that don't have a defined
    /// max_font_size (SvgNode, MathML).
    pub const fn max_font_size_mut(&mut self) -> Option<&mut f64> {
        match self {
            Self::DomSpan(span) => Some(&mut span.max_font_size),
            Self::Anchor(anchor) => Some(&mut anchor.max_font_size),
            Self::Img(img) => Some(&mut img.max_font_size),
            Self::Symbol(symbol) => Some(&mut symbol.max_font_size),
            Self::SvgNode(_) | Self::MathML { .. } => None,
            Self::Fragment(fragment) => Some(&mut fragment.max_font_size),
        }
    }

    /// Get the width of this node
    ///
    /// Returns the horizontal width in em units, or `None` if the node doesn't
    /// have a defined width. Most node types don't have a defined width,
    /// with Symbol being the primary exception.
    #[must_use]
    pub const fn width(&self) -> Option<f64> {
        match self {
            Self::DomSpan(span) => span.width,
            Self::Anchor(_)
            | Self::Img(_)
            | Self::SvgNode(_)
            | Self::MathML { .. }
            | Self::Fragment(_) => None,
            Self::Symbol(symbol) => Some(symbol.width),
        }
    }

    /// Get the inline CSS style object
    ///
    /// Returns a reference to the CSS style properties applied to this node.
    /// For node types that don't support inline styles (SvgNode, MathML),
    /// returns an empty style object.
    #[must_use]
    pub const fn style(&self) -> Option<&CssStyle> {
        match self {
            Self::DomSpan(span) => Some(&span.style),
            Self::Anchor(anchor) => Some(&anchor.style),
            Self::Img(img) => Some(&img.style),
            Self::Symbol(symbol) => Some(&symbol.style),
            Self::Fragment(fragment) => Some(&fragment.style),
            Self::SvgNode(_) | Self::MathML { .. } => None,
        }
    }

    /// Get a mutable reference to the inline CSS style object
    ///
    /// Returns a mutable reference to the CSS style properties applied to this
    /// node. For node types that don't support inline styles (SvgNode,
    /// MathML), returns an empty style object.
    pub const fn style_mut(&mut self) -> Option<&mut CssStyle> {
        match self {
            Self::DomSpan(span) => Some(&mut span.style),
            Self::Anchor(anchor) => Some(&mut anchor.style),
            Self::Img(img) => Some(&mut img.style),
            Self::Symbol(symbol) => Some(&mut symbol.style),
            Self::SvgNode(_) | Self::MathML { .. } => None,
            Self::Fragment(fragment) => Some(&mut fragment.style),
        }
    }

    /// Check if this node has a specific CSS class
    ///
    /// Returns `true` if the node contains the specified CSS class in its class
    /// list. For node types that don't support CSS classes (SvgNode,
    /// MathML), always returns `false`.
    #[must_use]
    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes().iter().any(|cls| cls == class_name)
    }

    /// Get the attributes of this node
    #[must_use]
    pub const fn attributes(&self) -> Option<&KeyMap<String, String>> {
        match self {
            Self::DomSpan(span) => Some(&span.attributes),
            Self::Anchor(anchor) => Some(&anchor.attributes),
            Self::Img(_) | Self::Symbol(_) | Self::Fragment(_) => None,
            Self::SvgNode(svg_node) => Some(&svg_node.attributes),
            Self::MathML(mathml) => Some(&mathml.attributes),
        }
    }
}

/// SVG path node
#[derive(Debug, Clone)]
pub struct PathNode {
    /// Name of the predefined path
    pub path_name: String,
    /// Optional alternate path data (used for sqrt, phase, tall delimiters)
    pub alternate: Option<String>,
}

impl VirtualNode for PathNode {
    /// Convert this path node into HTML markup string
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        let path_data = self.alternate.as_ref().map_or_else(
            || {
                PATH_MAP
                    .get(&self.path_name)
                    .map_or_else(String::new, |s| (*s).to_owned())
            },
            Clone::clone,
        );

        map_fmt(fmt.write_str("<path d=\""))?;
        map_fmt(escape_into(fmt, &path_data))?;
        map_fmt(fmt.write_str("\"/>"))?;
        Ok(())
    }

    /// Convert this path node into a DOM node representation
    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx
            .document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "path")
            .unwrap();

        let path_data = self.alternate.as_ref().map_or_else(
            || {
                PATH_MAP
                    .get(&self.path_name)
                    .map_or_else(String::new, |s| (*s).to_owned())
            },
            Clone::clone,
        );

        element.set_attribute("d", &path_data).unwrap();
        element.dyn_into::<web_sys::Node>().unwrap()
    }
}

/// SVG line node
#[derive(Debug, Clone)]
pub struct LineNode {
    /// SVG attributes for this line element
    pub attributes: KeyMap<String, String>,
}

fn node_attributes_to_markup<W: fmt::Write>(
    writer: &mut W,
    attributes: &KeyMap<String, String>,
) -> Result<(), ParseError> {
    for (attr, value) in attributes {
        if !attr.is_empty() {
            if attr.contains(|c: char| {
                c.is_whitespace() || "\"'>/=".contains(c) || ('\x00'..='\x1f').contains(&c)
            }) {
                return Err(ParseError::new(ParseErrorKind::InvalidAttributeName {
                    attr: attr.clone(),
                }));
            }
            map_fmt(write!(writer, " {attr}=\""))?;
            map_fmt(escape_into(writer, value))?;
            map_fmt(writer.write_char('"'))?;
        }
    }
    Ok(())
}

#[cfg(feature = "wasm")]
fn node_attributes_to_node(element: &web_sys::Element, attributes: &KeyMap<String, String>) {
    for (attr, value) in attributes {
        if !attr.is_empty() {
            if attr.contains(|c: char| {
                c.is_whitespace() || "\"'>/=".contains(c) || ('\x00'..='\x1f').contains(&c)
            }) {
                continue;
            }
            element.set_attribute(attr, value).unwrap();
        }
    }
}

impl VirtualNode for LineNode {
    /// Convert this line node into HTML markup string
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        map_fmt(fmt.write_str("<line"))?;
        node_attributes_to_markup(fmt, &self.attributes)?;
        map_fmt(fmt.write_str("/"))?;
        map_fmt(fmt.write_char('>'))?;
        Ok(())
    }

    /// Convert this line node into a DOM node representation
    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let element = ctx
            .document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "line")
            .unwrap();

        // Add attributes
        node_attributes_to_node(&element, &self.attributes);

        element.dyn_into::<web_sys::Node>().unwrap()
    }
}
