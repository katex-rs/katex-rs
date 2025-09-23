//! Build tree utilities for KaTeX Rust implementation
//!
//! This module provides functions for building the final DOM tree from parse
//! trees, migrated from the JavaScript buildTree.js file.

use crate::build_common::make_span;
use crate::build_html::build_html;
use crate::build_mathml::build_mathml;
use crate::context::KatexContext;
use crate::dom_tree::DomSpan;
use crate::options::{FontShape, FontWeight, Options};
use crate::parser::parse_node::AnyParseNode;
use crate::style;
use crate::types::{OutputFormat, ParseError, Settings};

/// Creates Options from Settings for building
fn options_from_settings(settings: &Settings) -> Options {
    let style = if settings.display_mode {
        style::DISPLAY
    } else {
        style::TEXT
    };

    Options {
        style,
        color: settings.color.clone(),
        size: Options::BASESIZE,
        text_size: Options::BASESIZE,
        phantom: false,
        font: String::new(),
        font_family: String::new(),
        font_weight: FontWeight::Empty,
        font_shape: FontShape::Empty,
        size_multiplier: settings.size_multiplier,
        max_size: settings.max_size,
        min_rule_thickness: settings.min_rule_thickness,
    }
}

/// Wraps the node with display-related classes if in display mode
fn display_wrap(node: DomSpan, settings: &Settings) -> DomSpan {
    if settings.display_mode {
        let mut classes = vec!["katex-display".to_owned()];
        if settings.leqno {
            classes.push("leqno".to_owned());
        }
        if settings.fleqn {
            classes.push("fleqn".to_owned());
        }
        make_span(classes, vec![node.into()], None, None)
    } else {
        node
    }
}

/// Builds the final DOM tree from a parse tree
///
/// This is the main entry point for building the DOM representation of a
/// mathematical expression. It handles different output formats (HTML, MathML,
/// or both) and applies display mode wrapping as needed.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing builders and symbols
/// * `tree` - The parse tree to build from
/// * `expression` - The original LaTeX expression string
/// * `settings` - Rendering settings
///
/// # Returns
/// A `Result` containing a `DomSpan` with the built DOM tree or a `ParseError`
pub fn build_tree(
    ctx: &KatexContext,
    tree: &[AnyParseNode],
    expression: &str,
    settings: &Settings,
) -> Result<DomSpan, ParseError> {
    let options = options_from_settings(settings);

    let katex_node = match settings.output {
        OutputFormat::Mathml => {
            // MathML only
            build_mathml(ctx, tree, expression, &options, settings.display_mode, true)?
        }
        OutputFormat::Html => {
            // HTML only
            let html_node = build_html(ctx, tree, &options)?;
            make_span(vec!["katex".to_owned()], vec![html_node], None, None)
        }
        OutputFormat::HtmlAndMathml => {
            // Both HTML and MathML
            let mathml_node = build_mathml(
                ctx,
                tree,
                expression,
                &options,
                settings.display_mode,
                false,
            )?;
            let html_node = build_html(ctx, tree, &options)?;
            make_span(
                vec!["katex".to_owned()],
                vec![mathml_node.into(), html_node],
                None,
                None,
            )
        }
    };

    Ok(display_wrap(katex_node, settings))
}

/// Builds HTML-only DOM tree from a parse tree
///
/// This function specifically builds HTML output, similar to buildTree but
/// guaranteed to produce HTML-only output regardless of settings.
///
/// # Parameters
/// * `ctx` - The KaTeX context containing builders and symbols
/// * `tree` - The parse tree to build from
/// * `expression` - The original LaTeX expression string
/// * `settings` - Rendering settings
///
/// # Returns
/// A `Result` containing a `DomSpan` with the built HTML DOM tree or a
/// `ParseError`
pub fn build_html_tree(
    ctx: &KatexContext,
    tree: &[AnyParseNode],
    _expression: &str,
    settings: &Settings,
) -> Result<DomSpan, ParseError> {
    let options = options_from_settings(settings);
    let html_node = build_html(ctx, tree, &options)?;
    let katex_node = make_span(vec!["katex".to_owned()], vec![html_node], None, None);
    Ok(display_wrap(katex_node, settings))
}
