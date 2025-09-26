//! Font function implementations for KaTeX Rust
//!
//! This module handles font changing commands in mathematical expressions,
//! migrated from KaTeX's font.js.

use phf::phf_map;

use crate::define_function::{
    FunctionContext, FunctionDefSpec, FunctionPropSpec, normalize_argument,
};
use crate::dom_tree::HtmlDomNode;
use crate::functions::mclass::binrel_class;
use crate::mathml_tree::MathDomNode;
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNode, ParseNodeFont, ParseNodeMclass, ParseNodeOrdGroup,
};
use crate::types::ParseError;
use crate::{KatexContext, build_html, build_mathml};

/// HTML builder for font nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Font(font_node) = node else {
        return Err(ParseError::new("Expected Font node"));
    };

    let new_options = options.with_font(font_node.font.clone());
    build_html::build_group(ctx, &font_node.body, &new_options, None)
}

/// MathML builder for font nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Font(font_node) = node else {
        return Err(ParseError::new("Expected Font node"));
    };

    let new_options = options.with_font(font_node.font.clone());
    build_mathml::build_group(ctx, &font_node.body, &new_options)
}

// Font aliases
const FONT_ALIASES_MAP: phf::Map<&str, &str> = phf_map!(
    "\\Bbb" => "\\mathbb",
    "\\bold" => "\\mathbf",
    "\\frak" => "\\mathfrak",
    "\\bm" => "\\boldsymbol",
);

// Main font functions
const FONT_NAMES: &[&str] = &[
    // styles, except \boldsymbol defined below
    "\\mathrm",
    "\\mathit",
    "\\mathbf",
    "\\mathnormal",
    "\\mathsfit",
    // families
    "\\mathbb",
    "\\mathcal",
    "\\mathfrak",
    "\\mathscr",
    "\\mathsf",
    "\\mathtt",
    // aliases, except \bm defined below
    "\\Bbb",
    "\\bold",
    "\\frak",
];

/// Registers font functions in the KaTeX context
pub fn define_font(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Font),
        names: FONT_NAMES,
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_argument: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let body = normalize_argument(&args[0]);
            let mut func = context.func_name.clone();

            // Apply aliases
            if let Some(replacement) = FONT_ALIASES_MAP.get(func.as_str()) {
                func = (*replacement).to_owned();
            }

            let font = func[1..].to_string(); // Remove backslash

            Ok(ParseNode::Font(ParseNodeFont {
                mode: context.parser.mode,
                loc: context.loc(),
                font,
                body: Box::new(body.clone()),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \boldsymbol and \bm
    let bold_names = ["\\boldsymbol", "\\bm"];
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Mclass),
        names: &bold_names,
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let body = &args[0];
            let is_character_box = body.is_character_box()?;

            // Create nested structure: mclass containing font
            let font_node = ParseNode::Font(ParseNodeFont {
                mode: context.parser.mode,
                loc: context.loc(),
                font: "boldsymbol".to_owned(),
                body: Box::new(body.clone()),
            });

            Ok(ParseNode::Mclass(ParseNodeMclass {
                mode: context.parser.mode,
                loc: context.loc(),
                mclass: binrel_class(body),
                body: vec![font_node],
                is_character_box,
            }))
        }),
        html_builder: None, // Use mclass builders
        mathml_builder: None,
    });

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Font),
        names: &["\\rm", "\\sf", "\\tt", "\\bf", "\\it", "\\cal"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            let func_name = context.func_name.as_str();
            let body = context
                .parser
                .parse_expression(true, context.break_on_token_text)?;

            let style = format!("math{}", &func_name[1..]);

            let ordgroup = AnyParseNode::OrdGroup(ParseNodeOrdGroup {
                mode: context.parser.mode,
                loc: context.loc(),
                body,
                semisimple: None,
            });

            Ok(ParseNode::Font(ParseNodeFont {
                mode: context.parser.mode,
                loc: context.loc(),
                font: style,
                body: Box::new(ordgroup),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
