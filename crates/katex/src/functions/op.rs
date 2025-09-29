//! Operator function implementations for KaTeX Rust
//!
//! This module handles various mathematical operators including big operators
//! like \sum, \prod, integrals, trigonometric functions, and limits.
//!
//! Migrated from KaTeX's op.js.

use crate::build_common::{
    VListElemAndShift, VListParam, make_span, make_symbol, make_v_list, mathsym, static_svg,
};
use crate::define_function::{FunctionDefSpec, FunctionPropSpec, ord_argument};
use crate::dom_tree::HtmlDomNode;
use crate::functions::utils::assemble_sup_sub;
use crate::mathml_tree::{self, MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeOp};
use crate::style::DISPLAY;
use crate::types::ClassList;
use crate::types::{CssProperty, Mode, ParseError, ParseErrorKind};
use crate::units::make_em;
use crate::{build_html, build_mathml};
use alloc::borrow::Cow;

/// HTML builder for op nodes
pub fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    // Operators are handled in the TeXbook pg. 443-444, rule 13(a).
    let (op_node, super_group, sub_group, has_limits) = match node {
        ParseNode::SupSub(supsub) => {
            // If we have limits, supsub will pass us its group to handle. Pull
            // out the superscript and subscript and set the group to the op in
            // its base.
            let sub_group = supsub.sub.as_deref();
            let super_group = supsub.sup.as_deref();
            if let Some(base) = supsub.base.as_ref()
                && let ParseNode::Op(op_node) = &**base
            {
                (op_node, super_group, sub_group, true)
            } else {
                return Err(ParseError::new(ParseErrorKind::ExpectedSupSubBaseNode {
                    node: NodeType::Op,
                }));
            }
        }
        ParseNode::Op(op_node) => (op_node, None, None, false),
        _ => {
            return Err(ParseError::new(ParseErrorKind::ExpectedNodeOrSupSub {
                node: NodeType::Op,
            }));
        }
    };

    let (symbol, name, body, suppress_base_shift, mode) = match op_node {
        ParseNodeOp::Symbol {
            name,
            suppress_base_shift,
            mode,
            symbol,
            ..
        } => (
            *symbol,
            Some(name.as_str()),
            vec![],
            *suppress_base_shift,
            *mode,
        ),
        ParseNodeOp::Body {
            body,
            suppress_base_shift,
            mode,
            ..
        } => (false, None, body.clone(), *suppress_base_shift, *mode),
    };

    let style = options.style;

    let large = if style.size == DISPLAY.size
        && let Some(name) = name
        && !no_successor(name)
    {
        // Most symbol operators get larger in displaystyle (rule 13)
        true
    } else {
        false
    };

    let mut base_shift = 0.0;
    let mut slant = 0.0;
    let mut base_shift_computed = false;

    let mut base = if let Some(name) = name {
        if symbol {
            let name_str: &str = name;
            // Create the symbol
            let font_name = if large {
                "Size2-Regular"
            } else {
                "Size1-Regular"
            };

            // No font glyphs yet, so use a glyph w/o the oval.
            // TODO: When font glyphs are available, delete this code.
            let (symbol_name, stash) = match name_str {
                "\\oiint" => ("\\iint", Some("oiint")),
                "\\oiiint" => ("\\iiint", Some("oiiint")),
                _ => (name_str, None),
            };

            let mut base_classes = ClassList::Const(&["mop", "op-symbol"]);
            if large {
                base_classes.push("large-op");
            } else {
                base_classes.push("small-op");
            }

            let symbol_base = make_symbol(
                ctx,
                symbol_name,
                font_name,
                Mode::Math,
                Some(options),
                base_classes,
            )?;

            if let Some(stash) = stash {
                // We're in \oiint or \oiiint. Overlay the oval.
                // TODO: When font glyphs are available, delete this code.
                if suppress_base_shift.is_none() {
                    base_shift = (symbol_base.height - symbol_base.depth) / 2.0
                        - options.font_metrics().axis_height;
                    slant = symbol_base.italic;
                    base_shift_computed = true;
                }

                let italic = symbol_base.italic;
                let oval = static_svg(
                    &format!("{}Size{}", stash, if large { "2" } else { "1" }),
                    options,
                )?;
                let mut base = make_v_list(
                    VListParam::IndividualShift {
                        children: vec![
                            VListElemAndShift::builder()
                                .elem(symbol_base.into())
                                .shift(0.0)
                                .build(),
                            VListElemAndShift::builder()
                                .elem(oval.into())
                                .shift(if large { 0.08 } else { 0.0 })
                                .build(),
                        ],
                    },
                    options,
                )?;
                base.classes.insert(0, "mop");
                base.italic = Some(italic);
                base.into()
            } else {
                symbol_base.into()
            }
        } else {
            // Text operator
            let mut output = Vec::new();
            for ch in name.chars().skip(1) {
                output.push(mathsym(ctx, &ch.to_string(), mode, options, ClassList::Empty)?.into());
            }
            make_span("mop", output, Some(options), None).into()
        }
    } else {
        // Compose the list
        let inner = build_html::build_expression(
            ctx,
            &body,
            options,
            build_html::GroupType::True,
            (None, None),
        )?;
        if inner.len() == 1 && matches!(inner[0], HtmlDomNode::Symbol(_)) {
            let mut symbol = inner[0].clone();
            if let HtmlDomNode::Symbol(ref mut sym) = symbol
                && let Some(class) = sym.classes.get_mut(0)
            {
                // replace old mclass
                *class = Cow::Borrowed("mop");
            }
            symbol
        } else {
            make_span("mop", inner, Some(options), None).into()
        }
    };

    if !base_shift_computed
        && (matches!(base, HtmlDomNode::Symbol(_))
            || name == Some("\\oiint")
            || name == Some("\\oiiint"))
        && suppress_base_shift.is_none()
        && let HtmlDomNode::Symbol(sym) = &base
    {
        base_shift = (sym.height - sym.depth) / 2.0 - options.font_metrics().axis_height;
        slant = sym.italic;
    }

    if has_limits {
        assemble_sup_sub(
            ctx,
            base,
            super_group,
            sub_group,
            options,
            style,
            slant,
            base_shift,
        )
    } else {
        if base_shift != 0.0
            && let Some(style) = base.style_mut()
        {
            style.insert(CssProperty::Position, "relative".to_owned());
            style.insert(CssProperty::Top, make_em(base_shift));
        }
        Ok(base)
    }
}

/// MathML builder for op nodes
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &crate::KatexContext,
) -> Result<MathDomNode, ParseError> {
    let ParseNode::Op(op_node) = node else {
        return Err(ParseError::new(ParseErrorKind::ExpectedNode {
            node: NodeType::Op,
        }));
    };

    let (symbol, name, body, parent_is_sup_sub) = match op_node {
        ParseNodeOp::Symbol {
            name,
            parent_is_sup_sub,
            ..
        } => (true, name.clone(), vec![], *parent_is_sup_sub),
        ParseNodeOp::Body {
            body,
            parent_is_sup_sub,
            ..
        } => (false, String::new(), body.clone(), *parent_is_sup_sub),
    };

    let node_result = if symbol {
        // Symbol
        let mut node = MathNode::builder()
            .node_type(MathNodeType::Mo)
            .children(vec![MathDomNode::Text(TextNode { text: name.clone() })])
            .build();

        if no_successor(&name) {
            node.attributes
                .insert("largeop".to_owned(), "false".to_owned());
        }

        MathDomNode::Math(node)
    } else if !body.is_empty() {
        // Operator with children
        let inner = build_mathml::build_expression(ctx, &body, options, None)?;
        MathDomNode::Math(
            MathNode::builder()
                .node_type(MathNodeType::Mo)
                .children(inner)
                .build(),
        )
    } else {
        // Text operator
        let text = if name.len() > 1 {
            name[1..].to_string()
        } else {
            name
        };

        let mi = MathNode::builder()
            .node_type(MathNodeType::Mi)
            .children(vec![MathDomNode::Text(TextNode { text })])
            .build();

        // Append an <mo>&ApplyFunction;</mo>
        let operator = MathNode::builder()
            .node_type(MathNodeType::Mo)
            .children(vec![MathDomNode::Text(TextNode {
                text: "\u{2061}".to_owned(), // &ApplyFunction;
            })])
            .build();

        if parent_is_sup_sub {
            MathDomNode::Math(
                MathNode::builder()
                    .node_type(MathNodeType::Mrow)
                    .children(vec![MathDomNode::Math(mi), MathDomNode::Math(operator)])
                    .build(),
            )
        } else {
            // MathML document fragment
            mathml_tree::make_fragment(vec![MathDomNode::Math(mi), MathDomNode::Math(operator)])
                .into()
        }
    };

    Ok(node_result)
}

/// Check if operator has no successor symbol
fn no_successor(name: &str) -> bool {
    matches!(name, "\\smallint")
}

/// Registers operator functions in the KaTeX context
pub fn define_op(ctx: &mut crate::KatexContext) {
    // Big operators
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Op),
        names: &[
            "\\coprod",
            "\\bigvee",
            "\\bigwedge",
            "\\biguplus",
            "\\bigcap",
            "\\bigcup",
            "\\intop",
            "\\prod",
            "\\sum",
            "\\bigotimes",
            "\\bigoplus",
            "\\bigodot",
            "\\bigsqcup",
            "\\smallint",
            "\u{220F}",
            "\u{2210}",
            "\u{2211}",
            "\u{22c0}",
            "\u{22c1}",
            "\u{22c2}",
            "\u{22c3}",
            "\u{2a00}",
            "\u{2a01}",
            "\u{2a02}",
            "\u{2a04}",
            "\u{2a06}",
        ],
        props: FunctionPropSpec {
            num_args: 0,
            ..Default::default()
        },
        handler: Some(|context, _args, _opt_args| {
            let mut f_name = context.func_name.to_owned();
            if f_name.len() == 1 {
                // Unicode character - map to LaTeX command
                f_name = match f_name.as_str() {
                    "\u{220F}" => "\\prod",
                    "\u{2210}" => "\\coprod",
                    "\u{2211}" => "\\sum",
                    "\u{22c0}" => "\\bigwedge",
                    "\u{22c1}" => "\\bigvee",
                    "\u{22c2}" => "\\bigcap",
                    "\u{22c3}" => "\\bigcup",
                    "\u{2a00}" => "\\bigodot",
                    "\u{2a01}" => "\\bigoplus",
                    "\u{2a02}" => "\\bigotimes",
                    "\u{2a04}" => "\\biguplus",
                    "\u{2a06}" => "\\bigsqcup",
                    _ => &f_name,
                }
                .to_owned();
            }

            Ok(ParseNode::Op(ParseNodeOp::Symbol {
                mode: context.parser.mode,
                loc: context.loc(),
                limits: true,
                always_handle_sup_sub: None,
                suppress_base_shift: None,
                parent_is_sup_sub: false,
                name: f_name,
                symbol: true,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // \mathop
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Op),
        names: &["\\mathop"],
        props: FunctionPropSpec {
            num_args: 1,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context, args, _opt_args| {
            let body = ord_argument(&args[0]);

            Ok(ParseNode::Op(ParseNodeOp::Body {
                mode: context.parser.mode,
                loc: context.loc(),
                limits: false,
                always_handle_sup_sub: None,
                suppress_base_shift: None,
                parent_is_sup_sub: false,
                body,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    // No limits, not symbols (trig functions, etc.)
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Op),
        names: &[
            "\\arcsin", "\\arccos", "\\arctan", "\\arctg", "\\arcctg", "\\arg", "\\ch", "\\cos",
            "\\cosec", "\\cosh", "\\cot", "\\cotg", "\\coth", "\\csc", "\\ctg", "\\cth", "\\deg",
            "\\dim", "\\exp", "\\hom", "\\ker", "\\lg", "\\ln", "\\log", "\\sec", "\\sin",
            "\\sinh", "\\sh", "\\tan", "\\tanh", "\\tg", "\\th",
        ],
        props: FunctionPropSpec {
            num_args: 0,
            ..Default::default()
        },
        handler: Some(|context, _args, _opt_args| {
            Ok(ParseNode::Op(ParseNodeOp::Symbol {
                mode: context.parser.mode,
                loc: context.loc(),
                limits: false,
                always_handle_sup_sub: None,
                suppress_base_shift: None,
                parent_is_sup_sub: false,
                name: context.func_name.to_owned(),
                symbol: false,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Op),
        names: &[
            "\\det", "\\gcd", "\\inf", "\\lim", "\\max", "\\min", "\\Pr", "\\sup",
        ],
        props: FunctionPropSpec {
            num_args: 0,
            ..Default::default()
        },
        handler: Some(|context, _args, _opt_args| {
            Ok(ParseNode::Op(ParseNodeOp::Symbol {
                mode: context.parser.mode,
                loc: context.loc(),
                limits: true,
                always_handle_sup_sub: None,
                suppress_base_shift: None,
                parent_is_sup_sub: false,
                name: context.func_name.to_owned(),
                symbol: false,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });

    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Op),
        names: &[
            "\\int", "\\iint", "\\iiint", "\\oint", "\\oiint", "\\oiiint", "\u{222b}", "\u{222c}",
            "\u{222d}", "\u{222e}", "\u{222f}", "\u{2230}",
        ],
        props: FunctionPropSpec {
            num_args: 0,
            ..Default::default()
        },
        handler: Some(|context, _args, _opt_args| {
            let mut f_name = context.func_name.to_owned();
            if f_name.len() == 1 {
                f_name = match f_name.as_str() {
                    "\u{222b}" => "\\int",
                    "\u{222c}" => "\\iint",
                    "\u{222d}" => "\\iiint",
                    "\u{222e}" => "\\oint",
                    "\u{222f}" => "\\oiint",
                    "\u{2230}" => "\\oiiint",
                    _ => &f_name,
                }
                .to_owned();
            }

            Ok(ParseNode::Op(ParseNodeOp::Symbol {
                mode: context.parser.mode,
                loc: context.loc(),
                limits: false,
                always_handle_sup_sub: None,
                suppress_base_shift: None,
                parent_is_sup_sub: false,
                name: f_name,
                symbol: true,
            }))
        }),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}
