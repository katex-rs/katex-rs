//! Tree structure definitions for virtual DOM nodes
//!
//! This module contains the foundational types for KaTeX's virtual DOM system,
//! including the base VirtualNode trait and DocumentFragment structure.

use core::cell::RefCell;
use core::fmt;

use crate::ClassList;
use crate::ParseError;
use crate::types::{CssStyle, ParseErrorKind};
#[cfg(feature = "wasm")]
use crate::web_context::WebContext;
#[cfg(feature = "wasm")]
use web_sys;

/// Base virtual DOM node interface used in both DOM tree and MathML tree
/// implementations
pub trait VirtualNode {
    /// Convert into HTML markup by writing into the provided formatter.
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError>;

    /// Convenience helper that renders the node into a [`String`].
    fn to_markup(&self) -> Result<String, ParseError>
    where
        Self: Sized,
    {
        markup_to_string(self)
    }

    /// Convert into a DOM node
    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node;
}

/// Document fragment containing elements without DOM representation
#[derive(Debug)]
pub struct DocumentFragment<ChildType: VirtualNode> {
    /// The child nodes contained within this document fragment. These represent
    /// the nested elements in the virtual DOM tree structure used for
    /// mathematical rendering in KaTeX. Each child is a VirtualNode that
    /// can be rendered into HTML or MathML markup.
    ///
    /// # See Also
    /// - [`VirtualNode`]: The trait implemented by child nodes
    /// - [`DocumentFragment::new`]: Constructor for creating fragments with
    ///   children
    pub children: Vec<ChildType>,

    /// CSS class names applied to this document fragment. These classes control
    /// styling and layout in the rendered mathematical output, following
    /// KaTeX's CSS conventions.
    ///
    /// # See Also
    /// - [`DocumentFragment::has_class`]: Method to check for specific classes
    pub classes: ClassList,

    /// The height of this document fragment in em units, representing the
    /// vertical extent above the baseline in mathematical rendering. Used
    /// for proper alignment and spacing in LaTeX expressions.
    ///
    /// # Mathematical Context
    /// In KaTeX, height is crucial for aligning superscripts, fractions, and
    /// other vertical elements.
    pub height: f64,

    /// The depth of this document fragment in em units, representing the
    /// vertical extent below the baseline in mathematical rendering. Used
    /// for proper alignment and spacing in LaTeX expressions.
    ///
    /// # Mathematical Context
    /// In KaTeX, depth is crucial for aligning subscripts, fractions, and other
    /// vertical elements.
    pub depth: f64,

    /// The maximum font size used within this document fragment, in points.
    /// This value determines the scaling factor for the entire fragment in
    /// mathematical rendering.
    ///
    /// # See Also
    /// - Font metrics in KaTeX for size calculations
    pub max_font_size: f64,

    /// Inline CSS styles applied to this document fragment. These styles
    /// override default KaTeX styling and allow for custom rendering
    /// adjustments in mathematical expressions.
    ///
    /// # See Also
    /// - [`CssStyle`]: The type used for style properties
    /// - KaTeX CSS documentation for available style properties
    pub style: CssStyle,
}

impl<ChildType: VirtualNode + Clone> Clone for DocumentFragment<ChildType> {
    fn clone(&self) -> Self {
        Self {
            children: self.children.clone(),
            classes: self.classes.clone(),
            height: self.height,
            depth: self.depth,
            max_font_size: self.max_font_size,
            style: self.style.clone(),
        }
    }
}

impl<ChildType: VirtualNode> DocumentFragment<ChildType> {
    /// Create a new document fragment with the given children
    #[must_use]
    pub fn new(children: Vec<ChildType>) -> Self {
        Self {
            children,
            classes: ClassList::Empty,
            height: 0.0,
            depth: 0.0,
            max_font_size: 0.0,
            style: CssStyle::default(),
        }
    }

    /// Check if the fragment has a specific class
    #[must_use]
    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes.contains(class_name)
    }
}

impl<ChildType: VirtualNode + Clone + 'static> VirtualNode for DocumentFragment<ChildType> {
    fn write_markup(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), ParseError> {
        for child in &self.children {
            child.write_markup(fmt)?;
        }
        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn to_node(&self, ctx: &WebContext) -> web_sys::Node {
        use wasm_bindgen::JsCast as _;

        let fragment = ctx.document.create_document_fragment();

        for child in &self.children {
            let node = child.to_node(ctx);
            fragment.append_child(&node).unwrap();
        }

        fragment.dyn_into::<web_sys::Node>().unwrap()
    }
}

pub use crate::dom_tree::HtmlDomNode;
pub use crate::mathml_tree::MathDomNode;

/// Renders the provided [`VirtualNode`] into a [`String`] buffer.
pub fn markup_to_string<T: VirtualNode + ?Sized>(node: &T) -> Result<String, ParseError> {
    struct FormatterWriter<'a> {
        buf: &'a mut String,
    }

    impl fmt::Write for FormatterWriter<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.buf.push_str(s);
            Ok(())
        }
    }

    struct DisplayAdapter<'a, T: VirtualNode + ?Sized> {
        node: &'a T,
        error: &'a RefCell<Option<ParseError>>,
    }

    impl<T: VirtualNode + ?Sized> fmt::Display for DisplayAdapter<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.node.write_markup(f) {
                Ok(()) => Ok(()),
                Err(err) => {
                    self.error.replace(Some(err));
                    Err(fmt::Error)
                }
            }
        }
    }

    let mut buffer = String::new();
    let error = RefCell::new(None);
    let mut writer = FormatterWriter { buf: &mut buffer };
    let adapter = DisplayAdapter {
        node,
        error: &error,
    };

    if fmt::write(&mut writer, format_args!("{adapter}")).is_err() {
        if let Some(err) = error.into_inner() {
            return Err(err);
        }
        return Err(ParseErrorKind::MarkupWriteFailure.into());
    }

    Ok(buffer)
}
