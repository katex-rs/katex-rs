/// Shared web context cached during `KatexContext` initialization.
///
/// This structure stores handles to frequently accessed Web APIs so that
/// rendering code can avoid repeated `window`/`document` lookups.  Fetching
/// these objects from the browser can be surprisingly expensive when performed
/// for every node in a large DOM tree.  By caching the `Document` once and
/// passing the [`WebContext`] through the rendering pipeline, the WebAssembly
/// backend can reuse the same handles throughout the entire tree conversion.
#[derive(Clone)]
pub struct WebContext {
    /// The window.document cache
    pub document: web_sys::Document,
}

impl WebContext {
    /// Creates a new [`WebContext`] wrapping the provided
    /// [`web_sys::Document`].
    #[must_use]
    pub fn new(document: web_sys::Document) -> Self {
        Self { document }
    }

    /// Attempts to construct a [`WebContext`] from the global `window` object.
    ///
    /// This helper is used during [`KatexContext`](crate::KatexContext)
    /// initialization to eagerly cache the document when it is available.  The
    /// lookup falls back to `None` in environments without a DOM (such as the
    /// Node.js benchmarks).
    #[must_use]
    pub fn from_window() -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        Some(Self::new(document))
    }
}
