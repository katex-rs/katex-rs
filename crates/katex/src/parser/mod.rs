use alloc::borrow::Cow;
use core::iter;

use crate::parser::parse_node::ParseNodeTextOrd;
use crate::types::{SourceRangeRef as _, TokenText};
use crate::unicode::unicode_sup_or_sub::U_SUBS_AND_SUPS;
use crate::{
    KatexContext, ParseError, Settings,
    define_function::FunctionContext,
    lexer::last_non_combining_mark_index,
    macro_expander::{IMPLICIT_COMMANDS, MacroExpander},
    macros::{MacroContextInterface as _, MacroDefinition},
    parser::parse_node::{AnyParseNode, NodeType, ParseNode, ParseNodeSize},
    style::TEXT,
    symbols::{Group, NonAtom},
    types::{ArgType, BreakToken, ErrorLocationProvider, Mode, ParseErrorKind, Spec, Token},
    unicode::{UNICODE_SYMBOLS, get_accent_mapping, supported_codepoint},
};

pub mod parse_node;
use crate::spacing_data::MeasurementOwned;
use crate::unicode::is_unicode_subscript;
use crate::units::valid_unit;
pub use parse_node::ParseNodeError;

/// The core parser for KaTeX, responsible for converting LaTeX mathematical
/// expressions into an abstract syntax tree (AST) of parse nodes.
///
/// # Parsing Strategy
///
/// The parser employs a recursive descent approach with lookahead tokens:
/// - Main parsing functions (e.g., parse, parse_expression) consume tokens
///   sequentially
/// - The lexer ([`MacroExpander`]) provides tokens on demand, supporting
///   arbitrary position access
/// - Mode switching between "math" and "text" contexts restricts available
///   commands
/// - Functions return [`ParseNode`] objects representing parsed structures
///
/// # LaTeX Command Handling
///
/// Supports comprehensive LaTeX math commands including:
/// - Superscripts/subscripts (`^`, `_`)
/// - Fractions (`\frac`, `\over`, `\choose`)
/// - Delimiters (`\left`, `\right`)
/// - Symbols and operators from the symbol table
/// - Functions with argument parsing
/// - Unicode superscript/subscript handling
///
/// # TeX Parsing Strategies
///
/// - **Token lookahead**: Maintains a single lookahead token for efficient
///   parsing
/// - **Mode enforcement**: Validates command availability in current context
/// - **Infix operator rewriting**: Converts `\over`, `\choose` into structured
///   fractions
/// - **Ligature formation**: Combines ASCII sequences in text mode
/// - **Error recovery**: Provides detailed error messages with token locations
///
/// # Error Handling
///
/// Returns `ParseError` for syntax errors, undefined commands, and mode
/// violations. Errors include token location information for precise error
/// reporting.
///
/// # Cross-references
///
/// - [`parse_node`] - AST node types
/// - [`MacroExpander`] - Token stream and macro handling
/// - [`Mode`] - Parsing context modes
/// - `ParseError` - Error types
pub struct Parser<'a> {
    /// Current parsing mode ([`Mode::Math`] or [`Mode::Text`])
    pub mode: Mode,
    ///  Token stream provider and macro expander
    pub gullet: MacroExpander<'a>,
    /// Global parsing configuration
    pub settings: &'a Settings,
    /// Nesting depth for `\left`/`\right` pairs
    pub leftright_depth: f64,
    /// Cached lookahead token
    pub next_token: Option<Token>,
    /// Shared context containing functions and symbols
    pub ctx: &'a KatexContext,
}

#[inline]
fn wrap_ordgroup(mut nodes: Vec<ParseNode>, mode: Mode) -> ParseNode {
    if nodes.len() == 1
        && let ParseNode::OrdGroup(_) = nodes[0]
    {
        return nodes.remove(0);
    }
    ParseNode::OrdGroup(parse_node::ParseNodeOrdGroup {
        mode,
        loc: None,
        body: nodes,
        semisimple: None,
    })
}

#[inline]
fn parse_size_with_unit(s: &str) -> Option<(f64, String)> {
    let mut chars = s.chars().peekable();

    let mut sign = 1.0;
    if let Some(&c) = chars.peek() {
        if c == '+' {
            chars.next();
        } else if c == '-' {
            sign = -1.0;
            chars.next();
        }
    }

    while matches!(chars.peek(), Some(' ')) {
        chars.next();
    }

    let mut number_str = String::new();
    let mut saw_digit_before_dot = false;
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        saw_digit_before_dot = true;
        number_str.push(chars.next()?);
    }
    if matches!(chars.peek(), Some('.')) {
        number_str.push('.');
        chars.next();
        let mut digit_after_dot = false;
        while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
            digit_after_dot = true;
            number_str.push(chars.next()?);
        }
        if !saw_digit_before_dot && !digit_after_dot {
            return None;
        }
    } else if !saw_digit_before_dot {
        return None;
    }

    while matches!(chars.peek(), Some(' ')) {
        chars.next();
    }

    let mut unit = String::new();
    for _ in 0..2 {
        if let Some(c) = chars.next() {
            if c.is_ascii_lowercase() {
                unit.push(c);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    let number_val: f64 = number_str.parse().ok()?;
    Some((sign * number_val, unit))
}

impl<'a> Parser<'a> {
    /// Creates a new parser instance initialized with the provided input
    /// string, settings, and context. This is the primary constructor for
    /// parsing LaTeX mathematical expressions.
    ///
    /// The parser starts in mathematical mode by default, with a fresh macro
    /// expander and no lookahead token cached. The input string is tokenized
    /// on-demand through the lexer component.
    ///
    /// # Parameters
    ///
    /// * `input` - The LaTeX source string to parse (e.g., `"x^2 + \\sqrt{y}"`)
    /// * `settings` - Configuration options affecting parsing behavior, such as
    ///   color handling and global grouping
    /// * `ctx` - Shared context containing function definitions, symbol tables,
    ///   and other parsing resources
    ///
    /// # Return Value
    ///
    /// Returns a fully initialized [`Parser`] ready to parse the input string.
    ///
    /// # Error Handling
    ///
    /// This constructor cannot fail, but subsequent parsing operations may
    /// return `ParseError` for invalid input or configuration issues.
    ///
    /// # Cross-references
    ///
    /// - [`Settings`] - Configuration structure
    /// - [`KatexContext`] - Shared parsing context
    #[must_use]
    pub fn new(input: &'a str, settings: &'a Settings, ctx: &'a KatexContext) -> Self {
        let mode = Mode::Math;
        let gullet = MacroExpander::new(input, settings, mode, ctx);

        Self {
            // Start in math mode
            mode,
            // Create a new macro expander (gullet) and (indirectly via that) also a
            // new lexer (mouth) for this parser (stomach, in the language of TeX)
            gullet,
            // Store the settings for use in parsing
            settings,
            // Count leftright depth (for \middle errors)
            leftright_depth: 0.0,
            next_token: None,

            ctx,
        }
    }

    /// Checks a result to make sure it has the right type, and throws an
    /// appropriate error otherwise.
    pub fn expect(&mut self, text: &str, consume: bool) -> Result<(), ParseError> {
        let token = self.fetch()?;
        if token.text != text {
            return Err(ParseError::with_token(
                ParseErrorKind::ExpectedToken {
                    expected: text.to_owned(),
                    found: token.text.to_owned_string(),
                },
                token,
            ));
        }
        if consume {
            self.consume();
        }
        Ok(())
    }

    /// Consumes the current lookahead token, advancing the parser state.
    ///
    /// This method discards the cached lookahead token (if any) and marks it as
    /// processed. The next call to fetch will retrieve a new token from the
    /// input stream. This is essential for progressing through the token
    /// sequence during parsing.
    pub fn consume(&mut self) {
        self.next_token = None;
    }

    /// Retrieves the current lookahead token, fetching a new one if necessary.
    ///
    /// This method implements the parser's lookahead mechanism. If a token is
    /// already cached in the lookahead buffer, it returns that token.
    /// Otherwise, it requests the next token from the macro expander and
    /// caches it for future use.
    ///
    /// # Return Value
    ///
    /// Returns a reference to the current [`Token`] if available, or an error
    /// if tokenization fails (e.g., due to macro expansion errors or end of
    /// input).
    ///
    /// # Behavior
    ///
    /// - Returns cached token if `next_token` is `Some`
    /// - Fetches new token from [`MacroExpander`] if cache is empty
    /// - The returned token remains cached until consume is called
    /// - Multiple calls without consuming return the same token
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` for:
    /// - Macro expansion failures
    /// - Lexer errors during tokenization
    /// - Unexpected end of input in certain contexts
    ///
    /// # Cross-references
    ///
    /// - consume - Consumes the current lookahead token
    /// - [`Token`] - Token structure
    /// - [`MacroExpander`] - Token source
    pub fn fetch(&mut self) -> Result<&Token, ParseError> {
        match &mut self.next_token {
            Some(next_token) => Ok(next_token),
            next_token => {
                let token = self.gullet.expand_next_token()?;
                let t_ref: &Token = next_token.get_or_insert(token);
                Ok(t_ref)
            }
        }
    }

    /// Changes the parser's current parsing mode, affecting available commands
    /// and behavior.
    ///
    /// LaTeX has two primary modes: mathematical mode for equations and
    /// symbols, and text mode for regular text content. This method
    /// switches between them, updating both the parser's internal state and
    /// the macro expander's mode.
    ///
    /// # Parameters
    ///
    /// * `new_mode` - The target mode to switch to ([`Mode::Math`] or
    ///   [`Mode::Text`])
    ///
    /// # Mode Differences
    ///
    /// **Math Mode ([`Mode::Math`]):**
    /// - Allows mathematical symbols, operators, and commands
    /// - Spaces are ignored between tokens
    /// - Superscripts/subscripts are permitted
    /// - Functions like `\sqrt`, `\frac` are available
    ///
    /// **Text Mode ([`Mode::Text`]):**
    /// - Supports text formatting and regular characters
    /// - Spaces are preserved
    /// - Limited mathematical commands (mainly `\text` and similar)
    /// - Enables ligature formation for typography
    ///
    /// # Cross-references
    ///
    /// - [`Mode`] - Enumeration of parsing modes
    /// - parse_expression - Expression parsing affected by mode
    /// - [`MacroExpander::switch_mode`] - Underlying mode switching
    pub const fn switch_mode(&mut self, new_mode: Mode) {
        self.mode = new_mode;
        self.gullet.switch_mode(new_mode);
    }

    /// Parses the entire input string into an abstract syntax tree (AST).
    ///
    /// This is the primary entry point for parsing LaTeX mathematical
    /// expressions. It processes the complete input from start to finish,
    /// handling macro expansion, expression parsing, and AST construction.
    /// The result is a vector of parse nodes wrapped in an OrdGroup to
    /// match KaTeX's top-level structure.
    ///
    /// # Processing Steps
    ///
    /// 1. **Group Setup**: Creates a namespace group for the expression (unless
    ///    `global_group` is enabled)
    /// 2. **Color Handling**: Applies `\color` behavior settings
    /// 3. **Expression Parsing**: Calls parse_expression to parse the content
    /// 4. **Validation**: Ensures the entire input is consumed (ends with EOF)
    /// 5. **Cleanup**: Closes any open groups and wraps result in OrdGroup
    ///
    /// # Return Value
    ///
    /// Returns a vector of [`ParseNode`] representing the AST on success, or a
    /// `ParseError` if parsing fails at any stage. The vector is
    /// typically wrapped in an OrdGroup for top-level expressions.
    ///
    /// # Error Handling
    ///
    /// Common error scenarios:
    /// - Syntax errors in LaTeX commands or expressions
    /// - Unmatched delimiters (`\left` without `\right`)
    /// - Undefined macros or functions
    /// - Mode violations (e.g., math commands in text mode)
    /// - Unexpected end of input
    ///
    /// # Examples
    ///
    /// Basic mathematical expression:
    /// ```rust
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings};
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("E = mc^2", &settings, &ctx);
    /// let ast = parser.parse().unwrap();
    /// // ast is a vector containing an OrdGroup with the parsed expression
    /// ```
    ///
    /// Complex expression with fractions:
    /// ```rust
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings};
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("\\frac{a}{b} + \\sqrt{x}", &settings, &ctx);
    /// match parser.parse() {
    ///     Ok(nodes) => println!("Parsed successfully: {} nodes", nodes.len()),
    ///     Err(e) => println!("Parse error: {}", e),
    /// }
    /// ```
    ///
    /// # Cross-references
    ///
    /// - parse_expression - Core expression parsing logic
    /// - [`ParseNode`] - AST node types
    /// - `ParseError` - Error types
    /// - [`Settings::global_group`] - Affects group creation behavior
    pub fn parse(&mut self) -> Result<Vec<ParseNode>, ParseError> {
        if !self.settings.global_group {
            // Create a group namespace for the math expression.
            // (LaTeX creates a new group for every $...$, $$...$$, \[...\].)
            self.gullet.begin_group();
        }

        // Use old \color behavior (same as LaTeX's \textcolor) if requested.
        // We do this within the group for the math expression, so it doesn't
        // pollute settings.macros.
        if self.settings.color_is_text_color {
            self.gullet.macros_mut().set(
                "\\color",
                Some(MacroDefinition::StaticStr("\\textcolor")),
                false,
            );
        }

        // Try to parse the input and ensure groups are closed even on error.
        let body = match self.parse_expression(false, None) {
            Ok(b) => b,
            Err(e) => {
                self.gullet.end_groups();
                return Err(e);
            }
        };

        if let Err(e) = self.expect("EOF", true) {
            self.gullet.end_groups();
            return Err(e);
        }

        if !self.settings.global_group
            && let Err(e) = self.gullet.end_group()
        {
            self.gullet.end_groups();
            return Err(e);
        }

        // Close any leftover groups
        self.gullet.end_groups();

        // Wrap result in OrdGroup to match KaTeX's top-level structure
        Ok(body)
    }

    /// Parses a sequence of atoms into an expression list.
    ///
    /// An expression in LaTeX parsing context is a sequence of atomic elements
    /// (symbols, functions, groups) that form a mathematical or textual unit.
    /// This method continues parsing until it encounters an end condition or
    /// reaches the end of input.
    ///
    /// # Parameters
    ///
    /// * `break_on_infix` - If `true`, stops parsing when encountering infix
    ///   operators (like `\over`, `\choose`) to allow higher-precedence
    ///   functions to handle them. Used for operator precedence in nested
    ///   expressions.
    ///
    /// * `break_on_token_text` - Optional token text that terminates the
    ///   expression. Common terminators include `"}"`, `"\endgroup"`, `"\end"`,
    ///   `"\right"`, `"&"`.
    ///
    /// # Return Value
    ///
    /// Returns a vector of [`ParseNode`] representing the parsed atoms. The
    /// result may be empty if no atoms are found before an end condition.
    ///
    /// # Parsing Behavior
    ///
    /// - **Space Handling**: Consumes spaces in math mode, preserves in text
    ///   mode
    /// - **End Conditions**: Stops at EOF, end-of-expression tokens, or break
    ///   tokens
    /// - **Infix Detection**: Checks for infix operators when `break_on_infix`
    ///   is true
    /// - **Atom Parsing**: Calls `parse_atom` for each atomic element
    /// - **Ligature Formation**: Applies typographic ligatures in text mode
    /// - **Infix Rewriting**: Converts infix operators to structured forms
    ///
    /// # Infix Operator Handling
    ///
    /// When `break_on_infix` is `false`, infix operators like `\over` are
    /// rewritten into `Genfrac` nodes with appropriate delimiters:
    /// - `\over` → fraction with bar line
    /// - `\choose` → fraction with parentheses
    /// - `\above` → fraction with bar line (size parsing not yet implemented)
    ///
    /// # Examples
    ///
    /// Basic expression parsing:
    /// ```rust
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings};
    ///
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("a + b \\cdot c", &settings, &ctx);
    /// let expr = parser.parse_expression(false, None).unwrap();
    /// // Returns vector of MathOrd("a"), MathOrd("+"), MathOrd("b"), etc.
    /// ```
    ///
    /// Breaking on infix operators:
    /// ```rust
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings};
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("a \\over b + c", &settings, &ctx);
    /// let expr = parser.parse_expression(true, None).unwrap();
    /// // Stops at \over, allowing parent function to handle precedence
    /// ```
    ///
    /// Parsing until specific token:
    /// ```rust
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings, types::BreakToken};
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("x + y }", &settings, &ctx);
    /// let expr = parser
    ///     .parse_expression(false, Some(&BreakToken::RightBrace))
    ///     .unwrap();
    /// // Stops at "}", leaving it for outer parser
    /// ```
    ///
    /// # Cross-references
    ///
    /// - parse_atom - Parses individual atomic elements
    /// - handle_infix_nodes - Rewrites infix operators
    /// - form_ligatures - Applies text ligatures
    /// - [`BreakToken`] - Expression termination tokens
    pub fn parse_expression(
        &mut self,
        break_on_infix: bool,
        break_on_token_text: Option<&BreakToken>,
    ) -> Result<Vec<ParseNode>, ParseError> {
        let mut body: Vec<ParseNode> = Vec::new();

        // Keep adding atoms to the body until we can't parse any more atoms (either
        // we reached the end, a }, or a \right)
        loop {
            // Ignore spaces in math mode
            if self.mode == Mode::Math {
                self.consume_spaces()?;
            }

            let (should_break, infix_candidate) = {
                let token = self.fetch()?;
                let lex_text = token.text.as_str();
                let end_of_expression =
                    matches!(lex_text, "}" | "\\endgroup" | "\\end" | "\\right" | "&");
                let explicit_break = match break_on_token_text {
                    Some(BreakToken::RightBrace) => lex_text == "}",
                    Some(BreakToken::EndGroup) => lex_text == "\\endgroup",
                    Some(BreakToken::End) => lex_text == "\\end",
                    Some(BreakToken::RightBracket) => lex_text == "]",
                    Some(BreakToken::RightParen) => lex_text == "\\)",
                    Some(BreakToken::DoubleBackslash) => lex_text == "\\\\",
                    Some(BreakToken::Dollar) => lex_text == "$",
                    Some(BreakToken::Eof) => lex_text == "EOF",
                    None => false,
                };
                let should_stop = end_of_expression || explicit_break;
                let candidate = (!should_stop && break_on_infix).then(|| token.text.clone());
                (should_stop, candidate)
            };

            let should_break = if should_break {
                true
            } else if let Some(token_text) = infix_candidate {
                self.ctx
                    .functions
                    .get(token_text.as_str())
                    .is_some_and(|func| func.infix)
            } else {
                false
            };

            if should_break {
                break;
            }

            // Parse one atom
            if let Some(atom) = self.parse_atom(break_on_token_text)? {
                // Internal nodes do not appear in parse tree
                if let ParseNode::Internal(_) = atom {
                    continue;
                }
                body.push(atom);
            } else {
                break;
            }
        }

        // In text mode, turn common ASCII sequences into ligatures.
        if self.mode == Mode::Text {
            self.form_ligatures(&mut body);
        }

        self.handle_infix_nodes(body)
    }

    /// Consumes consecutive space tokens, advancing to the next non-space
    /// token.
    ///
    /// In LaTeX mathematical mode, spaces between tokens are typically ignored
    /// and don't affect the output. This method efficiently skips over any
    /// whitespace tokens, positioning the parser at the next meaningful token.
    ///
    /// # Behavior
    ///
    /// - Repeatedly fetches and consumes tokens that are TokenType::Space
    /// - Stops when a non-space token is encountered (becomes the new
    ///   lookahead)
    /// - Does nothing if the current lookahead is already non-space
    /// - Safe to call at any point during parsing
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` on success, or `ParseError` if token fetching fails
    /// (e.g., due to macro expansion errors).
    ///
    /// # Examples
    ///
    /// Basic space consumption:
    /// ```rust
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings};
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("a   +   b", &settings, &ctx);
    /// let token = parser.fetch().unwrap();
    /// assert_eq!(token.text, "a");
    /// parser.consume(); // consume "a"
    /// parser.consume_spaces().unwrap(); // skip spaces
    /// let next = parser.fetch().unwrap();
    /// assert_eq!(next.text, "+");
    /// ```
    ///
    /// In expression parsing (automatic):
    /// ```rust
    /// // Spaces are automatically consumed in math mode
    /// use katex::parser::Parser;
    /// use katex::{KatexContext, Settings};
    /// let settings = Settings::default();
    /// let ctx = KatexContext::default();
    /// let mut parser = Parser::new("x   ^   2", &settings, &ctx);
    /// let expr = parser.parse_expression(false, None).unwrap();
    /// // Spaces between tokens are ignored
    /// ```
    ///
    /// # Cross-references
    ///
    /// - fetch - Retrieves the current lookahead token
    /// - consume - Consumes a single token
    /// - TokenType::Space - Space token type
    /// - parse_expression - Uses this method in math mode
    pub fn consume_spaces(&mut self) -> Result<(), ParseError> {
        while self.fetch()?.text == " " {
            self.consume();
        }
        Ok(())
    }

    // ------------------------
    // Helpers (Parser.js ports)
    // ------------------------

    /// Parse a base (group/function/symbol) and attach super/subscripts.
    /// Rough port of Parser.js parseAtom with limited coverage.
    fn parse_atom(
        &mut self,
        break_on_token_text: Option<&BreakToken>,
    ) -> Result<Option<ParseNode>, ParseError> {
        // 1) Parse the base via group/function/symbol helpers
        let mut base_opt = self.parse_group("atom", break_on_token_text)?;

        if let Some(base) = &base_opt
            && matches!(base, ParseNode::Internal(_))
        {
            // Internal nodes (e.g. \relax) cannot support super/subscripts.
            // Instead we will pick up super/subscripts with blank base next round.
            // No super/subscripts on internal nodes or text mode nodes
            return Ok(base_opt);
        }

        if self.mode == Mode::Text {
            return Ok(base_opt);
        }

        let mut superscript = None;
        let mut subscript = None;

        // 2) Handle superscripts/subscripts chain: ^, _, ', and Unicode sub/sup
        // In text mode, raw ^/_ should error (like KaTeX); we implement minimal check.
        loop {
            self.consume_spaces()?; // math mode ignores spaces, but safe in both
            let token = self.fetch()?;
            match token.text.as_str() {
                text if matches!(text, "\\limits" | "\\nolimits") => {
                    // Handle \limits and \nolimits
                    let limits = text == "\\limits";
                    if let Some(ParseNode::Op(base)) = &mut base_opt {
                        *base.limits_mut() = limits;
                        *base.always_handle_sup_sub_mut() = Some(true);
                    } else if let Some(ParseNode::OperatorName(base)) = &mut base_opt {
                        if base.always_handle_sup_sub {
                            base.limits = limits;
                        }
                    } else {
                        return Err(ParseError::with_token(
                            ParseErrorKind::LimitsMustFollowBase,
                            token,
                        ));
                    }
                    self.consume();
                }
                "^" => {
                    if superscript.is_some() {
                        return Err(ParseError::with_token(
                            ParseErrorKind::DoubleSuperscript,
                            token,
                        ));
                    }
                    superscript = Some(self.handle_sup_subscript("superscript")?);
                }
                "_" => {
                    if subscript.is_some() {
                        return Err(ParseError::with_token(
                            ParseErrorKind::DoubleSubscript,
                            token,
                        ));
                    }
                    subscript = Some(self.handle_sup_subscript("subscript")?);
                }
                "'" => {
                    if superscript.is_some() {
                        return Err(ParseError::with_token(
                            ParseErrorKind::DoubleSuperscript,
                            token,
                        ));
                    }
                    let mut n = 1;
                    self.consume();
                    while self.fetch()?.text == "'" {
                        n += 1;
                        self.consume();
                    }
                    let mut primes = iter::repeat_n(
                        ParseNode::TextOrd(ParseNodeTextOrd {
                            text: "\\prime".into(),
                            mode: self.mode,
                            loc: None,
                        }),
                        n,
                    )
                    .collect::<Vec<ParseNode>>();
                    if self.fetch()?.text == "^" {
                        primes.push(self.handle_sup_subscript("superscript")?);
                    }
                    superscript = Some(ParseNode::OrdGroup(parse_node::ParseNodeOrdGroup {
                        mode: self.mode,
                        loc: None,
                        body: primes,
                        semisimple: None,
                    }));
                }
                text => {
                    if let Some(ch) = text.chars().next()
                        && let Some(&mapped) = U_SUBS_AND_SUPS.get(&ch)
                    {
                        let is_sub = is_unicode_subscript(ch);
                        let mut subsup_tokens = vec![Token::new(mapped, None)];
                        self.consume();
                        loop {
                            let Some(c) = self.fetch()?.text.as_str().chars().next() else {
                                break;
                            };

                            if let Some(&mapped) = U_SUBS_AND_SUPS.get(&c)
                                && is_sub == is_unicode_subscript(c)
                            {
                                subsup_tokens.push(Token::new(mapped, None));
                                self.consume();
                            } else {
                                break;
                            }
                        }
                        subsup_tokens.reverse();
                        let body = self.subparse(subsup_tokens)?;
                        if is_sub {
                            subscript = Some(ParseNode::OrdGroup(parse_node::ParseNodeOrdGroup {
                                mode: Mode::Math,
                                loc: None,
                                body,
                                semisimple: None,
                            }));
                        } else {
                            superscript =
                                Some(ParseNode::OrdGroup(parse_node::ParseNodeOrdGroup {
                                    mode: Mode::Math,
                                    loc: None,
                                    body,
                                    semisimple: None,
                                }));
                        }
                    } else {
                        // If it wasn't ^, _, or ', stop parsing super/subscripts
                        break;
                    }
                }
            }
        }

        if superscript.is_some() || subscript.is_some() {
            return Ok(Some(ParseNode::SupSub(parse_node::ParseNodeSupSub {
                base: base_opt.map(Box::new),
                sup: superscript.map(Box::new),
                sub: subscript.map(Box::new),
                mode: self.mode,
                loc: None,
            })));
        }

        Ok(base_opt)
    }

    /// Rewrites infix operators such as \over and \choose with corresponding
    /// structures. Supports any infix operator, not just hardcoded ones.
    fn handle_infix_nodes(
        &mut self,
        mut body: Vec<ParseNode>,
    ) -> Result<Vec<ParseNode>, ParseError> {
        // Find any infix function in body
        let mut infix_pos: Option<usize> = None;
        let mut func_name: Option<String> = None;
        for (i, node) in body.iter().enumerate() {
            if let ParseNode::Infix(n) = node {
                if infix_pos.is_some() {
                    return Err(ParseError::with_token(
                        ParseErrorKind::MultipleInfixOperators,
                        &n.token,
                    ));
                }
                infix_pos = Some(i);
                func_name = Some(n.replace_with.clone());
            }
        }

        let (Some(over_idx), Some(func_name)) = (infix_pos, func_name) else {
            // No infix found, return original body
            return Ok(body);
        };

        let denom_body = body.split_off(over_idx + 1);
        let mut numer_body = body;
        let Some(infix_node) = numer_body.pop() else {
            return Err(ParseError::with_token(
                ParseErrorKind::InfixOperatorAtStart,
                &self.fetch()?.clone(),
            ));
        };

        let numer_node = wrap_ordgroup(numer_body, self.mode);
        let denom_node = wrap_ordgroup(denom_body, self.mode);

        let node = if func_name == r"\\abovefrac" {
            self.call_function(
                &func_name,
                vec![numer_node, infix_node, denom_node],
                vec![],
                None,
                None,
            )?
        } else {
            self.call_function(&func_name, vec![numer_node, denom_node], vec![], None, None)?
        };
        Ok(vec![node])
    }

    /// Combine ASCII sequences into typographic ligatures in text mode.
    /// Matches the JS implementation from KaTeX/src/Parser.js
    #[expect(clippy::unused_self)]
    fn form_ligatures(&self, group: &mut Vec<ParseNode>) {
        let mut n = group.len() as isize - 1;
        let mut i = 0usize;

        while (i as isize) < n {
            let a = group[i].clone();
            let v = a.text();
            if v == Some("-") && group[i + 1].text() == Some("-") {
                if (i as isize + 1) < n && group[i + 2].text() == Some("-") {
                    group.splice(
                        i..i + 3,
                        vec![ParseNode::TextOrd(parse_node::ParseNodeTextOrd {
                            mode: Mode::Text,
                            loc: a.loc().range_ref(group[i + 2].loc()),
                            text: "---".to_owned(),
                        })],
                    );
                    n -= 2;
                } else {
                    group.splice(
                        i..i + 2,
                        vec![ParseNode::TextOrd(parse_node::ParseNodeTextOrd {
                            mode: Mode::Text,
                            loc: a.loc().range_ref(group[i + 1].loc()),
                            text: "--".to_owned(),
                        })],
                    );
                    n -= 1;
                }
            }
            if let Some(ch) = v
                && (ch == "'" || ch == "`")
                && group[i + 1].text() == v
            {
                group.splice(
                    i..i + 2,
                    vec![ParseNode::TextOrd(parse_node::ParseNodeTextOrd {
                        mode: Mode::Text,
                        loc: a.loc().range_ref(group[i + 1].loc()),
                        text: format!("{ch}{ch}"),
                    })],
                );
                n -= 1;
            }
            i += 1;
        }
    }

    // ============ Additional parsing helpers (port of Parser.js) ============

    /// Parse a group using a validation function, similar to Parser.js
    /// parseRegexGroup. This is a general version that takes a validation
    /// function instead of regex.
    fn parse_regex_group<F, K>(
        &mut self,
        mut validator: F,
        build_error: K,
    ) -> Result<Token, ParseError>
    where
        F: FnMut(&str) -> bool,
        K: Fn(String) -> ParseErrorKind,
    {
        let first_token = self.fetch()?.clone();
        let mut last_token = first_token.clone();

        let mut str = String::new();

        while let next_token = self.fetch()?
            && next_token.text != "EOF"
        {
            let valid_length = str.len();
            str.push_str(next_token.text.as_str());
            if !validator(&str) {
                str.truncate(valid_length);
                break;
            }
            last_token = next_token.clone();
            self.consume();
        }

        if str.is_empty() {
            let value = first_token.text.to_owned_string();
            return Err(ParseError::with_token(build_error(value), &first_token));
        }

        // Create a new token with the combined text
        first_token
            .range(last_token, str)
            .ok_or_else(|| ParseError::new(ParseErrorKind::FailedToCreateCombinedToken))
    }

    /// Parse a string group from scan_argument; returns the concatenated token
    /// string.
    fn parse_string_group(
        &mut self,
        _mode_name: &str,
        optional: bool,
    ) -> Result<Option<Token>, ParseError> {
        let arg_token = self.gullet.scan_argument(optional)?;
        let Some(mut arg_token) = arg_token else {
            return Ok(None);
        };
        let mut s = String::new();
        loop {
            let next = self.fetch()?.clone();
            if next.text == "EOF" {
                break;
            }
            s.push_str(next.text.as_str());
            self.consume();
        }
        // consume EOF
        self.consume();
        arg_token.text = TokenText::Owned(s.into());
        Ok(Some(arg_token))
    }

    /// Parse a color specification; returns a color-token node
    fn parse_color_group(&mut self, optional: bool) -> Result<Option<ParseNode>, ParseError> {
        let res = self.parse_string_group("color", optional)?;
        let Some(tok) = res else { return Ok(None) };
        let mut text = tok.text.to_owned_string();
        let is_letters = text.chars().all(|c| c.is_ascii_alphabetic());
        let is_hash3 = text.starts_with('#')
            && text.len() == 4
            && text.chars().skip(1).all(|c| c.is_ascii_hexdigit());
        let is_hash6 = text.starts_with('#')
            && text.len() == 7
            && text.chars().skip(1).all(|c| c.is_ascii_hexdigit());
        let is_6hex = text.len() == 6 && text.chars().all(|c| c.is_ascii_hexdigit());
        if !(is_letters || is_hash3 || is_hash6 || is_6hex) {
            return Err(ParseError::with_token(
                ParseErrorKind::InvalidColor { color: text },
                &tok,
            ));
        }
        if is_6hex {
            text = format!("#{text}");
        }
        Ok(Some(ParseNode::ColorToken(
            parse_node::ParseNodeColorToken {
                mode: self.mode,
                loc: None,
                color: text,
            },
        )))
    }

    /// Parse a size specification.
    pub fn parse_size_group(
        &mut self,
        optional: bool,
    ) -> Result<Option<ParseNodeSize>, ParseError> {
        self.gullet.consume_spaces()?;
        let res = if !optional && self.gullet.future_mut()?.text != "{" {
            Some(self.parse_regex_group(
                |s| {
                    let t = s.trim();
                    let rest = if t.starts_with('+') || t.starts_with('-') {
                        &t[1..]
                    } else {
                        t
                    };
                    let rest = rest.trim_start();
                    if rest.is_empty() {
                        return true;
                    }
                    // Try to match number: \d+ | \d+\.\d* | \.\d*
                    let bytes = rest.as_bytes();
                    let mut i = 0;
                    let mut saw_digit = false;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        saw_digit = true;
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b'.' {
                        i += 1;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    } else if !saw_digit {
                        if bytes[0] == b'.' {
                            i = 1;
                            while i < bytes.len() && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                        } else {
                            return false;
                        }
                    }
                    let after_num = &rest[i..];
                    let after_num_trim = after_num.trim_start();
                    let mut j = 0;
                    while j < 2
                        && j < after_num_trim.len()
                        && after_num_trim.as_bytes()[j].is_ascii_lowercase()
                    {
                        j += 1;
                    }
                    let remaining = &after_num_trim[j..];
                    remaining.trim().is_empty()
                },
                |value| ParseErrorKind::InvalidSize { size: value },
            )?)
        } else {
            self.parse_string_group("size", optional)?
        };

        let Some(mut res) = res else { return Ok(None) };
        let is_blank = if !optional && res.text.is_empty() {
            res.text = TokenText::Static("0pt");
            true
        } else {
            false
        };

        let Some(matched) = parse_size_with_unit(res.text.as_str()) else {
            return Err(ParseError::with_token(
                ParseErrorKind::InvalidSize {
                    size: res.text.to_owned_string(),
                },
                &res,
            ));
        };

        let data = MeasurementOwned {
            number: matched.0,
            unit: matched.1,
        };

        if !valid_unit(&data) {
            return Err(ParseError::new(ParseErrorKind::InvalidUnit {
                unit: data.unit,
            }));
        }
        Ok(Some(ParseNodeSize {
            mode: self.mode,
            loc: None,
            value: data,
            is_blank,
        }))
    }

    /// Parse a URL group; simplified unescape.
    fn parse_url_group(&mut self, optional: bool) -> Result<Option<ParseNode>, ParseError> {
        // Set catcode for % to active character (13) and ~ to other character (12)
        // This follows hyperref package behavior
        self.gullet.set_catcode('%', 13); // active character
        self.gullet.set_catcode('~', 12); // other character

        let res = self.parse_string_group("url", optional)?;

        // Reset catcode for % to comment character (14) and ~ to active character (13)
        self.gullet.set_catcode('%', 14); // comment character
        self.gullet.set_catcode('~', 13); // active character

        let Some(tok) = res else { return Ok(None) };
        let mut url = String::new();
        let mut chars = tok.text.as_str().chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\'
                && let Some(&n) = chars.peek()
                && matches!(n, '#' | '$' | '%' | '&' | '~' | '_' | '^' | '{' | '}')
            {
                url.push(n);
                chars.next();
                continue;
            }
            // keep backslash if not recognized
            url.push(c);
        }
        Ok(Some(ParseNode::Url(parse_node::ParseNodeUrl {
            mode: self.mode,
            loc: None,
            url,
        })))
    }

    /// Parse an argument group (wrapped in ordgroup) with optional mode switch.
    fn parse_argument_group(
        &mut self,
        optional: bool,
        mode: Option<Mode>,
    ) -> Result<Option<ParseNode>, ParseError> {
        let Some(arg_token) = self.gullet.scan_argument(optional)? else {
            return Ok(None);
        };
        let outer_mode = self.mode;
        if let Some(m) = mode {
            self.switch_mode(m);
        }
        self.gullet.begin_group();
        let expression = self.parse_expression(false, Some(&BreakToken::Eof))?;
        self.expect("EOF", true)?;
        self.gullet.end_group()?;
        if mode.is_some() {
            self.switch_mode(outer_mode);
        }

        Ok(Some(ParseNode::OrdGroup(parse_node::ParseNodeOrdGroup {
            mode: self.mode,
            loc: arg_token.loc().cloned(),
            body: expression,
            semisimple: None,
        })))
    }

    /// Parse a group when the mode is changing or type-specific
    fn parse_group_of_type(
        &mut self,
        name: &str,
        arg_type: Option<&ArgType>,
        optional: bool,
    ) -> Result<Option<ParseNode>, ParseError> {
        match arg_type {
            Some(ArgType::Original) | None => self.parse_argument_group(optional, None),
            Some(ArgType::Color) => self.parse_color_group(optional),
            Some(ArgType::Size) => {
                let size = self.parse_size_group(optional)?;
                size.map_or(Ok(None), |s| Ok(Some(ParseNode::Size(s))))
            }
            Some(ArgType::Url) => self.parse_url_group(optional),
            Some(ArgType::Mode(mode)) => self.parse_argument_group(optional, Some(*mode)),
            Some(ArgType::Hbox) => self
                .parse_argument_group(optional, Some(Mode::Text))?
                .map_or(Ok(None), |group| {
                    Ok(Some(ParseNode::Styling(parse_node::ParseNodeStyling {
                        mode: group.mode(),
                        loc: None,
                        style: TEXT,
                        body: vec![group],
                    })))
                }),
            Some(ArgType::Raw) => {
                let token = self.parse_string_group("raw", optional)?;
                if let Some(t) = token {
                    Ok(Some(ParseNode::Raw(parse_node::ParseNodeRaw {
                        mode: Mode::Text,
                        loc: None,
                        string: t.text.to_owned_string(),
                    })))
                } else {
                    Ok(None)
                }
            }
            Some(ArgType::Primitive) => {
                if optional {
                    return Err(ParseError::new(
                        ParseErrorKind::PrimitiveArgumentCannotBeOptional,
                    ));
                }
                if let Some(group) = self.parse_group(name, None)? {
                    Ok(Some(group))
                } else {
                    let token = self.fetch()?;
                    Err(ParseError::with_token(
                        ParseErrorKind::ExpectedGroupAs {
                            context: name.to_owned(),
                        },
                        token,
                    ))
                }
            }
        }
    }

    /// Parses a group, returning ordgroup or result of function/symbol
    fn parse_group(
        &mut self,
        name: &str,
        break_on_token_text: Option<&BreakToken>,
    ) -> Result<Option<ParseNode>, ParseError> {
        let first_token = self.fetch()?.clone();
        let text = first_token.text.as_str();
        if text == "{" || text == "\\begingroup" {
            self.consume();
            let break_token = if text == "{" {
                BreakToken::RightBrace
            } else {
                BreakToken::EndGroup
            };

            self.gullet.begin_group();
            let expression = self.parse_expression(false, Some(&break_token))?;
            let last_loc = self.fetch()?.loc().cloned();
            self.expect(break_token.as_ref(), true)?;
            self.gullet.end_group()?;

            Ok(Some(ParseNode::OrdGroup(parse_node::ParseNodeOrdGroup {
                mode: self.mode,
                loc: first_token.loc().range_ref(last_loc.as_ref()),
                body: expression,
                // A group formed by \begingroup...\endgroup is a semi-simple group
                // which doesn't affect spacing in math mode, i.e., is transparent.
                semisimple: (text == "\\begingroup").then_some(true),
            })))
        } else {
            let result = self.parse_function(break_on_token_text, Some(name))?;
            let mut result = if result.is_some() {
                result
            } else {
                self.parse_symbol()?
            };

            if result.is_none()
                && let Some(first_char) = text.chars().next()
                && first_char == '\\'
                && !IMPLICIT_COMMANDS.contains(text)
            {
                if self.settings.throw_on_error {
                    return Err(ParseError::with_token(
                        ParseErrorKind::UndefinedControlSequence {
                            name: text.to_owned(),
                        },
                        &first_token,
                    ));
                }
                result = Some(self.format_unsupported_cmd(text).into());
                self.consume();
            }

            Ok(result)
        }
    }

    /// Convert textual input of an unsupported command into a color node
    /// containing a text node
    #[must_use]
    pub fn format_unsupported_cmd(&self, text: &str) -> parse_node::ParseNodeColor {
        let mut textord_array: Vec<AnyParseNode> = Vec::with_capacity(text.chars().count());
        for ch in text.chars() {
            textord_array.push(AnyParseNode::TextOrd(parse_node::ParseNodeTextOrd {
                mode: Mode::Text,
                loc: None,
                text: ch.to_string(),
            }));
        }
        let text_node = AnyParseNode::Text(parse_node::ParseNodeText {
            mode: self.mode,
            loc: None,
            body: textord_array,
            font: None,
        });
        parse_node::ParseNodeColor {
            mode: self.mode,
            loc: None,
            color: self.settings.error_color.clone(),
            body: vec![text_node],
        }
    }

    /// Parse a function if present at current token
    pub fn parse_function(
        &mut self,
        break_on_token_text: Option<&BreakToken>,
        name: Option<&str>,
    ) -> Result<Option<ParseNode>, ParseError> {
        let token = self.fetch()?.clone();
        let func = &token.text;
        let func_name = func.as_str();
        let Some(func_data) = self.ctx.functions.get(func_name) else {
            return Ok(None);
        };
        self.consume();

        // Check if function is allowed in argument context
        if let Some(name) = name
            && name != "atom"
            && !func_data.allowed_in_argument
        {
            return Err(ParseError::with_token(
                ParseErrorKind::FunctionMissingArguments {
                    func: func_name.to_owned(),
                    context: name.to_owned(),
                },
                &token,
            ));
        } else if self.mode == Mode::Text && !func_data.allowed_in_text {
            return Err(ParseError::with_token(
                ParseErrorKind::FunctionDisallowedInMode {
                    func: func_name.to_owned(),
                    mode: Mode::Text,
                },
                &token,
            ));
        } else if self.mode == Mode::Math && !func_data.allowed_in_math {
            return Err(ParseError::with_token(
                ParseErrorKind::FunctionDisallowedInMode {
                    func: func_name.to_owned(),
                    mode: Mode::Math,
                },
                &token,
            ));
        }

        let (args, opt_args) = self.parse_arguments(func_name, func_data.as_ref())?;
        let node =
            self.call_function(func_name, args, opt_args, Some(&token), break_on_token_text)?;
        Ok(Some(node))
    }

    /// Parse symbol at current token
    fn parse_symbol(&mut self) -> Result<Option<ParseNode>, ParseError> {
        let nucleus = self.fetch()?.clone();
        let mut text: Cow<'_, str> = Cow::Borrowed(nucleus.text.as_str());

        // Handle \verb commands
        if let Some(arg) = text.as_ref().strip_prefix("\\verb")
            && arg.chars().next().is_some_and(|c| !c.is_ascii_alphabetic())
        {
            self.consume();
            let star = arg.starts_with('*');
            let body = if star { &arg[1..] } else { arg };

            // Validate that body has matching delimiters
            if body.len() < 2 || body.chars().next() != body.chars().last() {
                return Err(ParseError::with_token(
                    ParseErrorKind::VerbAssertionFailed,
                    &nucleus,
                ));
            }

            // Extract content between delimiters
            let inner_body = &body[1..body.len() - 1];

            return Ok(Some(ParseNode::Verb(parse_node::ParseNodeVerb {
                mode: Mode::Text,
                loc: nucleus.loc,
                body: inner_body.to_owned(),
                star,
            })));
        }

        // Expand any accented base symbol according to unicodeSymbols.
        if let Some(first_char) = text.as_ref().chars().next()
            && let Some(mapped) = UNICODE_SYMBOLS.get(&first_char)
            && self
                .ctx
                .symbols
                .get(self.mode, &first_char.to_string())
                .is_none()
        {
            if self.mode == Mode::Math {
                // Report non-strict if configured
                self.settings.report_nonstrict(
                    "unicodeTextInMathMode",
                    &format!("Accented Unicode text character \"{first_char}\" used in math mode"),
                    nucleus
                        .loc
                        .as_ref()
                        .map(|loc| loc as &dyn ErrorLocationProvider),
                )?;
            }
            let rest: String = text.as_ref().chars().skip(1).collect();
            text = Cow::Owned(format!("{mapped}{rest}"));
        }

        // Strip off any trailing combining characters from the working text
        let matched = last_non_combining_mark_index(text.as_ref()).map(|strip_index| {
            let text_mut = text.to_mut();
            let accents = text_mut.split_off(strip_index);
            // Handle dotless i/j
            if text_mut == "i" {
                "\u{0131}".clone_into(text_mut);
            } else if text_mut == "j" {
                "\u{0237}".clone_into(text_mut);
            }
            accents
        });

        let text = text.into_owned();

        // Recognize base symbol via symbol table
        let mut symbol_node = if let Some(info) = self.ctx.symbols.get(self.mode, &text) {
            match info.group {
                Group::Atom(atom) => ParseNode::Atom(parse_node::ParseNodeAtom {
                    family: atom,
                    mode: self.mode,
                    loc: nucleus.loc.clone(),
                    text: text.clone(),
                }),
                Group::NonAtom(na) => match na {
                    NonAtom::MathOrd => ParseNode::MathOrd(parse_node::ParseNodeMathOrd {
                        mode: self.mode,
                        loc: nucleus.loc.clone(),
                        text: text.clone(),
                    }),
                    NonAtom::TextOrd => ParseNode::TextOrd(parse_node::ParseNodeTextOrd {
                        mode: self.mode,
                        loc: nucleus.loc.clone(),
                        text: text.clone(),
                    }),
                    NonAtom::Spacing => ParseNode::Spacing(parse_node::ParseNodeSpacing {
                        mode: self.mode,
                        loc: nucleus.loc.clone(),
                        text: text.clone(),
                    }),
                    NonAtom::AccentToken => {
                        ParseNode::AccentToken(parse_node::ParseNodeAccentToken {
                            mode: self.mode,
                            loc: nucleus.loc.clone(),
                            text: text.clone(),
                        })
                    }
                    NonAtom::OpToken => ParseNode::OpToken(parse_node::ParseNodeOpToken {
                        mode: self.mode,
                        loc: nucleus.loc.clone(),
                        text: text.clone(),
                    }),
                },
            }
        } else if let Some(first_char) = text.chars().next()
            && first_char >= '\u{80}'
        {
            // Non-ASCII fallback: report and make text-mode textord
            if !supported_codepoint(first_char as u32) {
                self.settings.report_nonstrict(
                    "unknownSymbol",
                    &format!(
                        "Unrecognized Unicode character \"{text}\" (0x{:X})",
                        first_char as u32
                    ),
                    nucleus
                        .loc
                        .as_ref()
                        .map(|loc| loc as &dyn ErrorLocationProvider),
                )?;
            } else if self.mode == Mode::Math {
                self.settings.report_nonstrict(
                    "unicodeTextInMathMode",
                    &format!("Unicode text character \"{text}\" used in math mode"),
                    nucleus
                        .loc
                        .as_ref()
                        .map(|loc| loc as &dyn ErrorLocationProvider),
                )?;
            }
            ParseNode::TextOrd(parse_node::ParseNodeTextOrd {
                mode: Mode::Text,
                loc: nucleus.loc.clone(),
                text: text.clone(),
            })
        } else {
            // EOF, ^, _, {, }, etc.
            return Ok(None);
        };

        // Consume the token now that we've formed the base node
        self.consume();

        // Transform combining characters into accents
        if let Some(accent_chars) = matched {
            for ch in accent_chars.chars() {
                if let Some(accent_mapping) = get_accent_mapping(ch) {
                    let command = if self.mode == Mode::Math {
                        accent_mapping.math.unwrap_or(accent_mapping.text)
                    } else {
                        accent_mapping.text
                    };
                    if command.is_empty() {
                        return Err(ParseError::with_token(
                            ParseErrorKind::UnsupportedAccentInMode {
                                accent: ch.to_string(),
                                mode: self.mode,
                            },
                            &nucleus,
                        ));
                    }
                    symbol_node = ParseNode::Accent(Box::new(parse_node::ParseNodeAccent {
                        mode: self.mode,
                        loc: nucleus.loc.clone(),
                        label: command.to_owned(),
                        is_stretchy: Some(false),
                        is_shifty: Some(true),
                        base: symbol_node,
                    }));
                } else {
                    return Err(ParseError::with_token(
                        ParseErrorKind::UnknownAccent {
                            accent: ch.to_string(),
                        },
                        &nucleus,
                    ));
                }
            }
        }

        Ok(Some(symbol_node))
    }

    /// Parses a separate sequence of tokens as a separate job.
    /// Tokens should be specified in reverse order, as in a MacroDefinition.
    pub fn subparse(&mut self, tokens: Vec<Token>) -> Result<Vec<ParseNode>, ParseError> {
        // Save the next token from the current job.
        let old_token = self.next_token.take();

        // Run the new job, terminating it with an excess '}'
        self.gullet.push_token(Token::new("}", None));
        self.gullet.push_tokens(tokens);
        let parse = self.parse_expression(false, None)?;
        self.expect("}", true)?;

        // Restore the next token from the current job.
        self.next_token = old_token;

        Ok(parse)
    }

    /// Handle a subscript or superscript with nice errors.
    pub fn handle_sup_subscript(
        &mut self,
        name: &str, // For error reporting.
    ) -> Result<ParseNode, ParseError> {
        let symbol_token = self.fetch()?.clone();
        self.consume();
        self.consume_spaces()?; // ignore spaces before sup/subscript argument

        // Skip over allowed internal nodes such as \relax
        let mut group = self.parse_group(name, None)?;
        while let Some(ParseNode::Internal(_)) = group {
            group = self.parse_group(name, None)?;
        }

        group.map_or_else(
            || {
                Err(ParseError::with_token(
                    ParseErrorKind::ExpectedGroupAfterSymbol {
                        symbol: symbol_token.text.to_owned_string(),
                    },
                    &symbol_token,
                ))
            },
            Ok,
        )
    }

    /// Call a function handler with a suitable context and arguments.
    pub fn call_function(
        &mut self,
        name: &str,
        args: Vec<ParseNode>,
        opt_args: Vec<Option<ParseNode>>,
        token: Option<&Token>,
        break_on_token_text: Option<&BreakToken>,
    ) -> Result<ParseNode, ParseError> {
        // Get the function spec before creating the context to avoid borrowing issues
        let func = self.ctx.functions.get(name);

        if let Some(func) = func
            && let Some(handler) = func.handler
        {
            let context = FunctionContext {
                func_name: name,
                parser: self,
                token,
                break_on_token_text,
            };
            return handler(context, args, opt_args);
        }

        Err(ParseError::new(ParseErrorKind::NoFunctionHandler {
            name: name.to_owned(),
        }))
    }

    /// Parses the arguments of a function or environment
    pub fn parse_arguments(
        &mut self,
        func: &str, // Should look like "\name" or "\begin{name}".
        func_data: &dyn Spec,
    ) -> Result<(Vec<ParseNode>, Vec<Option<ParseNode>>), ParseError> {
        let total_args = func_data.num_args() + func_data.num_optional_args();
        if total_args == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let num_optional = func_data.num_optional_args();
        let mut args = Vec::with_capacity(func_data.num_args());
        let mut opt_args = Vec::with_capacity(num_optional);

        let arg_context = format!("argument to '{func}'");
        let is_sqrt = func_data.node_type() == Some(&NodeType::Sqrt);

        for i in 0..total_args {
            let arg_type = func_data.arg_types().and_then(|v| v.get(i));
            let is_optional = i < num_optional;

            let arg_type = if (func_data.primitive() && arg_type.is_none()) ||
                // \sqrt expands into primitive if optional argument doesn't exist
                (is_sqrt && i == 1 && opt_args.first().is_none_or(|opt: &Option<ParseNode>| opt.is_none()))
            {
                Some(ArgType::Primitive)
            } else {
                arg_type.copied()
            };

            let arg = self.parse_group_of_type(&arg_context, arg_type.as_ref(), is_optional)?;

            if is_optional {
                opt_args.push(arg);
            } else if let Some(a) = arg {
                args.push(a);
            } else {
                return Err(ParseError::new(ParseErrorKind::NullArgument));
            }
        }

        Ok((args, opt_args))
    }
}
