//! Macro definition system for KaTeX
//!
//! This module provides the infrastructure for defining how macros are expanded
//! and processed, including the MacroContextInterface that provides context to
//! macro expansion functions.

use alloc::sync::Arc;
use core::fmt::{self, Debug};

use crate::{
    KatexContext, ParseError,
    namespace::Namespace,
    types::{Mode, Token},
};

pub mod builtins;

/// Represents the result of consuming an argument from the token stream during
/// LaTeX macro expansion.
///
/// In KaTeX, macros can take arguments enclosed in braces `{}` or delimited by
/// specific tokens. This struct captures the tokens that form the argument,
/// along with the boundary tokens that define its start and end in the input
/// stream. Arguments are essential for parameterized macro expansion, allowing
/// macros to operate on variable content.
///
/// # Examples
///
/// When parsing a macro like `\frac{a}{b}`, each braced group becomes a
/// `MacroArg`:
/// - The first argument contains tokens for 'a', with start/end tokens being
///   `{` and `}`.
/// - The second argument contains tokens for 'b', similarly bounded.
///
/// This structure enables precise tracking of argument boundaries for error
/// reporting and nested macro processing.
///
/// # Fields
///
/// - `tokens`: The sequence of tokens that constitute the argument's content.
///   These tokens are extracted from the input stream and may include nested
///   structures, subscripts, or other LaTeX constructs.
/// - `start`: The token that marks the beginning of the argument. Typically an
///   opening delimiter like `{` or a specific command token that initiates
///   argument consumption.
/// - `end`: The token that marks the end of the argument. Usually a closing
///   delimiter like `}` or a matching token that terminates argument parsing.
///
/// # Error Handling
///
/// Argument consumption may fail if delimiters are mismatched or if the stream
/// ends unexpectedly. See [`MacroContextInterface::consume_arg`] for related
/// error conditions.
///
/// # Cross-references
///
/// - Used by [`MacroContextInterface::consume_arg`] to return parsed arguments.
/// - Related to token processing in [`crate::lexer`] and macro expansion in
///   [`crate::parser`].
#[derive(Debug, Clone)]
pub struct MacroArg {
    /// The sequence of tokens that constitute the argument's content.
    /// These tokens are extracted from the input stream and represent the
    /// actual content of the macro argument, which may include nested
    /// structures, mathematical expressions, or text elements.
    pub tokens: Vec<Token>,
    /// The token that marks the beginning of the argument.
    /// This is typically an opening delimiter like `{` or a specific command
    /// token that signals the start of argument parsing in LaTeX macro
    /// syntax.
    pub start: Token,
    /// The token that marks the end of the argument.
    /// This is typically a closing delimiter like `}` or a matching delimiter
    /// that completes the argument in the token stream.
    pub end: Token,
}

/// Represents the expansion of a LaTeX macro into a sequence of tokens during
/// mathematical typesetting.
///
/// In KaTeX, macros are expanded to produce new token sequences that replace
/// the original macro invocation. This struct encapsulates the result of such
/// expansion, including the generated tokens, argument specifications, and
/// expansion properties. The tokens are stored in reverse order to facilitate
/// efficient stack-based processing during parsing.
///
/// Macro expansion is a core mechanism in LaTeX for defining reusable
/// constructs, such as mathematical symbols, formatting commands, or complex
/// expressions. For example, `\alpha` expands to the Greek letter α, while
/// parameterized macros like `\frac{numerator}{denominator}` expand based on
/// provided arguments.
///
///
/// # Fields
///
/// - `tokens`: The sequence of tokens resulting from macro expansion. These
///   tokens replace the original macro call in the input stream and are stored
///   in reverse order for efficient processing by the parser.
/// - `num_args`: The number of arguments this macro expects. For example,
///   `\frac` expects 2 arguments, while `\alpha` expects 0.
/// - `delimiters`: Optional delimiters for each argument. Each inner vector
///   specifies the opening and closing delimiters for an argument. If `None`,
///   arguments are typically braced.
/// - `unexpandable`: Whether this macro should not be further expanded. Used in
///   TeX's `\let` command to create aliases that preserve the original
///   behavior.
///
/// # Error Handling
///
/// Expansion may fail if arguments don't match the expected delimiters or if
/// the macro is undefined. See [`MacroContextInterface::expand_macro`] for
/// expansion error conditions.
///
/// # Cross-references
///
/// - Created during macro expansion in [`crate::parser`].
/// - Used by [`MacroContextInterface`] methods for token processing.
/// - Related to macro definitions in [`MacroDefinition`].
#[derive(Debug, Clone, Default)]
pub struct MacroExpansion {
    /// The sequence of tokens resulting from macro expansion.
    /// These tokens are stored in reverse order to optimize stack-based parsing
    /// and represent the replacement content for the original macro invocation.
    pub tokens: Vec<Token>,
    /// The number of arguments this macro expects when invoked.
    /// This determines how many [`MacroArg`] instances will be consumed during
    /// expansion.
    pub num_args: usize,
    /// Optional delimiters specifying how arguments should be parsed.
    /// Each element is a pair of strings representing opening and closing
    /// delimiters. If `None`, standard braced arguments `{}` are assumed.
    pub delimiters: Option<Vec<Vec<String>>>,
    /// Indicates whether this macro should resist further expansion.
    /// When `Some(true)`, prevents recursive expansion, useful for creating
    /// stable aliases.
    pub unexpandable: Option<bool>,
}

/// Type for function-based macro definitions in KaTeX.
pub type MacroFunction = Arc<
    dyn Fn(&mut dyn MacroContextInterface) -> Result<MacroExpansionResult, ParseError>
        + Send
        + Sync,
>;

/// Type for static function-based macro definitions in KaTeX.
pub type StaticMacroFunction =
    fn(&mut dyn MacroContextInterface) -> Result<MacroExpansionResult, ParseError>;

/// Defines how a LaTeX macro is expanded and processed in KaTeX's mathematical
/// typesetting system.
///
/// Macro definitions specify the transformation rules for macro invocations,
/// supporting different expansion strategies to handle various LaTeX
/// constructs. This enum encapsulates the three primary ways macros can be
/// defined: simple string replacement, token-based expansion, and dynamic
/// function-based generation.
///
/// In LaTeX/KaTeX, macros enable the creation of reusable mathematical notation
/// and formatting commands. For instance, `\alpha` might be defined as a simple
/// string or token expansion, while more complex macros like `\frac` use
/// structured token expansions with arguments.
///
/// # Variants
///
/// - `String`: Direct string replacement for simple macros. Used for basic
///   symbol substitutions where no complex token processing is needed.
/// - `Expansion`: Structured token expansion with argument handling. Supports
///   parameterized macros with specific token sequences and delimiter rules.
/// - `Function`: Dynamic expansion using a closure that can access parsing
///   context. Enables macros that adapt their behavior based on the current
///   parsing state.
///
/// # Error Handling
///
/// Function-based macros can return errors during expansion, typically due to
/// invalid arguments or context-dependent failures. String and expansion
/// variants are generally error-free unless token processing fails.
///
/// # Cross-references
///
/// - Stored in [`Namespace`] for lookup during parsing.
/// - Processed by [`MacroContextInterface`] methods during expansion.
/// - Related to expansion results in [`MacroExpansionResult`].
#[derive(Clone)]
pub enum MacroDefinition {
    /// Direct string replacement for simple macro expansion.
    /// The string is inserted literally into the output, suitable for basic
    /// symbol macros like `\alpha` that don't require complex token
    /// manipulation or arguments.
    String(String),
    /// Direct string replacement but without owning the string.
    StaticStr(&'static str),
    /// Structured expansion with tokens and argument specifications.
    /// Provides full control over the expansion process, including argument
    /// consumption and delimiter handling, ideal for complex mathematical
    /// constructs.
    Expansion(MacroExpansion),
    /// Dynamic expansion using a function that can access the parsing context.
    /// Allows macros to make decisions based on the current state, such as mode
    /// (math/text) or available macros, enabling sophisticated conditional
    /// expansions.
    Function(MacroFunction),
    /// Dynamic expansion using a static function pointer for better
    /// performance. Avoids the Arc overhead while providing the same
    /// functionality as Function.
    StaticFunction(StaticMacroFunction),
}

impl MacroDefinition {
    /// Try to directly get the String of current macro
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            Self::StaticStr(s) => Some(s),
            _ => None,
        }
    }
}

impl Debug for MacroDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticStr(s) => f.debug_tuple("StaticStr").field(s).finish(),
            Self::String(s) => f.debug_tuple("String").field(s).finish(),
            Self::Expansion(e) => f.debug_tuple("Expansion").field(e).finish(),
            Self::Function(_) => f.debug_struct("Function").finish(),
            Self::StaticFunction(_) => f.debug_struct("StaticFunction").finish(),
        }
    }
}

/// Represents the outcome of expanding a function-based macro in KaTeX's
/// mathematical typesetting system.
///
/// When macros are defined using functions (via [`MacroDefinition::Function`]),
/// they return this enum to specify the result of their expansion. This allows
/// dynamic macros to produce either simple string output or complex token-based
/// expansions depending on context and arguments.
///
/// Function-based macros provide the most flexibility in LaTeX processing,
/// enabling conditional behavior, argument validation, and context-aware
/// generation of mathematical content. For example, a macro might return
/// different symbols based on the current math mode or generate different
/// structures based on argument types.
///
/// # Variants
///
/// - `String`: A simple string that will be inserted directly into the output.
///   Suitable for basic text or symbol substitutions that don't require further
///   token processing.
/// - `Expansion`: A full token expansion with all associated metadata. Enables
///   complex macros that need argument handling, delimiters, or special
///   expansion properties.
///
/// # Error Handling
///
/// Function-based macros can fail during expansion, returning a `String` error
/// message. The expansion process may encounter issues like invalid arguments,
/// undefined references, or context-dependent failures that prevent successful
/// macro resolution.
///
/// # Cross-references
///
/// - Returned by function-based macros defined in
///   [`MacroDefinition::Function`].
/// - Processed by [`MacroContextInterface`] during macro expansion.
/// - Contains either strings or [`MacroExpansion`] instances.
#[derive(Debug, Clone)]
pub enum MacroExpansionResult {
    /// A simple string result from macro expansion.
    /// The string is inserted directly into the output stream without further
    /// processing, ideal for basic symbol replacements or simple text
    /// generation.
    String(String),
    /// A structured expansion result with tokens and metadata.
    /// Provides full control over the expansion process, including argument
    /// specifications and expansion properties, suitable for complex
    /// mathematical constructs.
    Expansion(MacroExpansion),
    /// Represents an empty expansion result.
    /// Used when a macro intentionally produces no output, effectively removing
    /// itself from the input stream.
    Empty,
}

/// Interface providing context and utilities for LaTeX macro expansion in
/// KaTeX's mathematical typesetting system.
///
/// This trait defines the contract for objects that provide the necessary
/// context and operations for expanding macros during LaTeX parsing. It is
/// implemented by the macro expander and offers access to the current parsing
/// state, token stream manipulation, and macro definitions.
///
/// The interface supports the full range of TeX-like macro expansion
/// operations, including token peeking, argument consumption, recursive
/// expansion, and mode-aware processing. Function-based macros use this
/// interface to access parsing context and perform complex expansion logic.
///
/// # Cross-references
///
/// - Implemented by the macro expansion engine in [`crate::parser`].
/// - Used by function-based macros defined in [`MacroDefinition::Function`].
/// - Provides access to [`Namespace`] of macro definitions.
pub trait MacroContextInterface<'a> {
    /// Returns the current parsing mode (math or text).
    ///
    /// The mode affects how tokens are interpreted and expanded. In math mode,
    /// certain macros may behave differently or have different symbol mappings
    /// compared to text mode.
    ///
    /// # Returns
    ///
    /// The current [`Mode`] of the parser.
    fn mode(&self) -> Mode;

    /// Provide access to the current KaTeX context.
    ///
    /// This method allows macros to access the broader KaTeX context,
    ///
    /// # Returns
    ///
    /// A reference to the current [`KatexContext`].
    fn context(&self) -> &KatexContext;

    /// Provides read-only access to the macro namespace.
    ///
    /// The namespace contains all currently defined macros, functions, and
    /// symbols that can be referenced during expansion. This allows macros
    /// to check for the existence of other macros or access their
    /// definitions.
    ///
    /// # Returns
    ///
    /// A reference to the [`Namespace`] containing macro definitions.
    fn macros<'s>(&'s self) -> &'s Namespace<'a, MacroDefinition>;

    /// Provides mutable access to the macro namespace.
    ///
    /// This method allows macros to modify the macro namespace during
    /// expansion, such as defining new macros or updating existing ones.
    /// This is essential for macros that create other macros or modify the
    /// parsing environment.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`Namespace`] containing macro definitions.
    fn macros_mut<'s>(&'s mut self) -> &'s mut Namespace<'a, MacroDefinition>;

    /// Peeks at the next token without consuming it.
    ///
    /// This method allows macros to look ahead in the token stream to make
    /// decisions based on upcoming tokens, similar to TeX's `\futurelet`
    /// command. The token is not removed from the stream and remains
    /// available for subsequent operations.
    ///
    /// # Returns
    ///
    /// The next [`Token`] in the stream, or an appropriate token if the stream
    /// is empty.
    fn future_mut(&mut self) -> Result<Token, ParseError>;

    /// Consumes and returns the next token from the stream.
    ///
    /// This method removes the next token from the input stream and returns it,
    /// advancing the parsing position. This is the primary way to consume input
    /// during macro expansion.
    ///
    /// # Returns
    ///
    /// The next [`Token`] from the stream.
    fn pop_token(&mut self) -> Result<Token, ParseError>;

    /// Consumes all consecutive whitespace tokens without expansion.
    ///
    /// This method skips over whitespace in the token stream, which is useful
    /// for ignoring spacing that doesn't affect the semantic meaning of the
    /// input. Unlike other consumption methods, this does not perform macro
    /// expansion.
    fn consume_spaces(&mut self) -> Result<(), ParseError>;

    /// Performs a single expansion step on the next token if possible.
    ///
    /// This method attempts to expand the next token exactly once, replacing
    /// it with its expansion if it's a macro. If the token is not expandable,
    /// it remains unchanged. The `expandable_only` parameter controls whether
    /// only explicitly expandable tokens are considered.
    ///
    /// # Parameters
    ///
    /// - `expandable_only`: If `Some(true)`, only expand tokens that are
    ///   explicitly marked as expandable. If `None` or `Some(false)`, attempt
    ///   expansion on any suitable token.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(offset))`: The token was expanded, and `offset` indicates the
    ///   change in token stream length.
    /// - `Ok(None)`: The token was not expandable.
    /// - `Err(ParseError)`: An error occurred during expansion.
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` if expansion fails due to invalid macro
    /// definitions, circular references, or malformed input.
    fn expand_once(&mut self, expandable_only: Option<bool>) -> Result<Option<isize>, ParseError>;

    /// Expands the next token and returns the resulting top token without
    /// consuming it.
    ///
    /// This method combines expansion with lookahead, similar to TeX's
    /// `\expandafter\futurelet`. It expands the next token and then returns
    /// the first token of the expansion result, without removing it from
    /// the stream.
    ///
    /// # Returns
    ///
    /// The first [`Token`] of the expansion result.
    ///
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` if expansion fails.
    fn expand_after_future(&mut self) -> Result<Token, ParseError>;

    /// Recursively expands the next token until a non-expandable token is
    /// found.
    ///
    /// This method performs full expansion of the next token, following all
    /// macro expansions until a primitive (non-expandable) token is
    /// reached. This is the standard expansion behavior for most LaTeX
    /// processing.
    ///
    /// # Returns
    ///
    /// The first non-expandable [`Token`] after full expansion.
    ///
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` if expansion fails due to circular references,
    /// undefined macros, or other expansion errors.
    fn expand_next_token(&mut self) -> Result<Token, ParseError>;

    /// Fully expands a named macro and returns its token sequence.
    ///
    /// This method looks up a macro by name and performs complete expansion,
    /// returning the resulting sequence of tokens. If the macro is not defined,
    /// returns `None`.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the macro to expand (without the leading
    ///   backslash).
    ///
    /// # Returns
    ///
    /// - `Ok(Some(tokens))`: The macro was found and expanded to the given
    ///   tokens.
    /// - `Ok(None)`: No macro with the given name is defined.
    /// - `Err(ParseError)`: Expansion failed.
    ///
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` for expansion failures such as circular
    /// references or invalid macro definitions.
    fn expand_macro(&mut self, name: &str) -> Result<Option<Vec<Token>>, ParseError>;

    /// Fully expands a named macro and returns its string representation.
    ///
    /// This method is similar to expand_macro but returns the result as a
    /// string rather than tokens. This is useful for macros that produce
    /// text output.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the macro to expand (without the leading
    ///   backslash).
    ///
    /// # Returns
    ///
    /// - `Ok(Some(string))`: The macro was found and expanded to the given
    ///   string.
    /// - `Ok(None)`: No macro with the given name is defined.
    /// - `Err(ParseError)`: Expansion failed.
    ///
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` for expansion failures.
    fn expand_macro_as_text(&mut self, name: &str) -> Result<Option<String>, ParseError>;

    /// Fully expands a given token stream.
    ///
    /// This method takes a sequence of tokens and performs complete expansion
    /// on them, returning the fully expanded result. Note that input tokens
    /// are expected in reverse order (for efficiency), but output tokens
    /// are in forward order.
    ///
    /// # Parameters
    ///
    /// - `tokens`: The token stream to expand, in reverse order.
    ///
    /// # Returns
    ///
    /// The fully expanded token sequence in forward order.
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` if any expansion in the stream fails.
    fn expand_tokens(&mut self, tokens: Vec<Token>) -> Result<Vec<Token>, ParseError>;

    /// Consumes a single argument from the token stream.
    ///
    /// This method parses an argument from the current position in the token
    /// stream, handling braced groups or custom delimiters. Arguments are
    /// fundamental to parameterized macros in LaTeX.
    ///
    /// # Parameters
    ///
    /// - `delims`: Optional custom delimiters for the argument. If `None`,
    ///   standard braced arguments `{}` are expected.
    ///
    /// # Returns
    ///
    /// A [`MacroArg`] containing the parsed argument tokens and boundary
    /// information.
    ///
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` if argument parsing fails due to mismatched
    /// delimiters, unexpected end of stream, or invalid syntax.
    fn consume_arg(&mut self, delims: Option<&Vec<String>>) -> Result<MacroArg, ParseError>;

    /// Consumes multiple arguments from the token stream.
    ///
    /// This method repeatedly calls consume_arg to parse the specified
    /// number of arguments, returning them as a vector. This is convenient
    /// for macros that take a fixed number of arguments.
    ///
    /// # Parameters
    ///
    /// - `num_args`: The number of arguments to consume.
    ///
    /// # Returns
    ///
    /// A vector of token vectors, each representing one argument.
    ///
    ///
    /// # Error Handling
    ///
    /// Returns `ParseError` if any argument parsing fails.
    fn consume_args(&mut self, num_args: usize) -> Result<Vec<Vec<Token>>, ParseError>;

    /// Checks if a command name is currently defined.
    ///
    /// This method determines whether a given command name has some
    /// functionality in the current parsing context. A command is
    /// considered defined if it's a macro, function, symbol, or one of the
    /// special implicit commands.
    ///
    /// # Parameters
    ///
    /// - `name`: The command name to check (without the leading backslash).
    ///
    /// # Returns
    ///
    /// `true` if the command is defined, `false` otherwise.
    fn is_defined(&self, name: &str) -> bool;

    /// Checks if a command name is expandable.
    ///
    /// This method determines whether a given command can be expanded as a
    /// macro. Only macros (not functions or symbols) are typically
    /// expandable.
    ///
    /// # Parameters
    ///
    /// - `name`: The command name to check (without the leading backslash).
    ///
    /// # Returns
    ///
    /// `true` if the command is expandable, `false` otherwise.
    fn is_expandable(&self, name: &str) -> bool;

    /// Starts a new group nesting within the macro namespace.
    ///
    /// This method creates a new scope for macro definitions, allowing
    /// temporary modifications to the macro namespace that can be undone
    /// when the group ends. This is essential for macros that need to
    /// temporarily redefine other macros.
    fn begin_group(&mut self);

    /// Ends the current group nesting within the macro namespace.
    ///
    /// This method restores the macro namespace to its state before the current
    /// group began, undoing any local macro definitions or redefinitions made
    /// within that group.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: The group was successfully ended.
    /// - `Err(ParseError)`: No group was active to end.
    fn end_group(&mut self) -> Result<(), ParseError>;
}
