//! Delimiter sizing function implementations for KaTeX Rust
//!
//! This module handles delimiter sizing commands in mathematical expressions,
//! migrated from KaTeX's delimsizing.js.

use crate::build_common::make_span;
use crate::build_html::DomType;
use crate::build_mathml::{make_row, make_text};
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::delimiter::{left_right_delim, size_to_max_height, sized_delim};
use crate::dom_tree::HtmlDomNode;
use crate::macros::MacroContextInterface as _;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNode, ParseNodeDelimsizing, ParseNodeLeftRight,
    ParseNodeLeftRightRight, ParseNodeMiddle, check_symbol_node_type,
};
use crate::types::{ArgType, ParseError, ParseErrorKind};
use crate::units::make_em;
use crate::{ClassList, KatexContext, build_html, build_mathml};

use phf::phf_map;

/// Delimiter size mappings for sizing commands
const DELIMITER_SIZES: phf::Map<&'static str, (DomType, u8)> = phf_map! {
    "\\bigl" => (DomType::Mopen, 1),
    "\\Bigl" => (DomType::Mopen, 2),
    "\\biggl" => (DomType::Mopen, 3),
    "\\Biggl" => (DomType::Mopen, 4),
    "\\bigr" => (DomType::Mclose, 1),
    "\\Bigr" => (DomType::Mclose, 2),
    "\\biggr" => (DomType::Mclose, 3),
    "\\Biggr" => (DomType::Mclose, 4),
    "\\bigm" => (DomType::Mrel, 1),
    "\\Bigm" => (DomType::Mrel, 2),
    "\\biggm" => (DomType::Mrel, 3),
    "\\Biggm" => (DomType::Mrel, 4),
    "\\big" => (DomType::Mord, 1),
    "\\Big" => (DomType::Mord, 2),
    "\\bigg" => (DomType::Mord, 3),
    "\\Bigg" => (DomType::Mord, 4),
};

/// Supported delimiters
static DELIMITERS: phf::Set<&'static str> = phf::phf_set! {
    "(", "\\lparen", ")", "\\rparen",
    "[", "\\lbrack", "]", "\\rbrack",
    "\\{", "\\lbrace", "\\}", "\\rbrace",
    "\\lfloor", "\\rfloor", "\u{230a}", "\u{230b}",
    "\\lceil", "\\rceil", "\u{2308}", "\u{2309}",
    "<", ">", "\\langle", "\u{27e8}", "\\rangle", "\u{27e9}", "\\lt", "\\gt",
    "\\lvert", "\\rvert", "\\lVert", "\\rVert",
    "\\lgroup", "\\rgroup", "\u{27ee}", "\u{27ef}",
    "\\lmoustache", "\\rmoustache", "\u{23b0}", "\u{23b1}",
    "/", "\\backslash",
    "|", "\\vert", "\\|", "\\Vert",
    "\\uparrow", "\\Uparrow",
    "\\downarrow", "\\Downarrow",
    "\\updownarrow", "\\Updownarrow",
    ".",
};

/// Validates delimiter arguments
fn check_delimiter(
    delim: Option<&AnyParseNode>,
    context: &FunctionContext<'_, '_, '_>,
) -> Result<String, ParseError> {
    let sym_delim = check_symbol_node_type(delim);
    if let Some(node_type) = sym_delim {
        if let Some(node) = delim
            && let Some(text) = node.text()
            && DELIMITERS.contains(text)
        {
            return Ok(text.to_owned());
        }
        return Err(ParseError::new(ParseErrorKind::InvalidDelimiterAfter {
            delimiter: node_type.to_string(),
            function: context.func_name.to_owned(),
        }));
    }
    Err(ParseError::new(ParseErrorKind::InvalidDelimiterTypeAfter {
        function: context.func_name.to_owned(),
    }))
}

/// Registers delimsizing functions in the KaTeX context
pub fn define_delimsizing(ctx: &mut KatexContext) {
    let names: Vec<&str> = DELIMITER_SIZES.keys().copied().collect();

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Delimsizing),
        names: &names,
        props: FunctionPropSpec {
            num_args: 1,
            arg_types: Some(vec![ArgType::Primitive]),
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let delim_text = check_delimiter(args.first(), &context)?;

            let (mclass, size) = DELIMITER_SIZES[context.func_name];

            Ok(ParseNode::Delimsizing(ParseNodeDelimsizing {
                mode: context.parser.mode,
                loc: context.loc(),
                size,
                mclass,
                delim: delim_text,
            }))
        }),
        html_builder: Some(delimsizing_html_builder),
        mathml_builder: Some(delimsizing_mathml_builder),
    });
}

/// Registers leftright functions in the KaTeX context
pub fn define_leftright(ctx: &mut KatexContext) {
    // Left delimiter
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::LeftRight),
        names: &["\\left"],
        props: FunctionPropSpec {
            num_args: 1,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let delim_text = check_delimiter(args.first(), &context)?;

            let loc = context.loc();
            let parser = context.parser;
            parser.leftright_depth += 1.0;
            let body = parser.parse_expression(false, None)?;
            parser.leftright_depth -= 1.0;

            parser.expect("\\right", false)?;
            let Some(AnyParseNode::LeftRightRight(right_node)) =
                parser.parse_function(None, None)?
            else {
                return Err(ParseError::new(ParseErrorKind::ExpectedRightAfterLeft));
            };

            Ok(ParseNode::LeftRight(ParseNodeLeftRight {
                mode: parser.mode,
                loc,
                body,
                left: delim_text,
                right: right_node.delim.clone(),
                right_color: right_node.color,
            }))
        }),
        html_builder: Some(leftright_html_builder),
        mathml_builder: Some(leftright_mathml_builder),
    });

    // Right delimiter
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::LeftRightRight),
        names: &["\\right"],
        props: FunctionPropSpec {
            num_args: 1,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let delim_text = check_delimiter(args.first(), &context)?;

            let color = context.parser.gullet.macros_mut().get("\\current@color");
            let color_str = if let Some(color_val) = color {
                if let Some(s) = color_val.as_str() {
                    Some(s.to_owned())
                } else {
                    return Err(ParseError::new(ParseErrorKind::CurrentColorMustBeString));
                }
            } else {
                None
            };

            Ok(ParseNode::LeftRightRight(ParseNodeLeftRightRight {
                mode: context.parser.mode,
                loc: context.loc(),
                delim: delim_text,
                color: color_str,
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}

/// Registers middle delimiter functions in the KaTeX context
pub fn define_middle(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Middle),
        names: &["\\middle"],
        props: FunctionPropSpec {
            num_args: 1,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            if context.parser.leftright_depth == 0.0 {
                return Err(ParseError::new(ParseErrorKind::MiddleWithoutPrecedingLeft));
            }

            let delim_text = check_delimiter(args.first(), &context)?;

            Ok(ParseNode::Middle(ParseNodeMiddle {
                mode: context.parser.mode,
                loc: context.loc(),
                delim: delim_text,
            }))
        }),
        html_builder: Some(middle_html_builder),
        mathml_builder: Some(middle_mathml_builder),
    });
}

/// HTML builder for delimsizing nodes
fn delimsizing_html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Delimsizing(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Delimsizing,
        }));
    };

    if group.delim == "." {
        // Empty delimiters still count as elements
        return Ok(make_span(group.mclass.as_str(), vec![], None, None).into());
    }

    // Use sized_delim to generate the delimiter
    Ok(sized_delim(
        ctx,
        &group.delim,
        usize::from(group.size),
        options,
        group.mode,
        group.mclass.as_str(),
    )?
    .into())
}

/// MathML builder for delimsizing nodes
fn delimsizing_mathml_builder(
    node: &ParseNode,
    _options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Delimsizing(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Delimsizing,
        }));
    };

    let children = if group.delim == "." {
        vec![]
    } else {
        vec![make_text(&group.delim, group.mode, None, &ctx.symbols).into()]
    };

    let mut node = MathNode::builder()
        .node_type(MathNodeType::Mo)
        .children(children)
        .build();

    if group.mclass == DomType::Mopen || group.mclass == DomType::Mclose {
        node.set_attribute("fence", "true");
    } else {
        node.set_attribute("fence", "false");
    }

    node.set_attribute("stretchy", "true");

    // Use delimiter size mapping
    let size_em = make_em(size_to_max_height(group.size));

    node.set_attribute("minsize", size_em.clone());
    node.set_attribute("maxsize", size_em);

    Ok(MathDomNode::Math(node))
}

/// HTML builder for leftright nodes
fn leftright_html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::LeftRight(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::LeftRight,
        }));
    };

    // Build the inner expression
    let inner = build_html::build_expression(
        ctx,
        &group.body,
        options,
        build_html::GroupType::True,
        (Some(DomType::Mopen), Some(DomType::Mclose)),
    )?;

    let mut inner_height: f64 = 0.0;
    let mut inner_depth: f64 = 0.0;
    let mut had_middle = false;

    // Calculate height and depth
    for item in &inner {
        if let HtmlDomNode::DomSpan(item) = item
            && item.is_middle.is_some()
        {
            had_middle = true;
        } else {
            inner_height = inner_height.max(item.height());
            inner_depth = inner_depth.max(item.depth());
        }
    }

    // Scale down inner size
    let scale = options.size_multiplier;
    inner_height *= scale;
    inner_depth *= scale;

    let mut inner_modified = inner;

    // Handle left delimiter
    let left_delim = if group.left == "." {
        build_html::make_null_delimiter(options, "mopen")
    } else {
        left_right_delim(
            ctx,
            &group.left,
            inner_height,
            inner_depth,
            options,
            group.mode,
            "mopen",
        )?
    };
    inner_modified.insert(0, left_delim.into());

    // Handle middle delimiters
    if had_middle {
        for i in 1..inner_modified.len() {
            if let HtmlDomNode::DomSpan(inner_modified) = &mut inner_modified[i]
                && let Some((delim, options)) = &inner_modified.is_middle
            {
                *inner_modified = left_right_delim(
                    ctx,
                    delim,
                    inner_height,
                    inner_depth,
                    options,
                    group.mode,
                    vec![],
                )?;
            }
        }
    }

    // Handle right delimiter
    let right_delim = if group.right == "." {
        build_html::make_null_delimiter(options, "mclose")
    } else {
        let mut right_options = options;
        let maybe_option;
        if let Some(color) = &group.right_color {
            maybe_option = options.with_color(color.clone());
            right_options = &maybe_option;
        }
        left_right_delim(
            ctx,
            &group.right,
            inner_height,
            inner_depth,
            right_options,
            group.mode,
            "mclose",
        )?
    };
    inner_modified.push(right_delim.into());

    Ok(make_span("minner", inner_modified, Some(options), None).into())
}

/// MathML builder for leftright nodes
fn leftright_mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::LeftRight(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::LeftRight,
        }));
    };

    let inner = build_mathml::build_expression(ctx, &group.body, options, None)?;

    let mut children = vec![];

    // Add left delimiter
    if group.left != "." {
        let mut left_node = MathNode::builder()
            .node_type(MathNodeType::Mo)
            .children(vec![
                make_text(&group.left, group.mode, None, &ctx.symbols).into(),
            ])
            .build();
        left_node.set_attribute("fence", "true");
        children.push(MathDomNode::Math(left_node));
    }

    children.extend(inner);

    // Add right delimiter
    if group.right != "." {
        let mut right_node = MathNode::builder()
            .node_type(MathNodeType::Mo)
            .children(vec![
                make_text(&group.right, group.mode, None, &ctx.symbols).into(),
            ])
            .build();
        right_node.set_attribute("fence", "true");

        if let Some(color) = &group.right_color {
            right_node.set_attribute("mathcolor", color);
        }

        children.push(MathDomNode::Math(right_node));
    }

    Ok(make_row(children))
}

/// HTML builder for middle nodes
fn middle_html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    let ParseNode::Middle(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Middle,
        }));
    };

    let mut middle_delim = if group.delim == "." {
        build_html::make_null_delimiter(options, ClassList::Empty)
    } else {
        sized_delim(ctx, &group.delim, 1, options, group.mode, vec![])?
    };

    // Mark as middle delimiter
    middle_delim.is_middle = Some((group.delim.clone(), options.clone()));

    Ok(middle_delim.into())
}

/// MathML builder for middle nodes
fn middle_mathml_builder(
    node: &ParseNode,
    _options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Middle(group) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Middle,
        }));
    };

    // Firefox compatibility: use plain "|" instead of "\vert"
    let text = if group.delim == "\\vert" || group.delim == "|" {
        "|"
    } else {
        &group.delim
    };

    let text_node = make_text(text, group.mode, None, &ctx.symbols);
    let mut middle_node = MathNode::builder()
        .node_type(MathNodeType::Mo)
        .children(vec![text_node.into()])
        .build();

    middle_node.set_attribute("fence", "true");
    middle_node.set_attribute("lspace", "0.05em");
    middle_node.set_attribute("rspace", "0.05em");

    Ok(MathDomNode::Math(middle_node))
}
