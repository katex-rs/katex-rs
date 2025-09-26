//! Square root function implementation for KaTeX Rust
//!
//! This module handles square root and nth root expressions,
//! migrated from KaTeX's sqrt.js.

use crate::build_common::{
    self, VListChild, VListElem, VListKern, VListParam, make_span, make_v_list,
};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec};
use crate::delimiter::make_sqrt_image;
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeSqrt};
use crate::style::{SCRIPTSCRIPT, TEXT};
use crate::types::{CssProperty, ParseError, ParseErrorKind};
use crate::units::make_em;
use crate::{KatexContext, build_html, build_mathml};

/// Registers the `\sqrt` function in the KaTeX context.
pub fn define_sqrt(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Sqrt),
        names: &["\\sqrt"],
        props: FunctionPropSpec {
            num_args: 1,
            num_optional_args: 1,
            ..Default::default()
        },
        handler: Some(|context, args, opt_args| {
            let body = args[0].clone();
            let index = opt_args[0].clone();
            Ok(ParseNode::Sqrt(Box::new(ParseNodeSqrt {
                mode: context.parser.mode,
                loc: context.loc(),
                body,
                index,
            })))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for sqrt nodes
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Sqrt(sqrt_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Sqrt,
        }));
    };

    // Square roots are handled in the TeXbook pg. 443, Rule 11.

    // First, we do the same steps as in overline to build the inner group
    // and line
    let mut inner =
        build_html::build_group(ctx, &sqrt_node.body, &options.having_cramped_style(), None)?;
    if let Some(height) = inner.height_mut()
        && *height == 0.0
    {
        *height = options.font_metrics().x_height;
    }

    // Some groups can return document fragments. Handle those by wrapping
    // them in a span.
    inner = build_common::wrap_fragment(inner, options);

    // Calculate the minimum size for the \surd delimiter
    let theta = options.font_metrics().default_rule_thickness;

    let phi = if options.style.id < TEXT.id {
        options.font_metrics().x_height
    } else {
        theta
    };

    // Calculate the clearance between the body and line
    let line_clearance = theta + phi / 4.0;

    let min_delimiter_height = inner.height() + inner.depth() + line_clearance + theta;

    // Create a sqrt SVG of the required minimum size
    let sqrt_result = make_sqrt_image(ctx, min_delimiter_height, options)?;

    let delim_depth = sqrt_result.span.height - sqrt_result.rule_width;

    // Adjust the clearance based on the delimiter size
    let line_clearance = if delim_depth > inner.height() + inner.depth() + line_clearance {
        (line_clearance + delim_depth - inner.height() - inner.depth()) / 2.0
    } else {
        line_clearance
    };

    // Shift the sqrt image
    let img_shift =
        sqrt_result.span.height - inner.height() - line_clearance - sqrt_result.rule_width;

    // Apply padding to inner element
    if let HtmlDomNode::DomSpan(ref mut span) = inner {
        span.style
            .insert(CssProperty::PaddingLeft, make_em(sqrt_result.advance_width));
    }

    let inner_height = inner.height();

    // Overlay the image and the argument.
    let body = make_v_list(
        VListParam::FirstBaseline {
            children: vec![
                VListElem::builder()
                    .elem(inner)
                    .wrapper_classes(vec!["svg-align".to_owned()])
                    .build()
                    .into(),
                VListChild::Kern(VListKern {
                    size: -(inner_height + img_shift),
                }),
                VListElem::builder()
                    .elem(HtmlDomNode::DomSpan(sqrt_result.span))
                    .build()
                    .into(),
                VListChild::Kern(VListKern {
                    size: sqrt_result.rule_width,
                }),
            ],
        },
        options,
    )?;

    let Some(sqrt_index) = &sqrt_node.index else {
        return Ok(make_span(
            vec!["mord".to_owned(), "sqrt".to_owned()],
            vec![body.into()],
            Some(options),
            None,
        )
        .into());
    };

    // Handle the optional root index

    // The index is always in scriptscript style
    let new_options = options.having_style(SCRIPTSCRIPT);
    let rootm = build_html::build_group(ctx, sqrt_index, &new_options, Some(options))?;

    // The amount the index is shifted by. This is taken from the TeX
    // source, in the definition of `\r@@t`.
    let to_shift = 0.6 * (body.height - body.depth);

    // Build a VList with the superscript shifted up correctly
    let root_v_list = make_v_list(
        VListParam::Shift {
            position_data: -to_shift,
            children: vec![VListElem::builder().elem(rootm).build().into()],
        },
        options,
    )?;

    // Add a class surrounding it so we can add on the appropriate
    // kerning
    let root_v_list_wrap = make_span(
        vec!["root".to_owned()],
        vec![root_v_list.into()],
        Some(options),
        None,
    );

    Ok(make_span(
        vec!["mord".to_owned(), "sqrt".to_owned()],
        vec![root_v_list_wrap.into(), body.into()],
        Some(options),
        None,
    )
    .into())
}

/// MathML builder for sqrt nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Sqrt(sqrt_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Sqrt,
        }));
    };

    let body_group = build_mathml::build_group(ctx, &sqrt_node.body, options)?;

    if let Some(index) = &sqrt_node.index {
        let index_group = build_mathml::build_group(ctx, index, options)?;
        Ok(MathNode::builder()
            .node_type(MathNodeType::Mroot)
            .children(vec![body_group, index_group])
            .build()
            .into())
    } else {
        Ok(MathNode::builder()
            .node_type(MathNodeType::Msqrt)
            .children(vec![body_group])
            .build()
            .into())
    }
}
