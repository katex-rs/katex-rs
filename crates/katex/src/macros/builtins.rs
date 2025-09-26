//! Built-in macro definitions for KaTeX
//!
//! This module contains all the built-in macros that are available by default,
//! equivalent to KaTeX's macros.js file.

use alloc::sync::Arc;
#[cfg(not(feature = "wasm"))]
use std::io::{self, Write as _};

use crate::{
    ParseError,
    font_metrics_data::MAIN_REGULAR_METRICS,
    macros::{MacroContextInterface, MacroDefinition, MacroExpansion, MacroExpansionResult},
    symbols::{Atom, Group},
    types::{Mode, ParseErrorKind, TokenText},
    units::make_em,
};
use phf::{phf_map, phf_set};

const DOTS_TYPE: phf::Map<&'static str, &'static str> = phf_map! {
    "," => "\\dotsc",
    "\\not" => "\\dotsb",
    "\\DOTSB" => "\\dotsb",
    "\\coprod" => "\\dotsb",
    "\\bigvee" => "\\dotsb",
    "\\bigwedge" => "\\dotsb",
    "\\biguplus" => "\\dotsb",
    "\\bigcap" => "\\dotsb",
    "\\bigcup" => "\\dotsb",
    "\\prod" => "\\dotsb",
    "\\sum" => "\\dotsb",
    "\\bigotimes" => "\\dotsb",
    "\\bigoplus" => "\\dotsb",
    "\\bigodot" => "\\dotsb",
    "\\bigsqcup" => "\\dotsb",
    "\\And" => "\\dotsb",
    "\\longrightarrow" => "\\dotsb",
    "\\Longrightarrow" => "\\dotsb",
    "\\longleftarrow" => "\\dotsb",
    "\\Longleftarrow" => "\\dotsb",
    "\\longleftrightarrow" => "\\dotsb",
    "\\Longleftrightarrow" => "\\dotsb",
    "\\mapsto" => "\\dotsb",
    "\\longmapsto" => "\\dotsb",
    "\\hookrightarrow" => "\\dotsb",
    "\\doteq" => "\\dotsb",
    "\\mathbin" => "\\dotsb",
    "\\mathrel" => "\\dotsb",
    "\\relbar" => "\\dotsb",
    "\\Relbar" => "\\dotsb",
    "\\xrightarrow" => "\\dotsb",
    "\\xleftarrow" => "\\dotsb",
    "\\DOTSI" => "\\dotsi",
    "\\int" => "\\dotsi",
    "\\oint" => "\\dotsi",
    "\\iint" => "\\dotsi",
    "\\iiint" => "\\dotsi",
    "\\iiiint" => "\\dotsi",
    "\\idotsint" => "\\dotsi",
    "\\DOTSX" => "\\dotsx",
};

const IS_SPACE_AFTER_DOTS: phf::Set<&'static str> = phf_set! {
    ")",
    "]",
    "\\rbrack",
    "\\}",
    "\\rbrace",
    "\\rangle",
    "\\rceil",
    "\\rfloor",
    "\\rgroup",
    "\\rmoustache",
    "\\right",
    "\\bigr",
    "\\biggr",
    "\\Bigr",
    "\\Biggr",
    "$",
    ";",
    ".",
    ",",
};

fn calculate_latex_raise_a() -> String {
    let t_metrics = MAIN_REGULAR_METRICS.get(&('T' as u32));
    let a_metrics = MAIN_REGULAR_METRICS.get(&('A' as u32));

    if let (Some(t), Some(a)) = (t_metrics, a_metrics) {
        let raise_value = 0.7f64.mul_add(-a.height, t.height);
        make_em(raise_value)
    } else {
        "0.2em".to_owned() // fallback value
    }
}

fn braket_helper(
    context: &mut dyn MacroContextInterface,
    one: bool,
) -> Result<MacroExpansionResult, ParseError> {
    let left = context.consume_arg(None)?.tokens;
    let middle = context.consume_arg(None)?.tokens;
    let middle_double = context.consume_arg(None)?.tokens;
    let right = context.consume_arg(None)?.tokens;

    // Save the current definitions of | and \|
    let old_middle = context.macros().get("|").cloned();
    let old_middle_double = context.macros().get("\\|").cloned();
    // Don't move mid_double into the clojure
    let mid_double_is_empty = middle_double.is_empty();
    context.begin_group();

    // Define the midMacro function that handles | and \| replacement
    let mid_macro = Arc::new(
        move |context: &mut dyn MacroContextInterface,
              double: bool|
              -> Result<MacroExpansionResult, ParseError> {
            if one {
                // Only modify the first instance of | or \|
                context.macros_mut().set("|", old_middle.clone(), false);
                if !mid_double_is_empty {
                    context
                        .macros_mut()
                        .set("\\|", old_middle_double.clone(), false);
                }
            }

            let mut doubled = double;
            if !double && !middle_double.is_empty() {
                // Mimic \@ifnextchar - check if next token is |
                let next_token = context.future_mut()?;
                if next_token.text == "|" {
                    context.pop_token()?; // consume the |
                    doubled = true;
                }
            }

            let tokens = if doubled {
                middle_double.clone()
            } else {
                middle.clone()
            };
            Ok(MacroExpansionResult::Expansion(MacroExpansion {
                tokens,
                num_args: 0,
                ..Default::default()
            }))
        },
    );

    let mid_macro_single = {
        let mid_macro = Arc::clone(&mid_macro);
        move |context: &mut dyn MacroContextInterface| mid_macro(context, false)
    };

    context.macros_mut().set(
        "|",
        Some(MacroDefinition::Function(Arc::new(mid_macro_single))),
        false,
    );

    if !mid_double_is_empty {
        let mid_macro_double = {
            let mid_macro = mid_macro;
            move |context: &mut dyn MacroContextInterface| mid_macro(context, true)
        };

        context.macros_mut().set(
            "\\|",
            Some(MacroDefinition::Function(Arc::new(mid_macro_double))),
            false,
        );
    }

    // Consume the argument (which may contain | or \| that get replaced)
    let arg = context.consume_arg(None)?.tokens;

    // Expand the tokens: right + arg + left (in reverse order for expansion)
    let mut tokens_to_expand = right;
    tokens_to_expand.extend(arg);
    tokens_to_expand.extend(left);

    let expanded = context.expand_tokens(tokens_to_expand)?;
    context.end_group()?;

    Ok(MacroExpansionResult::Expansion(MacroExpansion {
        tokens: expanded.into_iter().rev().collect(),
        num_args: 0,
        ..Default::default()
    }))
}

// \newcommand{\macro}[args]{definition}
// \renewcommand{\macro}[args]{definition}
// TODO: Optional arguments: \newcommand{\macro}[args][default]{definition}
fn new_command(
    context: &mut dyn MacroContextInterface,
    exists_ok: bool,
    nonexists_ok: bool,
    skip_if_exists: bool,
) -> Result<MacroExpansionResult, ParseError> {
    let arg = context.consume_arg(None)?.tokens;
    if arg.len() != 1 {
        return Err(ParseError::new(ParseErrorKind::ExpectedControlSequence));
    }

    let name = &arg[0].text;
    let exists = context.is_defined(name.as_str());
    if exists && !exists_ok {
        return Err(ParseError::new(ParseErrorKind::NewcommandRedefinition {
            name: name.to_owned_string(),
        }));
    }
    if !exists && !nonexists_ok {
        return Err(ParseError::new(ParseErrorKind::RenewcommandNonexistent {
            name: name.to_owned_string(),
        }));
    }

    let mut num_args = 0;
    let mut arg = context.consume_arg(None)?.tokens;
    if arg.len() == 1 && arg[0].text == "[" {
        let mut arg_text = String::new();
        let mut token = context.expand_next_token()?;
        while token.text != "]" && token.text != "EOF" {
            // TODO: Should properly expand arg, e.g., ignore {}s
            arg_text += token.text.as_str();
            token = context.expand_next_token()?;
        }
        num_args = arg_text
            .parse::<usize>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidNewcommandArgumentCount))?;
        arg = context.consume_arg(None)?.tokens;
    }

    if !(exists && skip_if_exists) {
        context.macros_mut().set(
            name.as_str(),
            Some(MacroDefinition::Expansion(MacroExpansion {
                tokens: arg,
                num_args,
                ..Default::default()
            })),
            false,
        );
    }

    Ok(MacroExpansionResult::Empty)
}

/// Built-in macros mapping
/// This is equivalent to KaTeX's `src/macros.js` built-in macros.
/// Note that this is a `phf::Map` for efficiency, as it is immutable
#[allow(clippy::print_stdout)]
#[allow(clippy::print_stderr)]
pub const BUILTIN_MACROS: phf::Map<&str, MacroDefinition> = phf_map! {
    "\\noexpand" => MacroDefinition::StaticFunction(|context| {
        // The expansion is the token itself; but that token is interpreted
        // as if its meaning were ‘\relax’ if it is a control sequence that
        // would ordinarily be expanded by TeX’s expansion rules.
        let mut token = context.pop_token()?;
        if context.is_expandable(token.text.as_str()) {
            token.noexpand = Some(true);
            token.treat_as_relax = Some(true);
        }
        Ok(MacroExpansionResult::Expansion(MacroExpansion {
            tokens: vec![token],
            num_args: 0,
            ..Default::default()
        }))
    }),
    "\\expandafter" => MacroDefinition::StaticFunction(|context| {
        // TeX first reads the token that comes immediately after \expandafter,
        // without expanding it; let’s call this token t. Then TeX reads the
        // token that comes after t (and possibly more tokens, if that token
        // has an argument), replacing it by its expansion. Finally TeX puts
        // t back in front of that expansion.
        let t = context.pop_token()?;
        let _ = context.expand_once(Some(true))?; // expand only an expandable token
        Ok(MacroExpansionResult::Expansion(MacroExpansion {
            tokens: vec![t],
            num_args: 0,
            ..Default::default()
        }))
    }),
    "\\@firstoftwo" => MacroDefinition::StaticFunction(|context| {
        // LaTeX's \@firstoftwo{#1}{#2} expands to #1, skipping #2
        // TeX source: \long\def\@firstoftwo#1#2{#1}
        let args = context.consume_args(2)?;
        Ok(MacroExpansionResult::Expansion(MacroExpansion {
            tokens: args[0].clone(),
            num_args: 0,
            ..Default::default()
        }))
    }),
    "\\@secondoftwo" => MacroDefinition::StaticFunction(|context| {
        // LaTeX's \@secondoftwo{#1}{#2} expands to #2, skipping #1
        // TeX source: \long\def\@secondoftwo#1#2{#2}
        let args = context.consume_args(2)?;
        Ok(MacroExpansionResult::Expansion(MacroExpansion {
            tokens: args[1].clone(),
            num_args: 0,
            ..Default::default()
        }))
    }),
    "\\@ifnextchar" => MacroDefinition::StaticFunction(|context| {
        // LaTeX's \@ifnextchar{#1}{#2}{#3} looks ahead to the next (unexpanded)
        // symbol that isn't a space, consuming any spaces but not consuming the
        // first nonspace character.  If that nonspace character matches #1, then
        // the macro expands to #2; otherwise, it expands to #3.
        let args = context.consume_args(3)?;
        context.consume_spaces()?;
        let next_token = context.future_mut()?;
        if args[0].len() == 1 && args[0][0].text == next_token.text {
            Ok(MacroExpansionResult::Expansion(MacroExpansion {
                tokens: args[1].clone(),
                num_args: 0,
                ..Default::default()
            }))
        } else {
            Ok(MacroExpansionResult::Expansion(MacroExpansion {
                tokens: args[2].clone(),
                num_args: 0,
                ..Default::default()
            }))
        }
    }),
    "\\@ifstar" => MacroDefinition::StaticStr("\\@ifnextchar *{\\@firstoftwo{#1}}"),
    "\\TextOrMath" => MacroDefinition::StaticFunction(|context| {
        let args = context.consume_args(2)?;
        let tokens = if context.mode() == Mode::Text {
            args[0].clone()
        } else {
            args[1].clone()
        };
        Ok(MacroExpansionResult::Expansion(MacroExpansion {
            tokens,
            num_args: 0,
            ..Default::default()
        }))
    }),
    "\\char" => MacroDefinition::StaticFunction(|context| {
        // TeX \char makes a literal character (catcode 12) using the following forms:
        // (see The TeXBook, p. 43)
        //   \char123  -- decimal
        //   \char'123 -- octal
        //   \char"123 -- hex
        //   \char`x   -- character that can be written (i.e. isn't active)
        //   \char`\x  -- character that cannot be written (e.g. %)
        // These all refer to characters from the font, so we turn them into special
        // calls to a function \@char dealt with in the Parser.
        let token = context.pop_token()?;
        let text_to_value = |s: &str| -> Option<u32> {
            let mut chars = s.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else { return None };
            match c {
                '0'..='9' => Some(c as u32 - '0' as u32),
                'a'..='f' => Some(c as u32 - 'a' as u32 + 10),
                'A'..='F' => Some(c as u32 - 'A' as u32 + 10),
                _ => None,
            }
        };

        let parse_number = |
            text: &TokenText,
            base: u32,
            context: &mut dyn MacroContextInterface,
        | -> Result<u32, ParseError> {
            // Parse a number in the given base, starting with first `token`.
            let digit = text_to_value(text.as_str());
            let mut number;
            if let Some(digit) = digit && digit < base {
                number = digit;
            } else {
                return Err(ParseError::new(ParseErrorKind::InvalidBaseDigit {
                    base,
                    digit: token.text.to_owned_string(),
                }));
            }

            while let Ok(tok) = context.future_mut() && tok.text != "EOF" {
                let digit = text_to_value(tok.text.as_str());
                if let Some(digit) = digit && digit < base {
                    number = number * base + digit;
                    context.pop_token()?;
                } else {
                    break;
                }
            }
            Ok(number)
        };

        let number = match token.text.as_str() {
            "'" => {
                let tok = context.pop_token()?;
                let tok_text = tok.text;
                parse_number(&tok_text, 8, context)?
            },
            "\"" => {
                let tok = context.pop_token()?;
                let tok_text = tok.text;
                parse_number(&tok_text, 16, context)?
            },
            "`" => {
                let token = context.pop_token()?;
                let code_at;
                if token.text.as_str().starts_with('\\') {
                    code_at = 1;
                } else if token.text.as_str() == "EOF" {
                    return Err(ParseError::new(ParseErrorKind::CharMissingArgument));
                } else {
                    code_at = 0;
                }
                token
                    .text
                    .as_str()
                    .chars()
                    .nth(code_at)
                    .ok_or_else(|| ParseError::new(ParseErrorKind::CharMissingArgument))?
                    as u32
            },
            _ => parse_number(&token.text, 10, context)?,
        };

        Ok(MacroExpansionResult::String(
            format!("\\@char{{{number}}}")
        ))
    }),
    "\\newcommand" => MacroDefinition::StaticFunction(|context| {
        new_command(context, false, true, false)
    }),
    "\\renewcommand" => MacroDefinition::StaticFunction(|context| {
        new_command(context, true, false, false)
    }),
    "\\providecommand" => MacroDefinition::StaticFunction(|context| {
        new_command(context, true, true, true)
    }),
    // terminal (console) tools
    "\\message" => MacroDefinition::StaticFunction(|context| {
        let args = context.consume_args(1)?;
        let msg = args[0]
            .iter()
            .rev()
            .map(|t| t.text.as_str())
            .collect::<String>();
        #[cfg(not(feature = "wasm"))]
        {
            let mut handle = io::stdout().lock();
            let _ = writeln!(handle, "{msg}");
            let _ = handle.flush();
        }
        #[cfg(feature = "wasm")]
        println!("{msg}");
        Ok(MacroExpansionResult::Empty)
    }),
    "\\errmessage" => MacroDefinition::StaticFunction(|context| {
        let args = context.consume_args(1)?;
        let msg = args[0]
            .iter()
            .rev()
            .map(|t| t.text.as_str())
            .collect::<String>();
        #[cfg(not(feature = "wasm"))]
        {
            let mut handle = io::stderr().lock();
            let _ = writeln!(handle, "{msg}");
            let _ = handle.flush();
        }
        #[cfg(feature = "wasm")]
        eprintln!("{msg}");
        Ok(MacroExpansionResult::Empty)
    }),
    "\\show" => MacroDefinition::StaticFunction(|context| {
        let tok = context.pop_token()?;
        let name = &tok.text;
        let func_desc =  if context.context().functions.contains_key(name.as_str()) {
            "<function>"
        } else {
            "<not a function>"
        };
        println!("{:?} {:?} {} {:?} {:?}",
            tok,
            context.macros().get(name.as_str()),
            func_desc,
            context.context().symbols.get_math(name.as_str()),
            context.context().symbols.get_text(name.as_str())
        );
        Ok(MacroExpansionResult::Empty)
    }),

    // Grouping
    "\\bgroup" => MacroDefinition::StaticStr("{"),
    "\\egroup" => MacroDefinition::StaticStr("}"),

    // Symbols from latex.ltx
    "~" => MacroDefinition::StaticStr("\\nobreakspace"),
    "\\lq" => MacroDefinition::StaticStr("`"),
    "\\rq" => MacroDefinition::StaticStr("'"),
    "\\aa" => MacroDefinition::StaticStr("\\r a"),
    "\\AA" => MacroDefinition::StaticStr("\\r A"),

    // Copyright (C) and registered (R) symbols. Use raw symbol in MathML.
    "\\textcopyright" => MacroDefinition::StaticStr("\\html@mathml{\\textcircled{c}}{\\char`\u{a9}}"),
    "\\copyright" => MacroDefinition::StaticStr("\\TextOrMath{\\textcopyright}{\\text{\\textcopyright}}"),
    "\\textregistered" => MacroDefinition::StaticStr("\\html@mathml{\\textcircled{\\scriptsize R}}{\\char`\u{ae}}"),

    // Characters omitted from Unicode range 1D400–1D7FF
    "\u{212C}" => MacroDefinition::StaticStr("\\mathscr{B}"),  // script
    "\u{2130}" => MacroDefinition::StaticStr("\\mathscr{E}"),
    "\u{2131}" => MacroDefinition::StaticStr("\\mathscr{F}"),
    "\u{210B}" => MacroDefinition::StaticStr("\\mathscr{H}"),
    "\u{2110}" => MacroDefinition::StaticStr("\\mathscr{I}"),
    "\u{2112}" => MacroDefinition::StaticStr("\\mathscr{L}"),
    "\u{2133}" => MacroDefinition::StaticStr("\\mathscr{M}"),
    "\u{211B}" => MacroDefinition::StaticStr("\\mathscr{R}"),
    "\u{212D}" => MacroDefinition::StaticStr("\\mathfrak{C}"),  // Fraktur
    "\u{210C}" => MacroDefinition::StaticStr("\\mathfrak{H}"),
    "\u{2128}" => MacroDefinition::StaticStr("\\mathfrak{Z}"),

    // Define \Bbbk with a macro that works in both HTML and MathML.
    "\\Bbbk" => MacroDefinition::StaticStr("\\Bbb{k}"),

    // Unicode middle dot
    "\u{00B7}" => MacroDefinition::StaticStr("\\cdotp"),

    // \llap and \rlap render their contents in text mode
    "\\llap" => MacroDefinition::StaticStr("\\mathllap{\\textrm{#1}}"),
    "\\rlap" => MacroDefinition::StaticStr("\\mathrlap{\\textrm{#1}}"),
    "\\clap" => MacroDefinition::StaticStr("\\mathclap{\\textrm{#1}}"),

    // \mathstrut from the TeXbook, p 360
    "\\mathstrut" => MacroDefinition::StaticStr("\\vphantom{(}"),

    // \underbar from TeXbook p 353
    "\\underbar" => MacroDefinition::StaticStr("\\underline{\\text{#1}}"),

    // \not is defined by base/fontmath.ltx via
    "\\not" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\mathrlap\\@not}}{\\char\"338}"),

    // Negated symbols from base/fontmath.ltx:
    "\\neq" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\not=}}{\\mathrel{\\char`\u{2260}}}"),
    "\\ne" => MacroDefinition::StaticStr("\\neq"),
    "\u{2260}" => MacroDefinition::StaticStr("\\neq"),
    "\\notin" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{{\\in}\\mathllap{/\\mskip1mu}}}{\\mathrel{\\char`\u{2209}}}"),
    "\u{2209}" => MacroDefinition::StaticStr("\\notin"),

    // Unicode stacked relations
    "\u{2258}" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{=\\kern{-1em}\\raisebox{0.4em}{$\\scriptsize\\frown$}}}{\\mathrel{\\char`\u{2258}}}"),
    "\u{2259}" => MacroDefinition::StaticStr("\\html@mathml{\\stackrel{\\tiny\\wedge}{=}}{\\mathrel{\\char`\u{2258}}}"),
    "\u{225A}" => MacroDefinition::StaticStr("\\html@mathml{\\stackrel{\\tiny\\vee}{=}}{\\mathrel{\\char`\u{225A}}}"),
    "\u{225B}" => MacroDefinition::StaticStr("\\html@mathml{\\stackrel{\\scriptsize\\star}{=}}{\\mathrel{\\char`\u{225B}}}"),
    "\u{225D}" => MacroDefinition::StaticStr("\\html@mathml{\\stackrel{\\tiny\\mathrm{def}}{=}}{\\mathrel{\\char`\u{225D}}}"),
    "\u{225E}" => MacroDefinition::StaticStr("\\html@mathml{\\stackrel{\\tiny\\mathrm{m}}{=}}{\\mathrel{\\char`\u{225E}}}"),
    "\u{225F}" => MacroDefinition::StaticStr("\\html@mathml{\\stackrel{\\tiny?}{=}}{\\mathrel{\\char`\u{225F}}}"),

    // Misc Unicode
    "\u{27C2}" => MacroDefinition::StaticStr("\\perp"),
    "\u{203C}" => MacroDefinition::StaticStr("\\mathclose{!\\mkern-0.8mu!}"),
    "\u{220C}" => MacroDefinition::StaticStr("\\notni"),
    "\u{231C}" => MacroDefinition::StaticStr("\\ulcorner"),
    "\u{231D}" => MacroDefinition::StaticStr("\\urcorner"),
    "\u{231E}" => MacroDefinition::StaticStr("\\llcorner"),
    "\u{231F}" => MacroDefinition::StaticStr("\\lrcorner"),
    "\u{00A9}" => MacroDefinition::StaticStr("\\copyright"),
    "\u{00AE}" => MacroDefinition::StaticStr("\\textregistered"),
    "\u{FE0F}" => MacroDefinition::StaticStr("\\textregistered"),

    // The KaTeX fonts have corners at codepoints that don't match Unicode.
    // For MathML purposes, use the Unicode code point.
    "\\ulcorner" => MacroDefinition::StaticStr("\\html@mathml{\\@ulcorner}{\\mathop{\\char\"231c}}"),
    "\\urcorner" => MacroDefinition::StaticStr("\\html@mathml{\\@urcorner}{\\mathop{\\char\"231d}}"),
    "\\llcorner" => MacroDefinition::StaticStr("\\html@mathml{\\@llcorner}{\\mathop{\\char\"231e}}"),
    "\\lrcorner" => MacroDefinition::StaticStr("\\html@mathml{\\@lrcorner}{\\mathop{\\char\"231f}}"),

    //////////////////////////////////////////////////////////////////////
    // LaTeX_2ε
    // We'll call \varvdots, which gets a glyph from symbols.js.
    // The zero-width rule gets us an equivalent to the vertical 6pt kern.

    "\\vdots" => MacroDefinition::StaticStr("{\\varvdots\\rule{0pt}{15pt}}"),
    "\u{22EE}" => MacroDefinition::StaticStr("\\vdots"),

    //////////////////////////////////////////////////////////////////////
    // amsmath.sty
    // http://mirrors.concertpass.com/tex-archive/macros/latex/required/amsmath/amsmath.pdf

    // We'll call \varvdots, which gets a glyph from symbols.js.
    // The zero-width rule gets us an equivalent to the vertical 6pt kern.
    "\\varGamma" => MacroDefinition::StaticStr("\\mathit{\\Gamma}"),
    "\\varDelta" => MacroDefinition::StaticStr("\\mathit{\\Delta}"),
    "\\varTheta" => MacroDefinition::StaticStr("\\mathit{\\Theta}"),
    "\\varLambda" => MacroDefinition::StaticStr("\\mathit{\\Lambda}"),
    "\\varXi" => MacroDefinition::StaticStr("\\mathit{\\Xi}"),
    "\\varPi" => MacroDefinition::StaticStr("\\mathit{\\Pi}"),
    "\\varSigma" => MacroDefinition::StaticStr("\\mathit{\\Sigma}"),
    "\\varUpsilon" => MacroDefinition::StaticStr("\\mathit{\\Upsilon}"),
    "\\varPhi" => MacroDefinition::StaticStr("\\mathit{\\Phi}"),
    "\\varPsi" => MacroDefinition::StaticStr("\\mathit{\\Psi}"),
    "\\varOmega" => MacroDefinition::StaticStr("\\mathit{\\Omega}"),

    "\\substack" => MacroDefinition::StaticStr("\\begin{subarray}{c}#1\\end{subarray}"),

    "\\colon" => MacroDefinition::StaticStr("\\nobreak\\mskip2mu\\mathpunct{}\\mathchoice{\\mkern-3mu}{\\mkern-3mu}{}{}{:}\\mskip6mu\\relax"),

    "\\boxed" => MacroDefinition::StaticStr("\\fbox{$\\displaystyle{#1}$}"),

    "\\iff" => MacroDefinition::StaticStr("\\DOTSB\\;\\Longleftrightarrow\\;"),
    "\\implies" => MacroDefinition::StaticStr("\\DOTSB\\;\\Longrightarrow\\;"),
    "\\impliedby" => MacroDefinition::StaticStr("\\DOTSB\\;\\Longleftarrow\\;"),

    "\\dddot" => MacroDefinition::StaticStr("{\\overset{\\raisebox{-0.1ex}{\\normalsize ...}}{#1}}"),
    "\\ddddot" => MacroDefinition::StaticStr("{\\overset{\\raisebox{-0.1ex}{\\normalsize ....}}{#1}}"),

    // !TODO: \dots and \dotso \dotsc, etc should be handled in FunctionHandler

    "\\dotsb" => MacroDefinition::StaticStr("\\cdots"),
    "\\dotsm" => MacroDefinition::StaticStr("\\cdots"),
    "\\dotsi" => MacroDefinition::StaticStr("\\!\\cdots"),
    // amsmath doesn't actually define \dotsx, but \dots followed by a macro
    // starting with \DOTSX implies \dotso, and then \extra@ detects this case
    // and forces the added `\,`.
    "\\dotsx" => MacroDefinition::StaticStr("\\ldots\\,"),

    "\\dots" => MacroDefinition::StaticFunction(|context| {
        let mut thedots = "\\dotso";
        let next = context.expand_after_future()?.text;
        if let Some(dots_type) = DOTS_TYPE.get(next.as_str()) {
            thedots = dots_type;
        } else if next.as_str().starts_with("\\not") {
            thedots = "\\dotsb";
        } else if let Some(char_info) = context.context().symbols.get_math(next.as_str())
            && let Group::Atom(Atom::Bin | Atom::Rel) = char_info.group {
                thedots = "\\dotsb";
            }
        Ok(MacroExpansionResult::String(thedots.to_owned()))
    }),
    "\\dotso" => MacroDefinition::StaticFunction(|context| {
        let next = context.future_mut()?.text;
        if IS_SPACE_AFTER_DOTS.contains(next.as_str()) {
            Ok(MacroExpansionResult::String("\\ldots\\,".to_owned()))
        } else {
            Ok(MacroExpansionResult::String("\\ldots".to_owned()))
        }
    }),
    "\\dotsc" => MacroDefinition::StaticFunction(|context| {
        let next = context.future_mut()?.text;
        if IS_SPACE_AFTER_DOTS.contains(next.as_str()) && next != "," {
            Ok(MacroExpansionResult::String("\\ldots\\,".to_owned()))
        } else {
            Ok(MacroExpansionResult::String("\\ldots".to_owned()))
        }
    }),
    "\\cdots" => MacroDefinition::StaticFunction(|context| {
        let next = context.future_mut()?.text;
        if IS_SPACE_AFTER_DOTS.contains(next.as_str()) {
            Ok(MacroExpansionResult::String("\\@cdots\\,".to_owned()))
        } else {
            Ok(MacroExpansionResult::String("\\@cdots".to_owned()))
        }
    }),

    "\\DOTSI" => MacroDefinition::StaticStr("\\relax"),
    "\\DOTSB" => MacroDefinition::StaticStr("\\relax"),
    "\\DOTSX" => MacroDefinition::StaticStr("\\relax"),

    // Spacing, based on amsmath.sty's override of LaTeX defaults
    "\\tmspace" => MacroDefinition::StaticStr("\\TextOrMath{\\kern#1#3}{\\mskip#1#2}\\relax"),
    "\\," => MacroDefinition::StaticStr("\\tmspace+{3mu}{.1667em}"),
    "\\thinspace" => MacroDefinition::StaticStr("\\,"),
    "\\>" => MacroDefinition::StaticStr("\\mskip{4mu}"),
    "\\:" => MacroDefinition::StaticStr("\\tmspace+{4mu}{.2222em}"),
    "\\medspace" => MacroDefinition::StaticStr("\\:"),
    "\\;" => MacroDefinition::StaticStr("\\tmspace+{5mu}{.2777em}"),
    "\\thickspace" => MacroDefinition::StaticStr("\\;"),
    "\\!" => MacroDefinition::StaticStr("\\tmspace-{3mu}{.1667em}"),
    "\\negthinspace" => MacroDefinition::StaticStr("\\!"),
    "\\negmedspace" => MacroDefinition::StaticStr("\\tmspace-{4mu}{.2222em}"),
    "\\negthickspace" => MacroDefinition::StaticStr("\\tmspace-{5mu}{.277em}"),
    "\\enspace" => MacroDefinition::StaticStr("\\kern.5em "),
    "\\enskip" => MacroDefinition::StaticStr("\\hskip.5em\\relax"),
    "\\quad" => MacroDefinition::StaticStr("\\hskip1em\\relax"),
    "\\qquad" => MacroDefinition::StaticStr("\\hskip2em\\relax"),

    // \tag@in@display form of \tag
    "\\tag" => MacroDefinition::StaticStr("\\@ifstar\\tag@literal\\tag@paren"),
    "\\tag@paren" => MacroDefinition::StaticStr("\\tag@literal{({#1})}"),
    "\\tag@literal" => MacroDefinition::StaticFunction(|context| {
        if context.macros().get("\\df@tag").is_some() {
            return Err(ParseError::new(ParseErrorKind::MultipleTag));
        }
        Ok(MacroExpansionResult::String("\\gdef\\df@tag{\\text{#1}}".to_owned()))
    }),

    // TODO: math mode should use \medmuskip = 4mu plus 2mu minus 4mu
    "\\bmod" => MacroDefinition::StaticStr("\\mathchoice{\\mskip1mu}{\\mskip1mu}{\\mskip5mu}{\\mskip5mu}\\mathbin{\\rm mod}\\mathchoice{\\mskip1mu}{\\mskip1mu}{\\mskip5mu}{\\mskip5mu}"),
    "\\pod" => MacroDefinition::StaticStr("\\allowbreak\\mathchoice{\\mkern18mu}{\\mkern8mu}{\\mkern8mu}{\\mkern8mu}(#1)"),
    "\\pmod" => MacroDefinition::StaticStr("\\pod{{\\rm mod}\\mkern6mu#1}"),
    "\\mod" => MacroDefinition::StaticStr("\\allowbreak\\mathchoice{\\mkern18mu}{\\mkern12mu}{\\mkern12mu}{\\mkern12mu}{\\rm mod}\\,\\,#1"),

    //////////////////////////////////////////////////////////////////////
    // LaTeX source2e

    "\\newline" => MacroDefinition::StaticStr("\\\\\\relax"),

    // \def\TeX{T\kern-.1667em\lower.5ex\hbox{E}\kern-.125emX\@}
    // TODO: Doesn't normally work in math mode because \@ fails.  KaTeX doesn't
    // support \@ yet, so that's omitted, and we add \text so that the result
    // doesn't look funny in math mode.
    "\\TeX" => MacroDefinition::StaticStr("\\textrm{\\html@mathml{T\\kern-.1667em\\raisebox{-.5ex}{E}\\kern-.125emX}{TeX}}"),

    // \DeclareRobustCommand{\LaTeX}{L\kern-.36em%
    //         {\sbox\z@ T%
    //          \vbox to\ht\z@{\hbox{\check@mathfonts
    //                               \fontsize\sf@size\z@
    //                               \math@fontsfalse\selectfont
    //                               A}%
    //                         \vss}%
    //         }%
    //         \kern-.15em%
    //         \TeX}
    // This code aligns the top of the A with the T (from the perspective of TeX's
    // boxes, though visually the A appears to extend above slightly).
    // We compute the corresponding \raisebox when A is rendered in \normalsize
    // \scriptstyle, which has a scale factor of 0.7 (see Options.js).
    "\\LaTeX" => MacroDefinition::StaticFunction(|_context| {
        let raise_value = calculate_latex_raise_a();
        Ok(MacroExpansionResult::String(format!(
            "\\textrm{{\\html@mathml{{L\\kern-.36em\\raisebox{{{raise_value}}}{{\\scriptstyle A}}\\kern-.15em\\TeX}}{{LaTeX}}}}"
        )))
    }),

    // New KaTeX logo based on tweaking LaTeX logo
    "\\KaTeX" => MacroDefinition::StaticFunction(|_context| {
        let raise_value = calculate_latex_raise_a();
        Ok(MacroExpansionResult::String(format!(
            "\\textrm{{\\html@mathml{{K\\kern-.17em\\raisebox{{{raise_value}}}{{\\scriptstyle A}}\\kern-.15em\\TeX}}{{KaTeX}}}}"
        )))
    }),

    // \DeclareRobustCommand\hspace{\@ifstar\@hspacer\@hspace}
    // \def\@hspace#1{\hskip  #1\relax}
    // \def\@hspacer#1{\vrule \@width\z@\nobreak
    //                 \hskip #1\hskip \z@skip}
    "\\hspace" => MacroDefinition::StaticStr("\\@ifstar\\@hspacer\\@hspace"),
    "\\@hspace" => MacroDefinition::StaticStr("\\hskip #1\\relax"),
    "\\@hspacer" => MacroDefinition::StaticStr("\\rule{0pt}{0pt}\\hskip #1\\relax"),

    //////////////////////////////////////////////////////////////////////
    // mathtools.sty

    //\providecommand\ordinarycolon{:}
    "\\ordinarycolon" => MacroDefinition::StaticStr(":"),
    //\def\vcentcolon{\mathrel{\mathop\ordinarycolon}}
    //TODO(edemaine): Not yet centered. Fix via \raisebox or #726
    "\\vcentcolon" => MacroDefinition::StaticStr("\\mathrel{\\mathop\\ordinarycolon}"),
    // \providecommand*\dblcolon{\vcentcolon\mathrel{\mkern-.9mu}\vcentcolon}
    "\\dblcolon" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-.9mu}\\vcentcolon}}{\\mathop{\\char\"2237}}"),
    // \providecommand*\coloneqq{\vcentcolon\mathrel{\mkern-1.2mu}=}
    "\\coloneqq" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}=}}{\\mathop{\\char\"2254}}"), // ≔
    // \providecommand*\Coloneqq{\dblcolon\mathrel{\mkern-1.2mu}=}
    "\\Coloneqq" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}=}}{\\mathop{\\char\"2237\\char\"3d}}"),
    // \providecommand*\coloneq{\vcentcolon\mathrel{\mkern-1.2mu}\mathrel{-}}
    "\\coloneq" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\mathrel{-}}}{\\mathop{\\char\"3a\\char\"2212}}"),
    // \providecommand*\Coloneq{\dblcolon\mathrel{\mkern-1.2mu}\mathrel{-}}
    "\\Coloneq" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\mathrel{-}}}{\\mathop{\\char\"2237\\char\"2212}}"),
    // \providecommand*\eqqcolon{=\mathrel{\mkern-1.2mu}\vcentcolon}
    "\\eqqcolon" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{=\\mathrel{\\mkern-1.2mu}\\vcentcolon}}{\\mathop{\\char\"2255}}"), // ≕
    // \providecommand*\Eqqcolon{=\mathrel{\mkern-1.2mu}\dblcolon}
    "\\Eqqcolon" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{=\\mathrel{\\mkern-1.2mu}\\dblcolon}}{\\mathop{\\char\"3d\\char\"2237}}"),
    // \providecommand*\eqcolon{\mathrel{-}\mathrel{\mkern-1.2mu}\vcentcolon}
    "\\eqcolon" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\mathrel{-}\\mathrel{\\mkern-1.2mu}\\vcentcolon}}{\\mathop{\\char\"2239}}"),
    // \providecommand*\Eqcolon{\mathrel{-}\mathrel{\mkern-1.2mu}\dblcolon}
    "\\Eqcolon" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\mathrel{-}\\mathrel{\\mkern-1.2mu}\\dblcolon}}{\\mathop{\\char\"2212\\char\"2237}}"),
    // \providecommand*\colonapprox{\vcentcolon\mathrel{\mkern-1.2mu}\approx}
    "\\colonapprox" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\approx}}{\\mathop{\\char\"3a\\char\"2248}}"),
    // \providecommand*\Colonapprox{\dblcolon\mathrel{\mkern-1.2mu}\approx}
    "\\Colonapprox" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\approx}}{\\mathop{\\char\"2237\\char\"2248}}"),
    // \providecommand*\colonsim{\vcentcolon\mathrel{\mkern-1.2mu}\sim}
    "\\colonsim" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\sim}}{\\mathop{\\char\"3a\\char\"223c}}"),
    // \providecommand*\Colonsim{\dblcolon\mathrel{\mkern-1.2mu}\sim}
    "\\Colonsim" => MacroDefinition::StaticStr("\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\sim}}{\\mathop{\\char\"2237\\char\"223c}}"),

    // Some Unicode characters are implemented with macros to mathtools functions.
    "\u{2237}" => MacroDefinition::StaticStr("\\dblcolon"),  // ::
    "\u{2239}" => MacroDefinition::StaticStr("\\eqcolon"),  // -:
    "\u{2254}" => MacroDefinition::StaticStr("\\coloneqq"),  // :=
    "\u{2255}" => MacroDefinition::StaticStr("\\eqqcolon"),  // =:
    "\u{2A74}" => MacroDefinition::StaticStr("\\Coloneqq"),  // ::=

    //////////////////////////////////////////////////////////////////////
    // colonequals.sty

    // Alternate names for mathtools's macros:
    "\\ratio" => MacroDefinition::StaticStr("\\vcentcolon"),
    "\\coloncolon" => MacroDefinition::StaticStr("\\dblcolon"),
    "\\colonequals" => MacroDefinition::StaticStr("\\coloneqq"),
    "\\coloncolonequals" => MacroDefinition::StaticStr("\\Coloneqq"),
    "\\equalscolon" => MacroDefinition::StaticStr("\\eqqcolon"),
    "\\equalscoloncolon" => MacroDefinition::StaticStr("\\Eqqcolon"),
    "\\colonminus" => MacroDefinition::StaticStr("\\coloneq"),
    "\\coloncolonminus" => MacroDefinition::StaticStr("\\Coloneq"),
    "\\minuscolon" => MacroDefinition::StaticStr("\\eqcolon"),
    "\\minuscoloncolon" => MacroDefinition::StaticStr("\\Eqcolon"),
    // \colonapprox name is same in mathtools and colonequals.
    "\\coloncolonapprox" => MacroDefinition::StaticStr("\\Colonapprox"),
    // \colonsim name is same in mathtools and colonequals.
    "\\coloncolonsim" => MacroDefinition::StaticStr("\\Colonsim"),

    // Additional macros, implemented by analogy with mathtools definitions:
    "\\simcolon" => MacroDefinition::StaticStr("\\mathrel{\\sim\\mathrel{\\mkern-1.2mu}\\vcentcolon}"),
    "\\simcoloncolon" => MacroDefinition::StaticStr("\\mathrel{\\sim\\mathrel{\\mkern-1.2mu}\\dblcolon}"),
    "\\approxcolon" => MacroDefinition::StaticStr("\\mathrel{\\approx\\mathrel{\\mkern-1.2mu}\\vcentcolon}"),
    "\\approxcoloncolon" => MacroDefinition::StaticStr("\\mathrel{\\approx\\mathrel{\\mkern-1.2mu}\\dblcolon}"),

    // Present in newtxmath, pxfonts and txfonts
    "\\notni" => MacroDefinition::StaticStr("\\html@mathml{\\not\\ni}{\\mathrel{\\char`\u{220C}}}"),
    "\\limsup" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{lim\\,sup}"),
    "\\liminf" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{lim\\,inf}"),

    //////////////////////////////////////////////////////////////////////
    // From amsopn.sty
    "\\injlim" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{inj\\,lim}"),
    "\\projlim" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{proj\\,lim}"),
    "\\varlimsup" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{\\overline{lim}}"),
    "\\varliminf" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{\\underline{lim}}"),
    "\\varinjlim" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{\\underrightarrow{lim}}"),
    "\\varprojlim" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{\\underleftarrow{lim}}"),

    //////////////////////////////////////////////////////////////////////
    // MathML alternates for KaTeX glyphs in the Unicode private area
    "\\gvertneqq" => MacroDefinition::StaticStr("\\html@mathml{\\@gvertneqq}{\u{2269}}"),
    "\\lvertneqq" => MacroDefinition::StaticStr("\\html@mathml{\\@lvertneqq}{\u{2268}}"),
    "\\ngeqq" => MacroDefinition::StaticStr("\\html@mathml{\\@ngeqq}{\u{2271}}"),
    "\\ngeqslant" => MacroDefinition::StaticStr("\\html@mathml{\\@ngeqslant}{\u{2271}}"),
    "\\nleqq" => MacroDefinition::StaticStr("\\html@mathml{\\@nleqq}{\u{2270}}"),
    "\\nleqslant" => MacroDefinition::StaticStr("\\html@mathml{\\@nleqslant}{\u{2270}}"),
    "\\nshortmid" => MacroDefinition::StaticStr("\\html@mathml{\\@nshortmid}{\u{2224}}"),
    "\\nshortparallel" => MacroDefinition::StaticStr("\\html@mathml{\\@nshortparallel}{\u{2226}}"),
    "\\nsubseteqq" => MacroDefinition::StaticStr("\\html@mathml{\\@nsubseteqq}{\u{2288}}"),
    "\\nsupseteqq" => MacroDefinition::StaticStr("\\html@mathml{\\@nsupseteqq}{\u{2289}}"),
    "\\varsubsetneq" => MacroDefinition::StaticStr("\\html@mathml{\\@varsubsetneq}{\u{228a}}"),
    "\\varsubsetneqq" => MacroDefinition::StaticStr("\\html@mathml{\\@varsubsetneqq}{\u{2acb}}"),
    "\\varsupsetneq" => MacroDefinition::StaticStr("\\html@mathml{\\@varsupsetneq}{\u{228b}}"),
    "\\varsupsetneqq" => MacroDefinition::StaticStr("\\html@mathml{\\@varsupsetneqq}{\u{2acc}}"),
    "\\imath" => MacroDefinition::StaticStr("\\html@mathml{\\@imath}{\u{0131}}"),
    "\\jmath" => MacroDefinition::StaticStr("\\html@mathml{\\@jmath}{\u{0237}}"),

    //////////////////////////////////////////////////////////////////////
    // stmaryrd and semantic

    // The stmaryrd and semantic packages render the next four items by calling a
    // glyph. Those glyphs do not exist in the KaTeX fonts. Hence the macros.

    "\\llbracket" => MacroDefinition::StaticStr("\\html@mathml{\\mathopen{[\\mkern-3.2mu[}}{\\mathopen{\\char`\u{27e6}}}"),
    "\\rrbracket" => MacroDefinition::StaticStr("\\html@mathml{\\mathclose{]\\mkern-3.2mu]}}{\\mathclose{\\char`\u{27e7}}}"),

    "\u{27e6}" => MacroDefinition::StaticStr("\\llbracket"), // blackboard bold [
    "\u{27e7}" => MacroDefinition::StaticStr("\\rrbracket"), // blackboard bold ]

    "\\lBrace" => MacroDefinition::StaticStr("\\html@mathml{\\mathopen{\\{\\mkern-3.2mu[}}{\\mathopen{\\char`\u{2983}}}"),
    "\\rBrace" => MacroDefinition::StaticStr("\\html@mathml{\\mathclose{]\\mkern-3.2mu\\}}}{\\mathclose{\\char`\u{2984}}}"),

    "\u{2983}" => MacroDefinition::StaticStr("\\lBrace"), // blackboard bold {
    "\u{2984}" => MacroDefinition::StaticStr("\\rBrace"), // blackboard bold }

    // TODO: Create variable sized versions of the last two items. I believe that
    // will require new font glyphs.

    // The stmaryrd function `\minuso` provides a "Plimsoll" symbol that
    // superimposes the characters \circ and \mathminus. Used in chemistry.
    "\\minuso" => MacroDefinition::StaticStr("\\mathbin{\\html@mathml{{\\mathrlap{\\mathchoice{\\kern{0.145em}}{\\kern{0.145em}}{\\kern{0.1015em}}{\\kern{0.0725em}}\\circ}{-}}}{\\char`\u{29b5}}}"),
    "\u{29b5}" => MacroDefinition::StaticStr("\\minuso"),

    //////////////////////////////////////////////////////////////////////
    // texvc.sty

    // The texvc package contains macros available in mediawiki pages.
    // We omit the functions deprecated at
    // https://en.wikipedia.org/wiki/Help:Displaying_a_formula#Deprecated_syntax

    // We also omit texvc's \O, which conflicts with \text{\O}

    "\\darr" => MacroDefinition::StaticStr("\\downarrow"),
    "\\dArr" => MacroDefinition::StaticStr("\\Downarrow"),
    "\\Darr" => MacroDefinition::StaticStr("\\Downarrow"),
    "\\lang" => MacroDefinition::StaticStr("\\langle"),
    "\\rang" => MacroDefinition::StaticStr("\\rangle"),
    "\\uarr" => MacroDefinition::StaticStr("\\uparrow"),
    "\\uArr" => MacroDefinition::StaticStr("\\Uparrow"),
    "\\Uarr" => MacroDefinition::StaticStr("\\Uparrow"),
    "\\N" => MacroDefinition::StaticStr("\\mathbb{N}"),
    "\\R" => MacroDefinition::StaticStr("\\mathbb{R}"),
    "\\Z" => MacroDefinition::StaticStr("\\mathbb{Z}"),
    "\\alef" => MacroDefinition::StaticStr("\\aleph"),
    "\\alefsym" => MacroDefinition::StaticStr("\\aleph"),
    "\\Alpha" => MacroDefinition::StaticStr("\\mathrm{A}"),
    "\\Beta" => MacroDefinition::StaticStr("\\mathrm{B}"),
    "\\bull" => MacroDefinition::StaticStr("\\bullet"),
    "\\Chi" => MacroDefinition::StaticStr("\\mathrm{X}"),
    "\\clubs" => MacroDefinition::StaticStr("\\clubsuit"),
    "\\cnums" => MacroDefinition::StaticStr("\\mathbb{C}"),
    "\\Complex" => MacroDefinition::StaticStr("\\mathbb{C}"),
    "\\Dagger" => MacroDefinition::StaticStr("\\ddagger"),
    "\\diamonds" => MacroDefinition::StaticStr("\\diamondsuit"),
    "\\empty" => MacroDefinition::StaticStr("\\emptyset"),
    "\\Epsilon" => MacroDefinition::StaticStr("\\mathrm{E}"),
    "\\Eta" => MacroDefinition::StaticStr("\\mathrm{H}"),
    "\\exist" => MacroDefinition::StaticStr("\\exists"),
    "\\harr" => MacroDefinition::StaticStr("\\leftrightarrow"),
    "\\hArr" => MacroDefinition::StaticStr("\\Leftrightarrow"),
    "\\Harr" => MacroDefinition::StaticStr("\\Leftrightarrow"),
    "\\hearts" => MacroDefinition::StaticStr("\\heartsuit"),
    "\\image" => MacroDefinition::StaticStr("\\Im"),
    "\\infin" => MacroDefinition::StaticStr("\\infty"),
    "\\Iota" => MacroDefinition::StaticStr("\\mathrm{I}"),
    "\\isin" => MacroDefinition::StaticStr("\\in"),
    "\\Kappa" => MacroDefinition::StaticStr("\\mathrm{K}"),
    "\\larr" => MacroDefinition::StaticStr("\\leftarrow"),
    "\\lArr" => MacroDefinition::StaticStr("\\Leftarrow"),
    "\\Larr" => MacroDefinition::StaticStr("\\Leftarrow"),
    "\\lrarr" => MacroDefinition::StaticStr("\\leftrightarrow"),
    "\\lrArr" => MacroDefinition::StaticStr("\\Leftrightarrow"),
    "\\Lrarr" => MacroDefinition::StaticStr("\\Leftrightarrow"),
    "\\Mu" => MacroDefinition::StaticStr("\\mathrm{M}"),
    "\\natnums" => MacroDefinition::StaticStr("\\mathbb{N}"),
    "\\Nu" => MacroDefinition::StaticStr("\\mathrm{N}"),
    "\\Omicron" => MacroDefinition::StaticStr("\\mathrm{O}"),
    "\\plusmn" => MacroDefinition::StaticStr("\\pm"),
    "\\rarr" => MacroDefinition::StaticStr("\\rightarrow"),
    "\\rArr" => MacroDefinition::StaticStr("\\Rightarrow"),
    "\\Rarr" => MacroDefinition::StaticStr("\\Rightarrow"),
    "\\real" => MacroDefinition::StaticStr("\\Re"),
    "\\reals" => MacroDefinition::StaticStr("\\mathbb{R}"),
    "\\Reals" => MacroDefinition::StaticStr("\\mathbb{R}"),
    "\\Rho" => MacroDefinition::StaticStr("\\mathrm{P}"),
    "\\sdot" => MacroDefinition::StaticStr("\\cdot"),
    "\\sect" => MacroDefinition::StaticStr("\\S"),
    "\\spades" => MacroDefinition::StaticStr("\\spadesuit"),
    "\\sub" => MacroDefinition::StaticStr("\\subset"),
    "\\sube" => MacroDefinition::StaticStr("\\subseteq"),
    "\\supe" => MacroDefinition::StaticStr("\\supseteq"),
    "\\Tau" => MacroDefinition::StaticStr("\\mathrm{T}"),
    "\\thetasym" => MacroDefinition::StaticStr("\\vartheta"),
    // TODO: defineMacro("\\varcoppa", "\\\mbox{\\coppa}");
    "\\weierp" => MacroDefinition::StaticStr("\\wp"),
    "\\Zeta" => MacroDefinition::StaticStr("\\mathrm{Z}"),

    //////////////////////////////////////////////////////////////////////
    // statmath.sty
    // https://ctan.math.illinois.edu/macros/latex/contrib/statmath/statmath.pdf

    "\\argmin" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{arg\\,min}"),
    "\\argmax" => MacroDefinition::StaticStr("\\DOTSB\\operatorname*{arg\\,max}"),
    "\\plim" => MacroDefinition::StaticStr("\\DOTSB\\mathop{\\operatorname{plim}}\\limits"),

    //////////////////////////////////////////////////////////////////////
    // braket.sty
    // http://ctan.math.washington.edu/tex-archive/macros/latex/contrib/braket/braket.pdf

    "\\bra" => MacroDefinition::StaticStr("\\mathinner{\\langle{#1}|}"),
    "\\ket" => MacroDefinition::StaticStr("\\mathinner{|{#1}\\rangle}"),
    "\\braket" => MacroDefinition::StaticStr("\\mathinner{\\langle{#1}\\rangle}"),
    "\\Bra" => MacroDefinition::StaticStr("\\left\\langle#1\\right|"),
    "\\Ket" => MacroDefinition::StaticStr("\\left|#1\\right\\rangle"),

    "\\Braket" => MacroDefinition::StaticStr("\\bra@ket{\\left\\langle}{\\,\\middle\\vert\\,}{\\,\\middle\\vert\\,}{\\right\\rangle}"),
    "\\Set" => MacroDefinition::StaticStr("\\bra@set{\\left\\{\\:}{\\;\\middle\\vert\\;}{\\;\\middle\\Vert\\;}{\\:\\right\\}}"),
    "\\set" => MacroDefinition::StaticStr("\\bra@set{\\{\\,}{\\mid}{}{\\,\\}}"),
    // has no support for special || or \

    "\\bra@ket" => MacroDefinition::StaticFunction(|context| braket_helper(context, false)),
    "\\bra@set" => MacroDefinition::StaticFunction(|context| braket_helper(context, true)),

    //////////////////////////////////////////////////////////////////////
    // actuarialangle.dtx
    "\\angln" => MacroDefinition::StaticStr("{\\angl n}"),

    // Custom Khan Academy colors, should be moved to an optional package
    "\\blue" => MacroDefinition::StaticStr("\\textcolor{##6495ed}{#1}"),
    "\\orange" => MacroDefinition::StaticStr("\\textcolor{##ffa500}{#1}"),
    "\\pink" => MacroDefinition::StaticStr("\\textcolor{##ff00af}{#1}"),
    "\\red" => MacroDefinition::StaticStr("\\textcolor{##df0030}{#1}"),
    "\\green" => MacroDefinition::StaticStr("\\textcolor{##28ae7b}{#1}"),
    "\\gray" => MacroDefinition::StaticStr("\\textcolor{gray}{#1}"),
    "\\purple" => MacroDefinition::StaticStr("\\textcolor{##9d38bd}{#1}"),
    "\\blueA" => MacroDefinition::StaticStr("\\textcolor{##ccfaff}{#1}"),
    "\\blueB" => MacroDefinition::StaticStr("\\textcolor{##80f6ff}{#1}"),
    "\\blueC" => MacroDefinition::StaticStr("\\textcolor{##63d9ea}{#1}"),
    "\\blueD" => MacroDefinition::StaticStr("\\textcolor{##11accd}{#1}"),
    "\\blueE" => MacroDefinition::StaticStr("\\textcolor{##0c7f99}{#1}"),
    "\\tealA" => MacroDefinition::StaticStr("\\textcolor{##94fff5}{#1}"),
    "\\tealB" => MacroDefinition::StaticStr("\\textcolor{##26edd5}{#1}"),
    "\\tealC" => MacroDefinition::StaticStr("\\textcolor{##01d1c1}{#1}"),
    "\\tealD" => MacroDefinition::StaticStr("\\textcolor{##01a995}{#1}"),
    "\\tealE" => MacroDefinition::StaticStr("\\textcolor{##208170}{#1}"),
    "\\greenA" => MacroDefinition::StaticStr("\\textcolor{##b6ffb0}{#1}"),
    "\\greenB" => MacroDefinition::StaticStr("\\textcolor{##8af281}{#1}"),
    "\\greenC" => MacroDefinition::StaticStr("\\textcolor{##74cf70}{#1}"),
    "\\greenD" => MacroDefinition::StaticStr("\\textcolor{##1fab54}{#1}"),
    "\\greenE" => MacroDefinition::StaticStr("\\textcolor{##0d923f}{#1}"),
    "\\goldA" => MacroDefinition::StaticStr("\\textcolor{##ffd0a9}{#1}"),
    "\\goldB" => MacroDefinition::StaticStr("\\textcolor{##ffbb71}{#1}"),
    "\\goldC" => MacroDefinition::StaticStr("\\textcolor{##ff9c39}{#1}"),
    "\\goldD" => MacroDefinition::StaticStr("\\textcolor{##e07d10}{#1}"),
    "\\goldE" => MacroDefinition::StaticStr("\\textcolor{##a75a05}{#1}"),
    "\\redA" => MacroDefinition::StaticStr("\\textcolor{##fca9a9}{#1}"),
    "\\redB" => MacroDefinition::StaticStr("\\textcolor{##ff8482}{#1}"),
    "\\redC" => MacroDefinition::StaticStr("\\textcolor{##f9685d}{#1}"),
    "\\redD" => MacroDefinition::StaticStr("\\textcolor{##e84d39}{#1}"),
    "\\redE" => MacroDefinition::StaticStr("\\textcolor{##bc2612}{#1}"),
    "\\maroonA" => MacroDefinition::StaticStr("\\textcolor{##ffbde0}{#1}"),
    "\\maroonB" => MacroDefinition::StaticStr("\\textcolor{##ff92c6}{#1}"),
    "\\maroonC" => MacroDefinition::StaticStr("\\textcolor{##ed5fa6}{#1}"),
    "\\maroonD" => MacroDefinition::StaticStr("\\textcolor{##ca337c}{#1}"),
    "\\maroonE" => MacroDefinition::StaticStr("\\textcolor{##9e034e}{#1}"),
    "\\purpleA" => MacroDefinition::StaticStr("\\textcolor{##ddd7ff}{#1}"),
    "\\purpleB" => MacroDefinition::StaticStr("\\textcolor{##c6b9fc}{#1}"),
    "\\purpleC" => MacroDefinition::StaticStr("\\textcolor{##aa87ff}{#1}"),
    "\\purpleD" => MacroDefinition::StaticStr("\\textcolor{##7854ab}{#1}"),
    "\\purpleE" => MacroDefinition::StaticStr("\\textcolor{##543b78}{#1}"),
    "\\mintA" => MacroDefinition::StaticStr("\\textcolor{##f5f9e8}{#1}"),
    "\\mintB" => MacroDefinition::StaticStr("\\textcolor{##edf2df}{#1}"),
    "\\mintC" => MacroDefinition::StaticStr("\\textcolor{##e0e5cc}{#1}"),
    "\\grayA" => MacroDefinition::StaticStr("\\textcolor{##f6f7f7}{#1}"),
    "\\grayB" => MacroDefinition::StaticStr("\\textcolor{##f0f1f2}{#1}"),
    "\\grayC" => MacroDefinition::StaticStr("\\textcolor{##e3e5e6}{#1}"),
    "\\grayD" => MacroDefinition::StaticStr("\\textcolor{##d6d8da}{#1}"),
    "\\grayE" => MacroDefinition::StaticStr("\\textcolor{##babec2}{#1}"),
    "\\grayF" => MacroDefinition::StaticStr("\\textcolor{##888d93}{#1}"),
    "\\grayG" => MacroDefinition::StaticStr("\\textcolor{##626569}{#1}"),
    "\\grayH" => MacroDefinition::StaticStr("\\textcolor{##3b3e40}{#1}"),
    "\\grayI" => MacroDefinition::StaticStr("\\textcolor{##21242c}{#1}"),
    "\\kaBlue" => MacroDefinition::StaticStr("\\textcolor{##314453}{#1}"),
    "\\kaGreen" => MacroDefinition::StaticStr("\\textcolor{##71B307}{#1}"),

    // From `src/environments/array.js`
    "\\nonumber" => MacroDefinition::StaticStr("\\gdef\\@eqnsw{0}"),
    "\\notag" => MacroDefinition::StaticStr("\\nonumber"),
    // From `src/functions/operatorname.js`
    "\\operatorname" => MacroDefinition::StaticStr("\\@ifstar\\operatornamewithlimits\\operatorname@"),
};
