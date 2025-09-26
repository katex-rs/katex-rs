//! Smash function implementation for KaTeX Rust
//!
//! This module handles the `\smash` command, which removes vertical space
//! from mathematical expressions for alignment purposes.
//!
//! Migrated from KaTeX's smash.js.

use crate::namespace::KeyMap;

use crate::KatexContext;
use crate::build_common::{VListElem, VListParam, make_span, make_v_list};
use crate::build_html;
use crate::build_mathml;
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeSmash};
use crate::types::{ParseError, ParseErrorKind};

/// Registers the `\smash` function in the KaTeX context.
///
/// The `\smash` command removes vertical space from mathematical expressions.
/// It can optionally take a \[tb\] argument to specify whether to smash the top
/// (t), bottom (b), or both (default).
///
/// # LaTeX Syntax
///
/// ```latex
/// \smash{x}        % Smash both height and depth
/// \smash[t]{y}     % Smash only height (top)
/// \smash[b]{z}     % Smash only depth (bottom)
/// ```
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
pub fn define_smash(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Smash),
        names: &["\\smash"],
        props: FunctionPropSpec {
            num_args: 1,
            num_optional_args: 1,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context, args, opt_args| {
            let mut smash_height = false;
            let mut smash_depth = false;

            // Handle optional [tb] argument
            if let Some(opt_arg) = &opt_args[0] {
                if let AnyParseNode::OrdGroup(ord_group) = opt_arg {
                    for node in &ord_group.body {
                        match node.text() {
                            Some("t") => smash_height = true,
                            Some("b") => smash_depth = true,
                            _ => {
                                // Invalid character, reset both to false
                                smash_height = false;
                                smash_depth = false;
                                break;
                            }
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        ParseErrorKind::OptionalSmashArgumentMustBeOrdGroup,
                    ));
                }
            } else {
                // No optional argument, smash both
                smash_height = true;
                smash_depth = true;
            }

            let body = Box::new(args[0].clone());

            Ok(ParseNode::Smash(ParseNodeSmash {
                mode: context.parser.mode,
                loc: context.loc(),
                body,
                smash_height,
                smash_depth,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for smash nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Smash(smash_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Smash,
        }));
    };

    // Build the base group
    let body = build_html::build_group(
        ctx,
        &smash_node.body,
        options,
        Some(&options.having_cramped_style()),
    )?;

    let mut node = make_span(vec![], vec![body], None, None);

    if !smash_node.smash_height && !smash_node.smash_depth {
        return Ok(node.into());
    }

    // Set height and depth to 0 as specified
    if smash_node.smash_height {
        node.height = 0f64;
        // Reset children heights if they exist
        for child in &mut node.children {
            if let Some(height) = child.height_mut() {
                *height = 0f64;
            }
        }
    }

    if smash_node.smash_depth {
        node.depth = 0f64;
        // Reset children depths if they exist
        for child in &mut node.children {
            if let Some(depth) = child.depth_mut() {
                *depth = 0f64;
            }
        }
    }

    // Create a VList to handle the smashed content
    let smashed_node = make_v_list(
        VListParam::FirstBaseline {
            children: vec![VListElem::builder().elem(node.into()).build().into()],
        },
        options,
    )?;

    // Return as mord (ordinary symbol) for spacing
    Ok(make_span(
        vec!["mord".to_owned()],
        vec![smashed_node.into()],
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for smash nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Smash(smash_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Smash,
        }));
    };

    // Build the base group
    let body = build_mathml::build_group(ctx, &smash_node.body, options)?;

    // Create mpadded element for MathML
    let mut attributes = KeyMap::default();

    if smash_node.smash_height {
        attributes.insert("height".to_owned(), "0px".to_owned());
    }

    if smash_node.smash_depth {
        attributes.insert("depth".to_owned(), "0px".to_owned());
    }

    let mpadded = MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .children(vec![body])
        .attributes(attributes)
        .build();

    Ok(MathDomNode::Math(mpadded))
}
