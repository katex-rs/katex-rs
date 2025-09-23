//! Sizing function implementations for KaTeX Rust
//!
//! This module handles font size adjustment commands in mathematical
//! expressions, migrated from KaTeX's sizing.js.

use crate::build_common::make_fragment;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeSizing};
use crate::types::ParseError;
use crate::units::make_em;
use crate::{build_html, build_mathml};

/// Size function names corresponding to KaTeX sizing commands
const SIZE_FUNCS: &[&str] = &[
    "\\tiny",
    "\\sixptsize",
    "\\scriptsize",
    "\\footnotesize",
    "\\small",
    "\\normalsize",
    "\\large",
    "\\Large",
    "\\LARGE",
    "\\huge",
    "\\Huge",
];

/// Creates a sizing group for HTML rendering
///
/// This function handles the sizing of expressions by applying size multipliers
/// and adjusting CSS classes for nested size changes.
pub fn sizing_group(
    ctx: &crate::KatexContext,
    value: &[ParseNode],
    options: &Options,
    base_options: &Options,
) -> Result<HtmlDomNode, ParseError> {
    let mut inner = build_html::build_expression(
        ctx,
        value,
        options,
        build_html::GroupType::False,
        (None, None),
    )?;
    let multiplier = options.size_multiplier / base_options.size_multiplier;

    // Add size-resetting classes to the inner list and set maxFontSize
    // manually. Handle nested size changes.
    for item in &mut inner {
        let Some(classes) = item.classes_mut() else {
            continue;
        };
        let pos = classes.iter().position(|c| c == "sizing");

        if let Some(pos) = pos {
            if classes.get(pos + 1) == Some(&format!("reset-size{}", options.size)) {
                // This is a nested size change: e.g., item is the "b" in
                // `\Huge a \small b`. Override the old size (the `reset-` class)
                // but not the new size.
                classes[pos + 1] = format!("reset-size{}", base_options.size);
            }
        } else {
            let sizing_classes = options.sizing_classes(base_options);
            classes.extend(sizing_classes);
        }

        if let Some(h) = item.height_mut() {
            *h *= multiplier;
        }
        if let Some(d) = item.depth_mut() {
            *d *= multiplier;
        }
    }

    Ok(make_fragment(&inner).into())
}

/// HTML builder for sizing nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Sizing(sizing_node) = node else {
        return Err(ParseError::new("Expected Sizing node"));
    };

    // Handle sizing operators like \Huge. Real TeX doesn't actually allow
    // these functions inside of math expressions, so we do some special
    // handling.
    let new_options = options.having_size(sizing_node.size);
    sizing_group(ctx, &sizing_node.body, &new_options, options)
}

/// MathML builder for sizing nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Sizing(sizing_node) = node else {
        return Err(ParseError::new("Expected Sizing node"));
    };

    let new_options = options.having_size(sizing_node.size);
    let inner = build_mathml::build_expression(ctx, &sizing_node.body, &new_options, None)?;

    let mut node = MathNode::builder()
        .node_type(MathNodeType::Mstyle)
        .children(inner)
        .build();

    // TODO(emily): This doesn't produce the correct size for nested size
    // changes, because we don't keep state of what style we're currently
    // in, so we can't reset the size to normal before changing it.  Now
    // that we're passing an options parameter we should be able to fix
    // this.
    node.set_attribute("mathsize", make_em(new_options.size_multiplier));

    Ok(MathDomNode::Math(node))
}

/// Registers sizing functions in the KaTeX context
pub fn define_sizing(ctx: &mut crate::KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Sizing),
        names: SIZE_FUNCS,
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            if !args.is_empty() {
                return Err(ParseError::new("Sizing functions take no arguments"));
            }

            let body = context
                .parser
                .parse_expression(false, context.break_on_token_text)?;

            Ok(ParseNode::Sizing(ParseNodeSizing {
                mode: context.parser.mode,
                loc: context.loc(),
                // Figure out what size to use based on the list of functions above
                size: SIZE_FUNCS
                    .iter()
                    .position(|&s| s == context.func_name)
                    .unwrap_or(0)
                    + 1,
                body,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
