//! Core KaTeX functionality - main entry points and error handling

#[cfg(feature = "wasm")]
use crate::types::ParseErrorKind;
use crate::{
    KatexContext,
    build_common::make_span,
    build_tree::{build_html_tree, build_tree},
    dom_tree::{DomSpan, SymbolNode},
    parse_tree::parse_tree,
    parser::parse_node::AnyParseNode,
    tree::VirtualNode as _,
    types::{ParseError, Settings},
};

fn render_error(
    error: ParseError,
    expression: &str,
    settings: &Settings,
) -> Result<DomSpan, ParseError> {
    if settings.throw_on_error {
        return Err(error);
    }

    let mut node = make_span(
        vec!["katex-error".to_owned()],
        vec![SymbolNode::builder().text(expression).build().into()],
        None,
        None,
    );

    node.attributes
        .insert("title".to_owned(), error.to_string());
    node.attributes.insert(
        "style".to_owned(),
        format!("color: {}", settings.error_color),
    );

    Ok(node)
}

#[cfg(feature = "wasm")]
use web_sys::Node;

/// Parse and build an expression, returning HTML markup string
pub fn render_to_string(
    ctx: &KatexContext,
    expression: &str,
    settings: &Settings,
) -> Result<String, ParseError> {
    let dom_tree = match parse_tree(ctx, expression, settings) {
        Ok(tree) => match build_tree(ctx, &tree, expression, settings) {
            Ok(dom) => Ok(dom),
            Err(e) => {
                if settings.throw_on_error {
                    Err(e)
                } else {
                    Ok(render_error(e, expression, settings)?)
                }
            }
        },
        Err(e) => {
            if settings.throw_on_error {
                Err(e)
            } else {
                Ok(render_error(e, expression, settings)?)
            }
        }
    }?;

    dom_tree.to_markup()
}

/// Parse and build an expression, and place that expression in the DOM node
/// given.
#[cfg(feature = "wasm")]
pub fn render(
    ctx: &KatexContext,
    expression: &str,
    base_node: &Node,
    settings: &Settings,
) -> Result<(), ParseError> {
    use crate::web_context::WebContext;

    base_node.set_text_content(None);
    let tree = parse_tree(ctx, expression, settings)?;
    let dom_tree = build_tree(ctx, &tree, expression, settings)?;
    let web_ctx = WebContext::from_window()
        .ok_or_else(|| ParseError::new(ParseErrorKind::MissingDocument))?;
    let node = dom_tree.to_node(&web_ctx);
    base_node.append_child(&node).map_err(|e| {
        ParseError::new(ParseErrorKind::FailedToAppendChild {
            details: format!("{e:?}"),
        })
    })?;
    Ok(())
}

/// Parse an expression and return the parse tree
///
/// This function parses a LaTeX expression and returns the raw parse tree,
/// equivalent to the `__parse` function in the JavaScript KaTeX API.
///
/// # Parameters
/// * `ctx` - The KaTeX context
/// * `expression` - The LaTeX expression to parse
/// * `settings` - Settings for parsing
///
/// # Returns
/// A `Result` containing the parse tree as a vector of `AnyParseNode`s or a
/// `ParseError`
pub fn parse(
    ctx: &KatexContext,
    expression: &str,
    settings: &Settings,
) -> Result<Vec<AnyParseNode>, ParseError> {
    parse_tree(ctx, expression, settings)
}

/// Render an expression to a DOM tree
///
/// This function parses and builds a LaTeX expression, returning the DOM tree
/// representation, equivalent to the `__renderToDomTree` function in the
/// JavaScript KaTeX API.
///
/// # Parameters
/// * `ctx` - The KaTeX context
/// * `expression` - The LaTeX expression to render
/// * `settings` - Settings for rendering
///
/// # Returns
/// A `Result` containing the DOM tree or a `ParseError`
pub fn render_to_dom_tree(
    ctx: &KatexContext,
    expression: &str,
    settings: &Settings,
) -> Result<DomSpan, ParseError> {
    let dom_tree = match parse_tree(ctx, expression, settings) {
        Ok(tree) => match build_tree(ctx, &tree, expression, settings) {
            Ok(dom) => Ok(dom),
            Err(e) => {
                if settings.throw_on_error {
                    Err(e)
                } else {
                    Ok(render_error(e, expression, settings)?)
                }
            }
        },
        Err(e) => {
            if settings.throw_on_error {
                Err(e)
            } else {
                Ok(render_error(e, expression, settings)?)
            }
        }
    }?;

    Ok(dom_tree)
}

/// Render an expression to an HTML-only DOM tree
///
/// This function parses and builds a LaTeX expression, returning the HTML-only
/// DOM tree representation, equivalent to the `__renderToHTMLTree` function in
/// the JavaScript KaTeX API.
///
/// # Parameters
/// * `ctx` - The KaTeX context
/// * `expression` - The LaTeX expression to render
/// * `settings` - Optional settings for rendering
///
/// # Returns
/// A `Result` containing the HTML DOM tree or a `ParseError`
pub fn render_to_html_tree(
    ctx: &KatexContext,
    expression: &str,
    settings: &Settings,
) -> Result<DomSpan, ParseError> {
    let dom_tree = match parse_tree(ctx, expression, settings) {
        Ok(tree) => match build_html_tree(ctx, &tree, expression, settings) {
            Ok(dom) => Ok(dom),
            Err(e) => {
                if settings.throw_on_error {
                    Err(e)
                } else {
                    Ok(render_error(e, expression, settings)?)
                }
            }
        },
        Err(e) => {
            if settings.throw_on_error {
                Err(e)
            } else {
                Ok(render_error(e, expression, settings)?)
            }
        }
    }?;

    Ok(dom_tree)
}
