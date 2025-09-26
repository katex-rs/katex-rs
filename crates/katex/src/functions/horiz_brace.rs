//! Horizontal brace function implementations for KaTeX Rust
//!
//! This module handles horizontal braces (\overbrace, \underbrace) in
//! mathematical expressions, migrated from KaTeX's horizBrace.js.

use crate::build_common::{VListChild, VListElem, VListParam, make_span, make_v_list};
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeHorizBrace};
use crate::stretchy::{math_ml_node, svg_span};
use crate::style::DISPLAY;
use crate::types::ParseError;
use crate::{KatexContext, build_html, build_mathml};

/// Registers horizontal brace functions in the KaTeX context
pub fn define_horiz_brace(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::HorizBrace),
        names: &["\\overbrace", "\\underbrace"],
        props: FunctionPropSpec {
            num_args: 1,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, args, _opt_args| {
            let base = args[0].clone();
            let is_over = context.func_name.starts_with("\\over");

            Ok(ParseNode::HorizBrace(ParseNodeHorizBrace {
                mode: context.parser.mode,
                loc: context.loc(),
                label: context.func_name,
                is_over,
                base: Box::new(base),
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for horizontal brace nodes
/// NOTE: Unlike most `htmlBuilder`s, this one handles not only "horizBrace",
/// but also "supsub" since a horizontal brace can affect super/subscripting.
pub fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    // Pull out the `ParseNode<"horizBrace">` if `grp` is a "supsub" node.
    let (group, sup_sub_group) = match node {
        ParseNode::SupSub(supsub) => {
            let base = supsub
                .base
                .as_ref()
                .ok_or_else(|| ParseError::new("Expected base in SupSub node"))?;
            let ParseNode::HorizBrace(group) = base.as_ref() else {
                return Err(ParseError::new("Expected HorizBrace node in SupSub base"));
            };

            let style = options.style;
            let sup_group = if let Some(sup) = &supsub.sup {
                let sup_options = options.having_style(style.sup());
                Some(build_html::build_group(
                    ctx,
                    sup,
                    &sup_options,
                    Some(options),
                )?)
            } else if let Some(sub) = &supsub.sub {
                let sub_options = options.having_style(style.sub());
                Some(build_html::build_group(
                    ctx,
                    sub,
                    &sub_options,
                    Some(options),
                )?)
            } else {
                None
            };

            (group, sup_group)
        }
        ParseNode::HorizBrace(hb) => (hb, None),
        _ => return Err(ParseError::new("Expected HorizBrace node or SupSub node")),
    };

    // Build the base group
    let body = build_html::build_group(
        ctx,
        &group.base,
        &options.having_base_style(Some(DISPLAY)),
        None,
    )?;

    // Create the stretchy element
    let brace_body = svg_span(&ParseNode::HorizBrace(group.clone()), options)?;

    // Generate the vlist, with the appropriate kerns
    let vlist = if group.is_over {
        make_v_list(
            VListParam::FirstBaseline {
                children: vec![
                    VListElem::builder().elem(body).build().into(),
                    VListChild::Kern(0.1.into()),
                    VListElem::builder()
                        .elem(brace_body)
                        .wrapper_classes(vec!["svg-align".to_owned()])
                        .build()
                        .into(),
                ],
            },
            options,
        )?
    } else {
        make_v_list(
            VListParam::Bottom {
                position_data: body.depth() + 0.1 + brace_body.height(),
                children: vec![
                    VListElem::builder()
                        .elem(brace_body)
                        .wrapper_classes(vec!["svg-align".to_owned()])
                        .build()
                        .into(),
                    VListChild::Kern(0.1.into()),
                    VListElem::builder().elem(body).build().into(),
                ],
            },
            options,
        )?
    };

    if let Some(sup_sub_group) = sup_sub_group {
        // To write the supsub, wrap the first vlist in another vlist:
        // They can't all go in the same vlist, because the note might be
        // wider than the equation. We want the equation to control the
        // brace width.

        let v_span = make_span(
            vec![
                "mord".to_owned(),
                if group.is_over { "mover" } else { "munder" }.to_owned(),
            ],
            vec![vlist.into()],
            Some(options),
            None,
        );

        if group.is_over {
            let vlist = make_v_list(
                VListParam::FirstBaseline {
                    children: vec![
                        VListElem::builder().elem(v_span.into()).build().into(),
                        VListChild::Kern(0.2.into()),
                        VListElem::builder().elem(sup_sub_group).build().into(),
                    ],
                },
                options,
            )?;
            Ok(make_span(
                vec!["mord".to_owned(), "mover".to_owned()],
                vec![vlist.into()],
                Some(options),
                None,
            )
            .into())
        } else {
            let vlist = make_v_list(
                VListParam::Bottom {
                    position_data: v_span.depth
                        + 0.2
                        + sup_sub_group.height()
                        + sup_sub_group.depth(),
                    children: vec![
                        VListElem::builder().elem(sup_sub_group).build().into(),
                        VListChild::Kern(0.2.into()),
                        VListElem::builder().elem(v_span.into()).build().into(),
                    ],
                },
                options,
            )?;
            Ok(make_span(
                vec!["mord".to_owned(), "munder".to_owned()],
                vec![vlist.into()],
                Some(options),
                None,
            )
            .into())
        }
    } else {
        Ok(make_span(
            vec![
                "mord".to_owned(),
                if group.is_over { "mover" } else { "munder" }.to_owned(),
            ],
            vec![vlist.into()],
            Some(options),
            None,
        )
        .into())
    }
}

/// MathML builder for horizontal brace nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::HorizBrace(group) = node else {
        return Err(ParseError::new("Expected HorizBrace node"));
    };

    let accent_node = math_ml_node(&group.label);
    let base_group = build_mathml::build_group(ctx, &group.base, options)?;

    let mut mover = MathNode::builder()
        .node_type(if group.is_over {
            MathNodeType::Mover
        } else {
            MathNodeType::Munder
        })
        .children(vec![base_group, MathDomNode::Math(accent_node)])
        .build();

    mover
        .attributes
        .insert("accent".to_owned(), "true".to_owned());

    Ok(MathDomNode::Math(mover))
}
