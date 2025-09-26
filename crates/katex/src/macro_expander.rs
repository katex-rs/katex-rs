//! MacroExpander – the “gullet” that expands macros to tokens
//!
//! Ported from KaTeX/src/MacroExpander.js with adjustments to fit the Rust
//! codebase.

use alloc::sync::Arc;

use crate::context::KatexContext;
use crate::lexer::Lexer;
use crate::macros::builtins::BUILTIN_MACROS;
use crate::namespace::{KeyMap, Namespace};
use crate::types::{Mode, ParseError, ParseErrorKind, Settings, Token};

use crate::macros::{
    MacroArg, MacroContextInterface, MacroDefinition, MacroExpansion, MacroExpansionResult,
};

/// Map of macro definitions.
pub type MacroMap = KeyMap<String, MacroDefinition>;

/// Commands that act like macros but aren't defined as a macro, function, or
/// symbol
pub const IMPLICIT_COMMANDS: phf::Set<&str> = phf::phf_set! {
    "^",
    "_",
    "\\limits",
    "\\nolimits",
};

/// MacroExpander: expands macros until only non-macro tokens remain
pub struct MacroExpander<'a> {
    settings: &'a Settings,
    expansion_count: usize,
    lexer: Lexer<'a>,
    macros: Namespace<'a, MacroDefinition>,
    stack: Vec<Token>, // tokens in REVERSE order
    mode: Mode,
    /// No global object in Rust; pass context reference around
    ctx: &'a KatexContext,
}

impl<'a> MacroExpander<'a> {
    /// Create a new MacroExpander (also creates a new Lexer)
    #[must_use]
    pub fn new(input: &str, settings: &'a Settings, mode: Mode, ctx: &'a KatexContext) -> Self {
        // Build macros namespace: builtins from context, globals from settings.macros
        let globals = settings.macros.borrow_mut();
        let macros = Namespace::new(&BUILTIN_MACROS, globals);

        let mut me = Self {
            lexer: Lexer::new(Arc::from(input), settings),
            settings,
            expansion_count: 0,
            macros,
            mode,
            stack: Vec::new(),

            ctx,
        };
        // Initialize by feeding the input
        me.feed(input);
        me
    }

    /// Feed a new input string to the same MacroExpander (with existing macros
    /// etc.).
    pub fn feed(&mut self, input: &str) {
        self.lexer = Lexer::new(Arc::from(input), self.settings);
    }

    /// Switches between text and math modes
    pub const fn switch_mode(&mut self, new_mode: Mode) {
        self.mode = new_mode;
    }

    /// Ends all currently nested groups (if any)
    pub fn end_groups(&mut self) {
        self.macros.end_groups();
    }

    /// Sets the category code for a character in the lexer
    pub fn set_catcode(&mut self, char: char, code: u8) {
        self.lexer.set_catcode(char, code);
    }

    /// Add a token to the stack
    pub fn push_token(&mut self, token: Token) {
        self.stack.push(token);
    }

    /// Append multiple tokens to the stack
    pub fn push_tokens(&mut self, tokens: Vec<Token>) {
        self.stack.extend(tokens);
    }

    /// Find a macro argument without expanding tokens and append the array of
    /// tokens to the token stack Returns a Token representing the argument
    /// range, or None for missing optional arg
    pub fn scan_argument(&mut self, is_optional: bool) -> Result<Option<Token>, ParseError> {
        let (start_tok, end_tok, tokens);
        if is_optional {
            self.consume_spaces()?;
            if self.future_mut()?.text != "[" {
                return Ok(None);
            }
            let start = self.pop_token()?; // drop [
            let arg = self.consume_arg(Some(&vec!["]".to_owned()]))?;
            start_tok = start;
            end_tok = arg.end;
            tokens = arg.tokens;
        } else {
            let arg = self.consume_arg(None)?;
            start_tok = arg.start;
            end_tok = arg.end;
            tokens = arg.tokens;
        }

        // indicate the end of an argument
        self.push_token(Token::new("EOF", None));
        self.push_tokens(tokens);

        // compute range token with empty text
        // We drop location info in this port; just synthesize an empty token
        Ok(start_tok.range(end_tok, String::new()))
    }

    /// Consume specified number of arguments with optional delimiters
    fn consume_args_with_delims(
        &mut self,
        num_args: usize,
        delimiters: Option<&Vec<Vec<String>>>,
    ) -> Result<Vec<Vec<Token>>, ParseError> {
        if let Some(d) = delimiters {
            if d.len() != num_args + 1 {
                return Err(ParseError::new(
                    ParseErrorKind::MacroDelimiterLengthMismatch,
                ));
            }
            for expected in &d[0] {
                let tok = self.pop_token()?;
                if expected != &tok.text {
                    return Err(ParseError::with_token(
                        ParseErrorKind::MacroDefinitionMismatch,
                        &tok,
                    ));
                }
            }
        }

        let mut args: Vec<Vec<Token>> = Vec::new();
        for i in 0..num_args {
            let delims_for_arg = delimiters.as_ref().map(|v| &v[i + 1]);
            let arg = self.consume_arg(delims_for_arg)?;
            args.push(arg.tokens);
        }
        Ok(args)
    }

    /// Increment expansion counter and check against max_expand
    fn count_expansion(&mut self, amount: usize) -> Result<(), ParseError> {
        self.expansion_count += amount;
        if self.expansion_count > self.settings.max_expand {
            return Err(ParseError::new(ParseErrorKind::MacroTooManyExpansions));
        }
        Ok(())
    }

    /// Expand the next token only once if possible
    fn expand_once_internal(&mut self, expandable_only: bool) -> Result<Option<isize>, ParseError> {
        let top_token = self.pop_token()?;
        let name = top_token.text.to_owned_string();
        let expansion = if top_token.noexpand == Some(true) {
            None
        } else {
            self.get_expansion(&name)
        };

        let expansion = match expansion {
            Some(exp) if !(expandable_only && exp.unexpandable == Some(true)) => exp,
            _ => {
                if expandable_only
                    && expansion.is_none()
                    && name.starts_with('\\')
                    && !self.is_defined(&name)
                {
                    return Err(ParseError::with_token(
                        ParseErrorKind::UndefinedControlSequence { name: name.clone() },
                        &top_token,
                    ));
                }
                self.push_token(top_token);
                return Ok(None);
            }
        };

        self.count_expansion(1)?;
        let mut tokens = expansion.tokens.clone();
        let args =
            self.consume_args_with_delims(expansion.num_args, expansion.delimiters.as_ref())?;
        if expansion.num_args > 0 {
            // Paste arguments in place of placeholders
            let mut i = (tokens.len() as isize) - 1;
            while i >= 0 {
                if tokens[i as usize].text == "#" {
                    if i == 0 {
                        return Err(ParseError::with_token(
                            ParseErrorKind::MacroIncompletePlaceholder,
                            &tokens[i as usize],
                        ));
                    }
                    let tok = tokens[(i - 1) as usize].clone();
                    if tok.text == "#" {
                        // ## -> #
                        tokens.remove(i as usize);
                        i -= 2;
                        continue;
                    } else if tok.text.len() == 1
                        && let Ok(parsed) = tok.text.as_str().parse::<usize>()
                    {
                        let arg_index = parsed - 1;
                        // replace placeholder (#n) with arg tokens
                        // remove the two tokens (# and n) at positions i-1 and i
                        tokens.splice((i as usize - 1)..=(i as usize), args[arg_index].clone());
                        i -= 2; // step past inserted
                        continue;
                    }

                    return Err(ParseError::with_token(
                        ParseErrorKind::InvalidMacroArgumentNumber {
                            value: tok.text.to_owned_string(),
                        },
                        &tok,
                    ));
                }
                i -= 1;
            }
        }
        self.push_tokens(tokens.clone());
        Ok(Some(tokens.len() as isize))
    }

    /// Fully expand the given token stream to forward-order tokens
    fn expand_tokens_internal(&mut self, tokens: Vec<Token>) -> Result<Vec<Token>, ParseError> {
        let mut output: Vec<Token> = Vec::new();
        let old_len = self.stack.len();
        self.push_tokens(tokens);
        while self.stack.len() > old_len {
            if self.expand_once_internal(true)?.is_none() {
                let mut token = self
                    .stack
                    .pop()
                    .ok_or_else(|| ParseError::new(ParseErrorKind::MacroStackUnexpectedlyEmpty))?;
                if token.treat_as_relax == Some(true) {
                    // the expansion of \noexpand is the token itself
                    token.noexpand = Some(false);
                    token.treat_as_relax = Some(false);
                }
                output.push(token);
            }
        }
        self.count_expansion(output.len())?;
        Ok(output)
    }

    /// Compute expansion for a name
    fn get_expansion(&mut self, name: &str) -> Option<MacroExpansion> {
        // If single character has a catcode other than 13 (active), don't expand it
        if name.chars().count() == 1
            && let Some(ch) = name.chars().next()
            && let Some(catcode) = self.lexer.get_catcode(ch)
            && catcode != 13
        {
            return None;
        }

        let definition = self.macros.get(name)?.clone();

        match definition {
            MacroDefinition::Function(f) => match f(self as &mut dyn MacroContextInterface) {
                Ok(MacroExpansionResult::String(s)) => Some(self.string_to_expansion(&s)),
                Ok(MacroExpansionResult::Expansion(e)) => Some(e),
                Ok(MacroExpansionResult::Empty) => Some(MacroExpansion::default()),
                Err(_) => None,
            },
            MacroDefinition::StaticFunction(f) => match f(self as &mut dyn MacroContextInterface) {
                Ok(MacroExpansionResult::String(s)) => Some(self.string_to_expansion(&s)),
                Ok(MacroExpansionResult::Expansion(e)) => Some(e),
                Ok(MacroExpansionResult::Empty) => Some(MacroExpansion::default()),
                Err(_) => None,
            },
            MacroDefinition::StaticStr(s) => Some(self.string_to_expansion(s)),
            MacroDefinition::String(s) => Some(self.string_to_expansion(&s)),
            MacroDefinition::Expansion(e) => Some(e),
        }
    }

    fn string_to_expansion(&self, expansion: &str) -> MacroExpansion {
        let mut num_args = 0usize;
        if expansion.contains('#') {
            let stripped = expansion.replace("##", "");
            while stripped.contains(&format!("#{}", num_args + 1)) {
                num_args += 1;
            }
        }

        let mut body_lexer = Lexer::new(Arc::from(expansion), self.settings);
        let mut tokens: Vec<Token> = Vec::new();
        while let Ok(tok) = body_lexer.lex() {
            if tok.text == "EOF" {
                break;
            }
            tokens.push(tok);
        }
        tokens.reverse();
        MacroExpansion {
            tokens,
            num_args,
            delimiters: None,
            unexpandable: None,
        }
    }
}

impl<'a> MacroContextInterface<'a> for MacroExpander<'a> {
    fn mode(&self) -> Mode {
        self.mode
    }

    fn context(&self) -> &KatexContext {
        self.ctx
    }

    fn macros<'s>(&'s self) -> &'s Namespace<'a, MacroDefinition> {
        &self.macros
    }

    fn macros_mut<'s>(&'s mut self) -> &'s mut Namespace<'a, MacroDefinition> {
        &mut self.macros
    }

    fn future_mut(&mut self) -> Result<Token, ParseError> {
        if self.stack.is_empty() {
            let tok = self.lexer.lex()?;
            self.push_token(tok);
        }
        self.stack
            .last()
            .cloned()
            .ok_or_else(|| ParseError::new(ParseErrorKind::EmptyMacroExpanderStack))
    }

    fn pop_token(&mut self) -> Result<Token, ParseError> {
        self.future_mut()?;
        self.stack
            .pop()
            .ok_or_else(|| ParseError::new(ParseErrorKind::EmptyMacroExpanderStack))
    }

    fn consume_spaces(&mut self) -> Result<(), ParseError> {
        loop {
            let token = self.future_mut()?;
            if token.text == " " {
                self.stack.pop();
            } else {
                break;
            }
        }
        Ok(())
    }

    fn expand_once(&mut self, expandable_only: Option<bool>) -> Result<Option<isize>, ParseError> {
        self.expand_once_internal(expandable_only.unwrap_or(false))
    }

    fn expand_after_future(&mut self) -> Result<Token, ParseError> {
        self.expand_once_internal(false)?;
        self.future_mut()
    }

    fn expand_next_token(&mut self) -> Result<Token, ParseError> {
        loop {
            if self.expand_once_internal(false)?.is_none() {
                let mut token = self
                    .stack
                    .pop()
                    .ok_or_else(|| ParseError::new(ParseErrorKind::MacroStackUnexpectedlyEmpty))?;
                if token.treat_as_relax == Some(true) {
                    token.set_text("\\relax");
                }
                return Ok(token);
            }
        }
    }

    fn expand_macro(&mut self, name: &str) -> Result<Option<Vec<Token>>, ParseError> {
        if self.macros.has(name) {
            let toks = self.expand_tokens_internal(vec![Token::new(name.to_owned(), None)])?;
            Ok(Some(toks))
        } else {
            Ok(None)
        }
    }

    fn expand_macro_as_text(&mut self, name: &str) -> Result<Option<String>, ParseError> {
        self.expand_macro(name)?.map_or(Ok(None), |tokens| {
            Ok(Some(
                tokens
                    .into_iter()
                    .map(|t| String::from(t.text))
                    .collect::<String>(),
            ))
        })
    }

    fn expand_tokens(&mut self, tokens: Vec<Token>) -> Result<Vec<Token>, ParseError> {
        let toks: Vec<Token> = tokens.into_iter().collect();
        self.expand_tokens_internal(toks)
    }

    fn consume_arg(&mut self, delims: Option<&Vec<String>>) -> Result<MacroArg, ParseError> {
        let mut tokens: Vec<Token> = Vec::new();
        let is_delimited = delims.as_ref().is_some_and(|d| !d.is_empty());
        if !is_delimited {
            self.consume_spaces()?;
        }
        let start = self.future_mut()?;
        let mut tok;
        let mut depth: isize = 0;
        let mut match_idx: usize = 0;
        loop {
            tok = self.pop_token()?;
            tokens.push(tok.clone());
            if tok.text == "{" {
                depth += 1;
            } else if tok.text == "}" {
                depth -= 1;
                if depth == -1 {
                    return Err(ParseError::with_token(
                        ParseErrorKind::ExtraCloseBrace,
                        &tok,
                    ));
                }
            } else if tok.text == "EOF" {
                let expected = delims.as_ref().map_or("}", |d| {
                    if is_delimited {
                        d.get(match_idx).map_or("}", String::as_str)
                    } else {
                        "}"
                    }
                });
                return Err(ParseError::with_token(
                    ParseErrorKind::UnexpectedEndOfMacroArgument {
                        expected: expected.to_owned(),
                    },
                    &tok,
                ));
            }
            if let Some(d) = &delims
                && is_delimited
            {
                if (depth == 0 || (depth == 1 && d[match_idx] == "{")) && tok.text == d[match_idx] {
                    match_idx += 1;
                    if match_idx == d.len() {
                        // don't include delimiters
                        let keep = tokens.len() - match_idx;
                        tokens.truncate(keep);
                        break;
                    }
                } else {
                    match_idx = 0;
                }
            }
            if depth == 0 && !is_delimited {
                // undelimited arg: stop after a single token or a {...} group
                if start.text != "{" || tok.text == "}" {
                    break;
                }
            }
            if depth == 0 && is_delimited {
                // keep going until delimiters matched
            }
        }

        // Remove outermost braces if present
        if start.text == "{" && tokens.last().map(|t| t.text.as_str()) == Some("}") {
            tokens.pop();
            if !tokens.is_empty() {
                tokens.remove(0);
            }
        }
        tokens.reverse();
        Ok(MacroArg {
            tokens,
            start,
            end: tok,
        })
    }

    fn consume_args(&mut self, num_args: usize) -> Result<Vec<Vec<Token>>, ParseError> {
        self.consume_args_with_delims(num_args, None)
    }

    fn is_defined(&self, name: &str) -> bool {
        if self.macros.has(name) {
            return true;
        }

        if self.ctx.functions.contains_key(name) {
            return true;
        }

        if IMPLICIT_COMMANDS.contains(name) {
            return true;
        }

        let symbols = &self.ctx.symbols;
        symbols.contains(Mode::Math, name) || symbols.contains(Mode::Text, name)
    }

    fn is_expandable(&self, name: &str) -> bool {
        if let Some(def) = self.macros.get(name) {
            match def {
                MacroDefinition::Expansion(e) => e.unexpandable != Some(true),
                _ => true,
            }
        } else {
            if let Some(md) = self.macros.get(name) {
                return match md {
                    MacroDefinition::Expansion(e) => !e.unexpandable.unwrap_or(false),
                    _ => true,
                };
            }

            self.ctx.functions.contains_key(name) && !self.ctx.functions[name].primitive
        }
    }

    fn begin_group(&mut self) {
        self.macros.begin_group();
    }

    fn end_group(&mut self) -> Result<(), ParseError> {
        self.macros.end_group()
    }
}
