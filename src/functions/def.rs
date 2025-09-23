//! Implementation of macro definition functions (\def, \gdef, \edef, \xdef,
//! \let, \futurelet, \global, \long)
//!
//! This module provides the core macro definition functionality for KaTeX,
//! allowing users to define custom macros with parameters and replacement text.
//!
//! Note: Due to current architecture limitations, these functions are
//! registered but their full implementation requires mutable access to the
//! parser's token stream, which is not available through the current
//! FunctionContext API.

use phf::phf_map;

use crate::context::KatexContext;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeInternal};

use crate::macros::{MacroContextInterface as _, MacroDefinition, MacroExpansion};
use crate::types::{ParseError, Token};

/// Register all macro definition functions
pub fn define_def(ctx: &mut KatexContext) {
    // Prefix commands
    define_global(ctx);

    // Macro definition commands
    define_def_cmd(ctx);
    // Assignment commands
    define_let_cmd(ctx);
    define_futurelet_cmd(ctx);
}

const GLOBAL_MAP: phf::Map<&str, &str> = phf_map!(
    "\\global" => "\\global",
    "\\long" => "\\\\globallong",
    "\\\\globallong" => "\\\\globallong",
    "\\def" => "\\gdef",
    "\\gdef" => "\\gdef",
    "\\edef" => "\\xdef",
    "\\xdef" => "\\xdef",
    "\\let" => "\\\\globallet",
    "\\futurelet" => "\\\\globalfuture",
);

/// Register the \global prefix command
fn define_global(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Internal),
        names: &["\\global", "\\long", "\\\\globallong"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            context.parser.consume_spaces()?;
            let mut token = context.parser.fetch()?.clone();
            let replacement = GLOBAL_MAP.get(&token.text);
            if let Some(&repl) = replacement {
                context.parser.consume();
                if (context.func_name == "\\global" || context.func_name == "\\\\globallong")
                    && repl != token.text
                {
                    repl.clone_into(&mut token.text);
                }
                context.parser.gullet.push_token(token);
                let inner_node = context
                    .parser
                    .parse_function(context.break_on_token_text, None)?
                    .ok_or_else(|| ParseError::new("Expected function after prefix"))?;
                Ok(inner_node)
            } else {
                Err(ParseError::with_token(
                    format!("Invalid token after macro prefix: {}", token.text),
                    &token,
                ))
            }
        }),
        html_builder: None,
        mathml_builder: None,
    });
}

/// Register the \def command
fn define_def_cmd(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Internal),
        names: &["\\def", "\\gdef", "\\edef", "\\xdef"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            let name_tok = context.parser.gullet.pop_token()?;
            let name = name_tok.text.clone();
            if matches!(
                name.as_str(),
                "\\" | "{" | "}" | "$" | "&" | "#" | "^" | "_" | "EOF"
            ) {
                return Err(ParseError::with_token(
                    "Expected a control sequence",
                    &name_tok,
                ));
            }

            let mut num_args = 0usize;
            let mut delimiters: Vec<Vec<String>> = vec![Vec::new()];
            let mut insert: Option<Token> = None;

            loop {
                let next_text = context.parser.gullet.future_mut()?.text.clone();
                if next_text == "{" {
                    break;
                }
                let tok = context.parser.gullet.pop_token()?;
                if tok.text == "#" {
                    if context.parser.gullet.future_mut()?.text == "{" {
                        insert = Some(context.parser.gullet.future_mut()?);
                        delimiters[num_args].push("{".to_owned());
                        break;
                    }
                    let arg_tok = context.parser.gullet.pop_token()?;
                    if arg_tok.text.len() != 1
                        || !arg_tok
                            .text
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_digit() && c != '0')
                    {
                        return Err(ParseError::with_token(
                            format!("Invalid argument number: {}", arg_tok.text),
                            &arg_tok,
                        ));
                    }
                    let arg_num: usize = arg_tok
                        .text
                        .parse()
                        .map_err(|_| ParseError::with_token("Invalid number", &arg_tok))?;
                    if arg_num != num_args + 1 {
                        return Err(ParseError::with_token(
                            format!("Expected #{} but found #{}", num_args + 1, arg_num),
                            &arg_tok,
                        ));
                    }
                    num_args += 1;
                    delimiters.push(Vec::new());
                } else if tok.text == "EOF" {
                    return Err(ParseError::with_token("Expected a macro definition", &tok));
                } else {
                    delimiters[num_args].push(tok.text.clone());
                }
            }

            let arg = context.parser.gullet.consume_arg(None)?;
            let mut tokens = arg.tokens;
            if let Some(ins) = insert {
                tokens.insert(0, ins);
            }

            let global = matches!(context.func_name.as_str(), "\\gdef" | "\\xdef");
            if matches!(context.func_name.as_str(), "\\edef" | "\\xdef") {
                tokens = context.parser.gullet.expand_tokens(tokens)?;
                tokens.reverse();
            }

            let expansion = MacroExpansion {
                tokens,
                num_args,
                delimiters: Some(delimiters),
                unexpandable: None,
            };
            context.parser.gullet.macros_mut().set(
                &name,
                Some(MacroDefinition::Expansion(expansion)),
                global,
            );

            Ok(ParseNode::Internal(ParseNodeInternal {
                mode: context.parser.mode,
                loc: context.loc(),
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}

/// Register the \let command
fn define_let_cmd(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Internal),
        names: &["\\let", "\\\\globallet"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            let name_tok = context.parser.gullet.pop_token()?;
            let name = name_tok.text.clone();
            if matches!(
                name.as_str(),
                "\\" | "{" | "}" | "$" | "&" | "#" | "^" | "_" | "EOF"
            ) {
                return Err(ParseError::with_token(
                    "Expected a control sequence",
                    &name_tok,
                ));
            }

            context.parser.gullet.consume_spaces()?;

            let rhs_tok = {
                let tok = context.parser.gullet.pop_token()?;
                if tok.text == "=" {
                    let next_tok = context.parser.gullet.pop_token()?;
                    if next_tok.text == " " {
                        context.parser.gullet.pop_token()
                    } else {
                        Ok(next_tok)
                    }
                } else {
                    Ok(tok)
                }
            }?;

            let global = context.func_name == "\\\\globallet";

            let macro_def =
                if let Some(existing) = context.parser.gullet.macros().get(&rhs_tok.text) {
                    existing.clone()
                } else {
                    let mut tok = rhs_tok.clone();
                    tok.noexpand = Some(true);
                    let unexpandable = !context.parser.gullet.is_expandable(&rhs_tok.text);
                    MacroDefinition::Expansion(MacroExpansion {
                        tokens: vec![tok],
                        num_args: 0,
                        delimiters: None,
                        unexpandable: Some(unexpandable),
                    })
                };

            context
                .parser
                .gullet
                .macros_mut()
                .set(&name, Some(macro_def), global);

            Ok(ParseNode::Internal(ParseNodeInternal {
                mode: context.parser.mode,
                loc: context.loc(),
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}

/// Register the \futurelet command
fn define_futurelet_cmd(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Internal),
        names: &["\\futurelet", "\\\\globalfuture"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            primitive: true,
            ..Default::default()
        },
        handler: Some(|context: FunctionContext, _args, _opt_args| {
            let name_tok = context.parser.gullet.pop_token()?;
            let name = name_tok.text.clone();
            if matches!(
                name.as_str(),
                "\\" | "{" | "}" | "$" | "&" | "#" | "^" | "_" | "EOF"
            ) {
                return Err(ParseError::with_token(
                    "Expected a control sequence",
                    &name_tok,
                ));
            }

            let middle_tok = context.parser.gullet.pop_token()?;
            let tok = context.parser.gullet.pop_token()?;

            let global = context.func_name == "\\\\globalfuture";

            let macro_def = if let Some(existing) = context.parser.gullet.macros().get(&tok.text) {
                existing.clone()
            } else {
                let mut t = tok.clone();
                t.noexpand = Some(true);
                let unexpandable = !context.parser.gullet.is_expandable(&tok.text);
                MacroDefinition::Expansion(MacroExpansion {
                    tokens: vec![t],
                    num_args: 0,
                    delimiters: None,
                    unexpandable: Some(unexpandable),
                })
            };

            context
                .parser
                .gullet
                .macros_mut()
                .set(&name, Some(macro_def), global);

            context.parser.gullet.push_token(tok);
            context.parser.gullet.push_token(middle_tok);

            Ok(ParseNode::Internal(ParseNodeInternal {
                mode: context.parser.mode,
                loc: context.loc(),
            }))
        }),
        html_builder: None,
        mathml_builder: None,
    });
}
