//! Text function implementations for KaTeX Rust
//!
//! This module handles text-related functions in mathematical expressions,
//! migrated from KaTeX's text.js.

use crate::build_common::make_span;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::MathDomNode;
use crate::options::{FontShape, FontWeight, Options};
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeText};
use crate::types::{ArgType, Mode, ParseError};
use crate::{build_html, build_mathml};
use phf::phf_map;

/// Text font families mapping
static TEXT_FONT_FAMILIES: phf::Map<&'static str, &'static str> = phf_map! {
    "\\text" => "",
    "\\textrm" => "textrm",
    "\\textsf" => "textsf",
    "\\texttt" => "texttt",
    "\\textnormal" => "textrm",
};

/// Text font weights mapping
static TEXT_FONT_WEIGHTS: phf::Map<&'static str, FontWeight> = phf_map! {
    "\\textbf" => FontWeight::TextBf,
    "\\textmd" => FontWeight::TextMd,
};

/// Text font shapes mapping
static TEXT_FONT_SHAPES: phf::Map<&'static str, FontShape> = phf_map! {
    "\\textit" => FontShape::TextIt,
    "\\textup" => FontShape::TextUp,
};

/// Creates options with font applied based on the group font
fn options_with_font(group: &ParseNodeText, options: &Options) -> Options {
    let font = group.font.as_deref().unwrap_or("");

    if font.is_empty() {
        return options.clone();
    }

    if let Some(font_family) = TEXT_FONT_FAMILIES.get(font) {
        return options.with_text_font_family((*font_family).to_owned());
    }

    if let Some(font_weight) = TEXT_FONT_WEIGHTS.get(font) {
        return options.with_text_font_weight(font_weight.clone());
    }

    if font == "\\emph" {
        // Toggle italic/upright based on current shape
        let new_shape = if options.font_shape == FontShape::TextIt {
            FontShape::TextUp
        } else {
            FontShape::TextIt
        };
        return options.with_text_font_shape(new_shape);
    }

    // Default to font shape
    if let Some(font_shape) = TEXT_FONT_SHAPES.get(font) {
        return options.with_text_font_shape(font_shape.clone());
    }

    options.with_text_font_shape(FontShape::Empty)
}

/// Registers text functions in the KaTeX context
pub fn define_text(ctx: &mut crate::KatexContext) {
    let names = [
        // Font families
        "\\text",
        "\\textrm",
        "\\textsf",
        "\\texttt",
        "\\textnormal",
        // Font weights
        "\\textbf",
        "\\textmd",
        // Font shapes
        "\\textit",
        "\\textup",
        "\\emph",
    ];

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Text),
        names: &names,
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Mode(Mode::Text)]),
            allowed_in_argument: true,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = ord_argument(&args[0]);
            Ok(ParseNode::Text(ParseNodeText {
                mode: context.parser.mode,
                loc: context.loc(),
                body,
                font: Some(context.func_name),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for text nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Text(group) = node else {
        return Err(ParseError::new("Expected Text node"));
    };

    let new_options = options_with_font(group, options);
    let inner = build_html::build_expression(
        ctx,
        &group.body,
        &new_options,
        build_html::GroupType::True,
        (None, None),
    )?;
    Ok(make_span(
        vec!["mord".to_owned(), "text".to_owned()],
        inner,
        Some(&new_options),
        None,
    )
    .into())
}

/// MathML builder for text nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Text(group) = node else {
        return Err(ParseError::new("Expected Text node"));
    };

    let new_options = options_with_font(group, options);
    build_mathml::build_expression_row(ctx, &group.body, &new_options, None)
}
