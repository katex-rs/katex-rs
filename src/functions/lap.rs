//! Lap function implementations for KaTeX Rust
//!
//! This module handles horizontal overlap functions (\mathllap, \mathrlap,
//! \mathclap) migrated from KaTeX's lap.js.

use crate::build_common::make_span;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{LapAlignment, NodeType, ParseNode, ParseNodeLap};
use crate::types::{ArgType, CssProperty, ParseError};
use crate::units::make_em;
use crate::{KatexContext, build_html, build_mathml};

/// Registers lap functions in the KaTeX context
pub fn define_lap(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Lap),
        names: &["\\mathllap", "\\mathrlap", "\\mathclap"],
        props: FunctionPropSpec {
            num_args: 1,
            allowed_in_text: true,
            arg_types: Some(vec![ArgType::Primitive]),
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            if args.len() != 1 {
                return Err(ParseError::new("Lap functions require exactly 1 argument"));
            }

            let body = args[0].clone();
            let alignment = match context.func_name.as_str() {
                "\\mathllap" => LapAlignment::Left,
                "\\mathrlap" => LapAlignment::Right,
                "\\mathclap" => LapAlignment::Center,
                _ => unreachable!(),
            };

            Ok(ParseNode::Lap(ParseNodeLap {
                mode: context.parser.mode,
                loc: context.loc(),
                alignment,
                body: Box::new(body),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for lap nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Lap(lap_node) = node else {
        return Err(ParseError::new("Expected Lap node"));
    };

    // Build the base group
    let body = build_html::build_group(
        ctx,
        &lap_node.body,
        options,
        Some(&options.having_cramped_style()),
    )?;

    // Create inner span
    let inner = if lap_node.alignment == LapAlignment::Center {
        // For clap, wrap in inner span for CSS centering
        let inner_span = make_span(vec!["inner".to_owned()], vec![body], Some(options), None);
        make_span(vec![], vec![inner_span.into()], Some(options), None)
    } else {
        make_span(vec!["inner".to_owned()], vec![body], Some(options), None)
    };

    // Create fix span
    let fix = make_span(vec!["fix".to_owned()], vec![], None, None);

    // Create main lap span
    let mut lap_span = make_span(
        vec![lap_node.alignment.as_ref().to_owned()],
        vec![inner.into(), fix.into()],
        Some(options),
        None,
    );

    // Set height for strut
    let mut strut = make_span(vec!["strut".to_owned()], vec![], None, None);
    let strut_height = lap_span.height + lap_span.depth;
    strut
        .style
        .insert(CssProperty::Height, make_em(strut_height));
    if lap_span.depth > 0.0 {
        strut
            .style
            .insert(CssProperty::VerticalAlign, make_em(-lap_span.depth));
    }

    // Add strut to children
    lap_span.children.insert(0, strut.into());

    // Wrap in thinbox and vbox
    let thinbox = make_span(
        vec!["thinbox".to_owned()],
        vec![lap_span.into()],
        Some(options),
        None,
    );
    let result = make_span(
        vec!["mord".to_owned(), "vbox".to_owned()],
        vec![thinbox.into()],
        Some(options),
        None,
    );

    Ok(result.into())
}

/// MathML builder for lap nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Lap(lap_node) = node else {
        return Err(ParseError::new("Expected Lap node"));
    };

    // Build the base group
    let base_group = build_mathml::build_group(ctx, &lap_node.body, options)?;

    // Create mpadded element
    let mut mpadded = MathNode::builder()
        .node_type(MathNodeType::Mpadded)
        .children(vec![base_group])
        .build();

    // Set attributes based on alignment
    if lap_node.alignment != LapAlignment::Right {
        let offset = if lap_node.alignment == LapAlignment::Left {
            "-1"
        } else {
            "-0.5"
        };
        mpadded.set_attribute("lspace", format!("{offset}width"));
    }
    mpadded.set_attribute("width", "0px");

    Ok(MathDomNode::Math(mpadded))
}
