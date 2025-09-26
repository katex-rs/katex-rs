//! The Lexer class handles tokenizing the input in various ways. Since our
//! parser expects us to be able to backtrack, the lexer allows lexing from any
//! given starting point.
//!
//! Its main exposed function is the `lex` function, which takes a position to
//! lex from and a type of token to lex. It defers to the appropriate
//! `_innerLex` function.
//!
//! The various `_innerLex` functions perform the actual lexing of different
//! kinds.

use crate::namespace::KeyMap;
use crate::types::{
    LexerInterface, ParseError, ParseErrorKind, Settings, SourceLocation, Token, TokenText,
};
use alloc::sync::Arc;

/// Returns the byte index of the last character in the string `s`
/// that is **not** a Unicode combining diacritical mark
#[must_use]
pub fn last_non_combining_mark_index(s: &str) -> Option<usize> {
    let mut cut_idx = s.len();
    for (idx, ch) in s.char_indices().rev() {
        if (0x0300..=0x036F).contains(&(ch as u32)) {
            cut_idx = idx;
        } else {
            break;
        }
    }
    if cut_idx == s.len() {
        None
    } else {
        Some(cut_idx)
    }
}

const fn is_combining_mark(ch: char) -> bool {
    (ch as u32) >= 0x0300 && (ch as u32) <= 0x036F
}

fn match_space(s: &str) -> Option<usize> {
    let mut len = 0;
    for c in s.chars() {
        if matches!(c, ' ' | '\r' | '\n' | '\t') {
            len += c.len_utf8();
        } else {
            break;
        }
    }
    (len > 0).then_some(len)
}

fn match_control_space(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    let mut len = 0;
    if chars.next()? != '\\' {
        return None;
    }
    len += 1;
    let next = chars.next()?;
    len += next.len_utf8();
    if next == '\n' {
    } else if matches!(next, ' ' | '\r' | '\t') {
        while let Some(c) = chars.clone().next() {
            if matches!(c, ' ' | '\r' | '\t') {
                chars.next();
                len += c.len_utf8();
            } else {
                break;
            }
        }
        if let Some(c) = chars.clone().next()
            && c == '\n'
        {
            chars.next();
            len += c.len_utf8();
        }
    } else {
        return None;
    }
    while let Some(c) = chars.clone().next() {
        if matches!(c, ' ' | '\r' | '\t') {
            chars.next();
            len += c.len_utf8();
        } else {
            break;
        }
    }
    Some(len)
}

fn match_normal_char_with_accents(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    let first = chars.next()?;
    let mut len_b = first.len_utf8();
    let u = first as u32;
    let in_range = |x: u32, a: u32, b: u32| x >= a && x <= b;
    if in_range(u, 0x0021, 0x005B)
        || in_range(u, 0x005D, 0x2027)
        || in_range(u, 0x202A, 0xD7FF)
        || in_range(u, 0xF900, 0xFFFF)
    {
        while let Some(c) = chars.clone().next() {
            if is_combining_mark(c) {
                chars.next();
                len_b += c.len_utf8();
            } else {
                break;
            }
        }
        return Some(len_b);
    }
    if u > 0xFFFF {
        while let Some(c) = chars.clone().next() {
            if is_combining_mark(c) {
                chars.next();
                len_b += c.len_utf8();
            } else {
                break;
            }
        }
        return Some(len_b);
    }
    None
}

fn match_verb(s: &str, star: bool) -> Option<usize> {
    let prefix = if star { "\\verb*" } else { "\\verb" };
    let rest = s.strip_prefix(prefix)?;

    let mut chars = rest.char_indices();
    let (_, delim_char) = chars.next()?;
    if !star && delim_char.is_ascii_alphabetic() {
        return None;
    }

    for (i, c) in chars {
        if matches!(c, '\n' | '\r') {
            return None;
        }
        if c == delim_char {
            return Some(prefix.len() + i + c.len_utf8());
        }
    }
    None
}

fn match_control_word(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    let mut len = 0;
    if chars.next()? != '\\' {
        return None;
    }
    len += 1;
    let mut matched = false;
    for c in chars {
        if c.is_ascii_alphabetic() || c == '@' {
            len += c.len_utf8();
            matched = true;
        } else {
            break;
        }
    }
    matched.then_some(len)
}

fn match_control_word_with_space(s: &str) -> Option<(usize, usize)> {
    if let Some(len) = match_control_word(s) {
        let rest = &s[len..];
        if let Some(space_len) = match_space(rest) {
            return Some((len, space_len));
        }
        return Some((len, 0));
    }
    None
}

fn match_control_symbol(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    if chars.next()? != '\\' {
        return None;
    }
    let c = chars.next()?;
    let cu = c as u32;
    if (0xD800..=0xDFFF).contains(&cu) {
        return None;
    }
    Some(1 + c.len_utf8())
}

fn exec(last_index: &mut usize, slice: &str) -> TokenMatch {
    let (branch, mlen, skip) = if let Some(l) = match_space(slice) {
        (BranchKind::Space, l, 0)
    } else if let Some(l) = match_control_space(slice) {
        (BranchKind::ControlSpace, l, 0)
    } else if let Some(l) = match_normal_char_with_accents(slice) {
        (BranchKind::NormalWithAccents, l, 0)
    } else if let Some(l) = match_verb(slice, true) {
        (BranchKind::VerbStar, l, 0)
    } else if let Some(l) = match_verb(slice, false) {
        (BranchKind::Verb, l, 0)
    } else if let Some((l, s)) = match_control_word_with_space(slice) {
        (BranchKind::ControlWordWhitespace, l + s, s)
    } else if let Some(l) = match_control_symbol(slice) {
        (BranchKind::ControlSymbol, l, 0)
    } else {
        // fallback
        let Some(ch) = slice.chars().next() else {
            return TokenMatch {
                branch: BranchKind::Unknown,
                mlen: 0,
                skip: 0,
            };
        };
        (BranchKind::Unknown, ch.len_utf8(), 0)
    };

    *last_index += mlen;
    TokenMatch { branch, mlen, skip }
}

#[derive(PartialEq, Eq)]
enum BranchKind {
    Unknown,
    Space,
    ControlSpace,
    NormalWithAccents,
    VerbStar,
    Verb,
    ControlWordWhitespace,
    ControlSymbol,
}

struct TokenMatch {
    branch: BranchKind,
    mlen: usize,
    skip: usize,
}

/// The core lexer for tokenizing LaTeX mathematical expressions in KaTeX.
pub struct Lexer<'a> {
    input: Arc<str>,
    last_index: usize,
    settings: &'a Settings,
    catcodes: KeyMap<char, u8>,
}

impl<'a> Lexer<'a> {
    /// Creates a new `Lexer` instance for tokenizing the provided LaTeX input
    /// string.
    #[must_use]
    pub fn new(input: Arc<str>, settings: &'a Settings) -> Self {
        let mut catcodes = KeyMap::default();
        catcodes.insert('%', 14); // comment character
        catcodes.insert('~', 13); // active character

        Self {
            input,
            last_index: 0,
            settings,
            catcodes,
        }
    }

    /// Sets the category code for a specific character, controlling its lexical
    /// behavior.
    pub fn set_catcode(&mut self, char: char, code: u8) {
        self.catcodes.insert(char, code);
    }

    /// Retrieves the category code for a specific character, if one has been
    /// set.
    #[must_use]
    pub fn get_catcode(&self, ch: char) -> Option<u8> {
        self.catcodes.get(&ch).copied()
    }

    /// Tokenizes and returns the next token from the current position in the
    /// input string.
    pub fn lex(&mut self) -> Result<Token, ParseError> {
        // If at end of input, return EOF token
        if self.last_index >= self.input.len() {
            return Ok(Token {
                text: TokenText::Static("EOF"),
                loc: Some(SourceLocation {
                    input: Arc::clone(&self.input),
                    start: self.last_index,
                    end: self.last_index,
                }),
                noexpand: None,
                treat_as_relax: None,
            });
        }

        let slice = &self.input[self.last_index..];
        let matched = exec(&mut self.last_index, slice);

        let token_text = match matched.branch {
            BranchKind::Unknown => {
                let ch = &slice[..matched.mlen];
                let loc = Some(SourceLocation {
                    input: Arc::clone(&self.input),
                    start: self.last_index - matched.mlen,
                    end: self.last_index,
                });
                let token = Token::new(ch.to_owned(), loc);
                return Err(ParseError::with_token(
                    ParseErrorKind::UnexpectedCharacter {
                        character: ch.to_owned(),
                    },
                    &token,
                ));
            }
            BranchKind::ControlWordWhitespace => TokenText::slice(
                Arc::clone(&self.input),
                self.last_index - matched.mlen,
                self.last_index - matched.skip,
            ),
            BranchKind::ControlSymbol
            | BranchKind::NormalWithAccents
            | BranchKind::Verb
            | BranchKind::VerbStar => TokenText::slice(
                Arc::clone(&self.input),
                self.last_index - matched.mlen,
                self.last_index,
            ),
            BranchKind::ControlSpace => TokenText::Static(r"\ "),
            BranchKind::Space => TokenText::Static(" "),
        };

        if token_text.len() == 1
            && let Some(first_char) = token_text.as_str().chars().next()
            && self.catcodes.get(&first_char) == Some(&14)
        {
            // Comment character, skip to end of line
            if let Some(rel_pos) = slice.find('\n') {
                let nl_index_global = self.last_index + rel_pos;
                self.last_index = nl_index_global;
            } else {
                self.last_index = self.input.len();
                self.settings.report_nonstrict("commentAtEnd", "% comment has no terminating newline; LaTeX would fail because of commenting the end of math mode (e.g. $)", None)?;
            }
            return self.lex();
        }

        Ok(Token::new(
            token_text,
            Some(SourceLocation {
                input: Arc::clone(&self.input),
                start: self.last_index - matched.mlen,
                end: self.last_index,
            }),
        ))
    }

    /// Returns the current byte position in the input string where the lexer
    /// will next read.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.last_index
    }

    /// Manually sets the current position in the input string for lexing.
    pub const fn set_position(&mut self, last_index: usize) {
        self.last_index = last_index;
    }
}

impl LexerInterface for Lexer<'_> {
    fn input(&self) -> &str {
        &self.input
    }
}
