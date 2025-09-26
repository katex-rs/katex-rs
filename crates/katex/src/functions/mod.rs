//! Function implementations for KaTeX
//!
//! This module provides implementations of various LaTeX functions supported by
//! KaTeX, focusing on mathematical typesetting and rendering. It includes
//! mechanisms for defining and registering functions that handle LaTeX
//! commands, such as generalized fractions (genfrac), relaxation commands, and
//! various mathematical operators.
//!
//! ## Key Components
//!
//! - **Generalized Fractions (genfrac)**: Handles infix primitives like
//!   `\over`, `\choose`, `\atop`, etc., which are used to create fractions and
//!   similar constructs in mathematical expressions. These are replaced with
//!   appropriate fraction variants during parsing.
//!
//! - **Relaxation Mechanisms**: Implements commands like `\relax`, which serve
//!   as no-op operations to control parsing flow and prevent unwanted
//!   expansions in TeX-like contexts.
//!
//! - **Poor Man's Bold (pmb)**: Implements `\pmb` command that simulates bold
//!   text using CSS text-shadow when proper bold fonts are not available.
//!
//! - **Font Commands**: Provides font family and style commands like `\mathrm`,
//!   `\mathbf`, `\mathit`, etc., for controlling text appearance in
//!   mathematical expressions.
//!
//! - **Operator Commands**: Implements mathematical operators like `\int`,
//!   `\sum`, `\lim`, with proper spacing and limit positioning.
//!
//! - **Styling Commands**: Handles text styling commands like `\textstyle`,
//!   `\scriptstyle`, `\scriptscriptstyle` for controlling mathematical
//!   expression sizing.
//!
//! The module organizes function definitions into submodules for better
//! maintainability and follows KaTeX conventions for LaTeX command handling.

// Export individual function modules
// Export individual function modules
mod accent;
mod accentunder;
mod arrow;
mod char;
mod color;
mod cr;
mod def;
mod delimsizing;
mod enclose;
mod environment;
mod font;
mod genfrac;
mod hbox;
mod horiz_brace;
mod href;
mod html;
mod htmlmathml;
mod includegraphics;
mod kern;
mod lap;
mod math;
mod mathchoice;
mod mclass;
mod op;
mod operatorname;
mod ordgroup;
mod overline;
mod phantom;
mod pmb;
mod raisebox;
mod relax;
mod rule;
mod sizing;
mod smash;
mod sqrt;
mod styling;
mod supsub;
mod symbols_op;
mod symbols_ord;
mod symbols_spacing;
mod tag;
mod text;
mod underline;
pub mod utils;
mod vcenter;
mod verb;

// Re-export function registration functions
/// Registers infix generalized fraction commands in the KaTeX context.
/// !TODO: document
pub use environment::define_environment;

/// This function defines LaTeX infix primitives such as `\over`, `\choose`,
/// `\atop`, `\brace`, and `\brack`, which are essential for creating fractions
/// and related mathematical constructs. These commands are not rendered
/// directly but are immediately replaced with their corresponding fraction
/// variants (e.g., `\frac`, `\binom`) during the parsing process.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # Error Handling
///
/// Errors may occur during parsing if an unrecognized infix command is
/// encountered, resulting in a `ParseError`.
///
/// # See Also
///
/// - [`define_relax`] for implementing relaxation commands.
/// - The genfrac module for detailed implementation.
pub use genfrac::define_genfrac;

/// Registers the `\relax` command in the KaTeX context.
///
/// The `\relax` command is a LaTeX no-op primitive that produces no visible
/// output but can be used to stop macro expansion or control parsing behavior
/// in certain contexts. It is particularly useful in TeX-like environments
/// where expansion needs to be halted without affecting the document structure.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # Error Handling
///
/// As a no-op, this function typically does not produce errors, but parsing
/// errors may occur in the broader context.
///
/// # See Also
///
/// - [`define_genfrac`] for fraction-related commands.
/// - The relax module for detailed implementation.
pub use relax::define_relax;

/// Registers the `\sqrt` function in the KaTeX context.
///
/// The `\sqrt` command creates square root and nth root expressions with proper
/// sizing and positioning of the radical symbol and optional root indices.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \sqrt{x}      % Square root
/// \sqrt[3]{x}   % Cube root
/// \sqrt[n]{x}   % nth root
/// ```
///
/// # Arguments
///
/// - Required: The expression under the radical
/// - Optional: The root index (e.g., 3 for cube root, n for nth root)
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_genfrac`] for fraction-related commands.
/// - The sqrt module for detailed implementation.
pub use sqrt::define_sqrt;

/// Registers the `\@char` function in the KaTeX context.
///
/// The `\@char` function is an internal LaTeX command that converts a decimal
/// number enclosed in braces (e.g., {123}) into the corresponding Unicode
/// character. It is used by the `\char` macro to create symbols from code
/// points.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # Error Handling
///
/// Errors may occur during parsing if the argument is not a valid ordgroup,
/// contains non-numeric content, or represents an invalid Unicode code point.
///
/// # See Also
///
/// - [`define_relax`] for other internal commands.
pub use char::define_char;

/// Registers the `\\` (line break) function in the KaTeX context.
///
/// The `\\` command handles line breaks and spacing in mathematical
/// expressions. It can optionally take a size parameter in square brackets to
/// specify the amount of vertical spacing to add.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # Error Handling
///
/// Errors may occur during parsing if the size parameter is malformed.
///
/// # See Also
///
/// - [`define_char`] for other internal commands.
pub use cr::define_cr;

/// Font styling functions are documented above.
///
/// Registers math mode switching functions (\(, \), $, \]) in the KaTeX
/// context.
///
/// These functions handle switching between text and math modes using
/// delimiters:
/// - `\(` and `$` switch from text mode to math mode and parse expressions
///   until `\)` or `$`
/// - `\)` and `\]` throw errors for mismatched delimiters
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # Error Handling
///
/// Errors may occur during parsing if delimiters are mismatched or if
/// expression parsing fails.
///
/// # See Also
///
/// - [`define_char`] for other internal commands.
pub use math::define_math;

/// Registers macro definition functions (\def, \gdef, \edef, \xdef, \let,
/// \futurelet, \global, \long) in the KaTeX context.
///
/// These functions provide the core macro definition functionality for KaTeX,
/// allowing users to define custom macros with parameters and replacement text.
/// The functions include:
/// - `\def`, `\gdef`: Define macros with optional global scope
/// - `\edef`, `\xdef`: Define macros with expansion of replacement text
/// - `\let`: Assign tokens to control sequences
/// - `\futurelet`: Assign tokens with lookahead
/// - `\global`, `\long`: Prefix commands for scope and parameter handling
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # Implementation Notes
///
/// Due to current architecture limitations, these functions are registered but
/// their full implementation requires mutable access to the parser's token
/// stream, which is not available through the current FunctionContext API. The
/// functions return internal nodes but do not perform the actual macro
/// definition operations.
///
/// # See Also
///
/// - [`define_char`] for other internal commands.
pub use def::define_def;

/// Registers the `\rule` function in the KaTeX context.
///
/// The `\rule` command creates horizontal or vertical rules (lines) with
/// specified width and height, commonly used for creating custom spacing or
/// decorative elements in mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \rule{2em}{0.1em}        % Basic rule with width and height
/// \rule[0.5em]{2em}{0.1em} % Rule with vertical shift
/// ```
///
/// # Arguments
///
/// - `width`: The width of the rule (required)
/// - `height`: The height/thickness of the rule (required)
/// - `shift`: Optional vertical shift of the rule from baseline (optional)
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required width or height arguments are missing
/// - Size specifications have invalid formats or units
///
/// # See Also
///
/// - [`define_kern`] for spacing-related commands.
pub use rule::define_rule;

/// Registers the `\verb` function in the KaTeX context.
///
/// The `\verb` command creates verbatim text that preserves exact formatting
/// and spacing. It takes text between matching delimiters and renders it in a
/// monospace font without interpreting LaTeX commands. The `\verb*` variant
/// replaces spaces with visible symbols.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \verb|verbatim text|     % Regular verb
/// \verb*|verbatim text|    % Verb with visible spaces
/// ```
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - The delimiters don't match
/// - The verb command is terminated by end of line without matching delimiter
///
/// # See Also
///
/// - [`define_rule`] for other text formatting commands.
pub use verb::define_verb;

/// Registers the `\tag` function in the KaTeX context.
///
/// The `\tag` command creates tagged equations with custom labels, typically
/// used for equation numbering or referencing. It renders the main expression
/// alongside a tag in a table layout for proper alignment.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \tag{1} E = mc^2        % Equation with number 1
/// \tag{*} F = ma          % Equation with custom label
/// ```
///
/// # Arguments
///
/// - Required: The main mathematical expression
/// - Required: The tag/label expression
///
/// # MathML Output
///
/// The function generates a MathML table structure:
/// ```xml
/// <mtable width="100%">
///   <mtr>
///     <mtd width="50%"></mtd>     <!-- Left padding -->
///     <mtd>expression</mtd>       <!-- Main expression -->
///     <mtd width="50%"></mtd>     <!-- Right padding -->
///     <mtd>tag</mtd>              <!-- Tag/label -->
///   </mtr>
/// </mtable>
/// ```
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_verb`] for other text formatting commands.
pub use tag::define_tag;

/// Registers the `\overline` function in the KaTeX context.
///
/// The `\overline` command draws a horizontal line above mathematical
/// expressions for emphasis or special notation, commonly used for repeating
/// decimals or special mathematical notation.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \overline{ABC}    % Overline above expression
/// ```
///
/// # Arguments
///
/// - Required: The expression to be overlined
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent-related commands.
pub use overline::define_overline;

/// Registers superscript and subscript functions in the KaTeX context.
///
/// This function defines LaTeX superscript and subscript commands that create
/// mathematical expressions with raised or lowered elements. Superscripts and
/// subscripts follow TeXbook rules 18(a-f) for precise positioning and spacing,
/// ensuring proper alignment with the base expression.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// x^2          % Superscript
/// x_2          % Subscript
/// x^{a+b}_c    % Both superscript and subscript
/// \sum_{i=1}^n % Sum with limits
/// ```
///
/// # Supported Commands
///
/// - `^`: Superscript operator
/// - `_`: Subscript operator
/// - Combined superscript and subscript expressions
/// - Automatic sizing and positioning based on context
///
/// # Implementation Details
///
/// The implementation handles complex positioning rules including:
/// - Rule 18a: Basic shift calculations
/// - Rule 18b: Subscript positioning
/// - Rule 18c-d: Superscript positioning
/// - Rule 18e: Collision avoidance between superscript and subscript
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required base expression is missing
/// - Invalid argument types are provided
/// - Superscript/subscript syntax is malformed
///
/// # See Also
///
/// - [`define_op`] for operator-related functions with limits.
/// - [`define_accent`] for accent-related positioning.
pub use supsub::define_supsub;

/// Registers atom symbol functions in the KaTeX context.
///
/// These functions handle atomic symbols that have specific mathematical
/// meaning and spacing rules, such as binary operators, relations, delimiters,
/// and punctuation. Atoms are fundamental building blocks of mathematical
/// expressions with defined interaction rules for spacing and rendering.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # Atom Types
///
/// - `bin`: Binary operators (e.g., +, -, ×, ÷)
/// - `rel`: Relations (e.g., =, <, >, ≠)
/// - `open`: Opening delimiters (e.g., (, [, {)
/// - `close`: Closing delimiters (e.g., ), ], })
/// - `punct`: Punctuation (e.g., ,, ;, .)
/// - `inner`: Inner operators (e.g., integrals, sums)
///
/// # See Also
///
/// - [`define_symbols_ord`] for ordinary symbols.
pub use symbols_op::define_symbols_op;

/// Registers mathord and textord symbol functions in the KaTeX context.
///
/// These functions handle ordinary mathematical symbols and text symbols that
/// don't have special spacing rules. They are fundamental building blocks for
/// mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # MathOrd vs TextOrd
///
/// - `mathord`: Ordinary mathematical symbols (variables, operators without
///   special spacing)
/// - `textord`: Ordinary text symbols in math mode (letters, punctuation)
///
/// # See Also
///
/// - [`define_accent`] for accent-related functions.
pub use symbols_ord::define_symbols_ord;

/// Registers accent functions (\hat, \bar, \tilde, etc.) in the KaTeX context.
///
/// This function defines LaTeX accent commands that place diacritical marks
/// above or below mathematical expressions. Accents can be stretchy (adapting
/// to the width of their base) or non-stretchy, and may be shifted for better
/// positioning.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \hat{x}      % Hat accent
/// \bar{y}      % Bar accent
/// \tilde{z}    % Tilde accent
/// \widehat{abc} % Wide hat accent
/// ```
///
/// # Math vs Text Mode
///
/// Some accents work differently in math mode vs text mode:
/// - Math accents: `\acute`, `\grave`, `\ddot`, `\tilde`, `\bar`, `\breve`,
///   `\check`, `\hat`, `\vec`, `\dot`, `\mathring`, `\widecheck`, `\widehat`,
///   `\widetilde`, `\overrightarrow`, `\overleftarrow`, `\Overrightarrow`,
///   `\overleftrightarrow`, `\overgroup`, `\overlinesegment`,
///   `\overleftharpoon`, `\overrightharpoon`
/// - Text accents: `\'`, `\``, `\^`, `\~`, `\=`, `\u`, `\.`, `\"`, `\c`, `\r`,
///   `\H`, `\v`, `\textcircled`
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_genfrac`] for fraction-related commands.
pub use accent::define_accent;

/// Registers math class functions (\mathord, \mathbin, \mathrel, etc.) in the
/// KaTeX context.
///
/// This function defines LaTeX math class commands that control the spacing and
/// rendering behavior of mathematical elements according to their semantic
/// classification. Math classes determine how symbols interact with surrounding
/// elements in terms of spacing and positioning.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \mathord{x}      % Ordinary symbol
/// \mathbin{+}      % Binary operator
/// \mathrel{=}      % Relation
/// \mathopen{(}     % Opening delimiter
/// \mathclose{)}    % Closing delimiter
/// \mathpunct{,}    % Punctuation
/// \mathinner{\int} % Inner operator
/// \@binrel{a}{b}   % Binrel with class from first argument
/// \stackrel{a}{b}  % Stack symbols
/// ```
///
/// # Math Classes
///
/// - `mord`: Ordinary symbols (variables, operators without special spacing)
/// - `mbin`: Binary operators (+, -, ×, ÷)
/// - `mrel`: Relations (=, <, >, ≠)
/// - `mopen`: Opening delimiters ((, [, {)
/// - `mclose`: Closing delimiters (), ], })
/// - `mpunct`: Punctuation (,, ;, .)
/// - `minner`: Inner operators (integrals, sums)
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent-related commands.
pub use mclass::define_mclass;

/// Registers extensible arrow functions (\xleftarrow, \xrightarrow, etc.) in
/// the KaTeX context.
///
/// This function defines LaTeX extensible arrow commands that can stretch to
/// fit their labels and support optional labels above and below the arrow.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \xleftarrow{f}        % Left arrow with label above
/// \xrightarrow[g]{f}    % Right arrow with labels above and below
/// \xleftrightarrow{h}   % Bidirectional arrow
/// \xlongequal{=}        % Long equal sign
/// ```
///
/// # Supported Commands
///
/// The following extensible arrow commands are supported:
/// - `\xleftarrow`, `\xrightarrow`, `\xLeftarrow`, `\xRightarrow`
/// - `\xleftrightarrow`, `\xLeftrightarrow`
/// - `\xhookleftarrow`, `\xhookrightarrow`, `\xmapsto`
/// - `\xrightharpoondown`, `\xrightharpoonup`, `\xleftharpoondown`,
///   `\xleftharpoonup`
/// - `\xrightleftharpoons`, `\xleftrightharpoons`, `\xlongequal`
/// - `\xtwoheadrightarrow`, `\xtwoheadleftarrow`, `\xtofrom`
/// - `\xrightleftarrows`, `\xrightequilibrium`, `\xleftequilibrium`
/// - `\cdrightarrow`, `\cdleftarrow`, `\cdlongequal` (CD environment arrows)
///
/// # Arguments
///
/// - Required: The main label/content above the arrow
/// - Optional: Additional label/content below the arrow
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent-related commands.
pub use arrow::define_arrow;

/// Registers accent under functions (\underleftarrow, \underrightarrow, etc.)
/// in the KaTeX context.
///
/// This function defines LaTeX accent under commands that place diacritical
/// marks below mathematical expressions. These are typically used for
/// under-arrows and under-tildes.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \underleftarrow{x}      % Under left arrow
/// \underrightarrow{y}     % Under right arrow
/// \utilde{z}              % Under tilde
/// ```
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for over-accent commands.
pub use accentunder::define_accentunder;

/// Registers the `\includegraphics` function in the KaTeX context.
///
/// The `\includegraphics` command includes external images or graphics in
/// mathematical expressions with specified dimensions and styling options.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \includegraphics[width=5em]{diagram.png}
/// \includegraphics[height=3em,alt=Figure]{plot.jpg}
/// \includegraphics[totalheight=4em,width=6em]{image.svg}
/// ```
///
/// # Arguments
///
/// - Required: The image source path/URL
/// - Optional: Comma-separated key-value pairs for dimensions and metadata:
///   - `width`: Image width (with unit)
///   - `height`: Image height (with unit)
///   - `totalheight`: Total height including depth (with unit)
///   - `alt`: Alternative text for accessibility
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required source argument is missing or invalid
/// - Size specifications have invalid formats or unsupported units
/// - Invalid keys are used in the optional argument
/// - The command is not trusted by security settings
///
/// # Security Considerations
///
/// This command is subject to trust validation to prevent inclusion of
/// potentially malicious images. The trust settings determine whether external
/// URLs are permitted.
///
/// # See Also
///
/// - [`define_href`] for other URL-related commands.
pub use includegraphics::define_includegraphics;

/// Registers the `\vcenter` function in the KaTeX context.
///
/// The `\vcenter` command vertically centers mathematical expressions
/// on the math axis, useful for aligning elements in complex layouts.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \vcenter{expression}    % Vertically center expression
/// ```
///
/// # Arguments
///
/// - Required: The mathematical expression to be vertically centered
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent-related commands.
pub use vcenter::define_vcenter;

/// Registers the `\raisebox` function in the KaTeX context.
///
/// The `\raisebox` command vertically displaces mathematical content
/// by a specified amount, useful for fine-tuning layout and alignment.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \raisebox{1em}{content}    % Raise content by 1em
/// \raisebox{-0.5em}{text}    % Lower content by 0.5em
/// ```
///
/// # Arguments
///
/// - Required: The vertical displacement amount (size)
/// - Required: The mathematical expression to be displaced
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - First argument is not a valid size specification
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_vcenter`] for vertical centering.
pub use raisebox::define_raisebox;

/// Registers lap functions (\mathllap, \mathrlap, \mathclap) in the KaTeX
/// context.
///
/// This function defines LaTeX horizontal overlap commands that allow text
/// to overlap other content without taking up horizontal space.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \mathllap{left}     % Left overlap
/// \mathrlap{right}    % Right overlap
/// \mathclap{center}   % Center overlap
/// ```
///
/// # Arguments
///
/// - Required: The content to be overlapped
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_phantom`] for invisible content.
pub use lap::define_lap;

/// Registers the `\html@mathml` function in the KaTeX context.
///
/// The `\html@mathml` command allows different content to be rendered in HTML
/// and MathML formats. It takes two arguments: the first is used for HTML
/// output, and the second is used for MathML output.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \html@mathml{html_content}{mathml_content}
/// ```
///
/// # Arguments
///
/// - Required: HTML content (first argument)
/// - Required: MathML content (second argument)
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_html`] for HTML-specific content.
pub use htmlmathml::define_htmlmathml;

/// Registers enclose functions (\colorbox, \fcolorbox, \fbox, \cancel, etc.) in
/// the KaTeX context.
///
/// This function defines LaTeX enclosure commands that add visual styling
/// around mathematical expressions, including colored backgrounds, borders, and
/// cancellation lines.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \colorbox{yellow}{x}     % Background color
/// \fcolorbox{red}{blue}{x} % Border and background colors
/// \fbox{x}                 % Simple border
/// \cancel{x}               % Diagonal cancellation line
/// \bcancel{x}              % Backslash cancellation
/// \xcancel{x}              % Cross cancellation
/// \sout{x}                 % Strikethrough
/// \phase{x}                % Phase angle symbol
/// \angl{x}                 % Actuarial angle
/// ```
///
/// # Supported Commands
///
/// The following enclosure commands are supported:
/// - `\colorbox`: Colored background
/// - `\fcolorbox`: Colored border and background
/// - `\fbox`: Simple border box
/// - `\cancel`: Upward diagonal strike
/// - `\bcancel`: Downward diagonal strike
/// - `\xcancel`: Cross (both diagonals)
/// - `\sout`: Horizontal strike
/// - `\phase`: Phase angle with SVG
/// - `\angl`: Actuarial angle
///
/// # Arguments
///
/// - `\colorbox`: Background color, content
/// - `\fcolorbox`: Border color, background color, content
/// - Other commands: Content only
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid color specifications are provided
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent-related commands.
pub use enclose::define_enclose;

/// Registers href functions (\href, \url) in the KaTeX context.
///
/// This function defines LaTeX hyperlink commands that create clickable links
/// within mathematical expressions. The `\href` command creates a hyperlink
/// with custom text, while `\url` creates a hyperlink from a URL that displays
/// the URL itself.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \href{https://example.com}{link text}    % Custom link text
/// \url{https://example.com}                % URL as link text
/// ```
///
/// # Arguments
///
/// - `\href`: URL (required), link text (required)
/// - `\url`: URL (required)
///
/// # Security Considerations
///
/// Both commands are subject to trust validation to prevent inclusion of
/// potentially malicious URLs. The trust settings determine whether external
/// URLs are permitted.
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
/// - The command is not trusted by security settings
///
/// # See Also
///
/// - [`define_includegraphics`] for other URL-related commands.
pub use href::define_href;

/// Registers color functions (\color, \textcolor, \colorbox, \fcolorbox) in the
/// KaTeX context.
///
/// This function defines LaTeX color commands that add color styling to
/// mathematical expressions, including text colors and colored backgrounds.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \textcolor{red}{text}        % Colored text
/// \color{blue}                 % Set current color
/// \colorbox{yellow}{content}   % Colored background
/// \fcolorbox{red}{blue}{text}  % Border and background colors
/// ```
///
/// # Arguments
///
/// - `\textcolor`: Color (required), content (required)
/// - `\color`: Color (required)
/// - `\colorbox`: Background color (required), content (required)
/// - `\fcolorbox`: Border color (required), background color (required),
///   content (required)
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid color specifications are provided
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_enclose`] for other visual styling commands.
pub use color::define_color;

/// Registers delimsizing functions (\bigl, \Bigl, \biggl, etc.) in the KaTeX
/// context.
///
/// This function defines LaTeX delimiter sizing commands that create
/// appropriately sized delimiters (parentheses, brackets, braces, etc.) based
/// on the content they enclose.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \bigl( \frac{a}{b} \bigr)     % Large left/right parentheses
/// \Bigl[ content \Bigr]         % Very large brackets
/// \biggl\{ content \biggr\}     % Extremely large braces
/// \Biggl\langle content \Biggr\rangle  % Extremely large angle brackets
/// ```
///
/// # Supported Commands
///
/// The following sizing commands are supported:
/// - `\bigl`, `\Bigl`, `\biggl`, `\Biggl` (left delimiters)
/// - `\bigr`, `\Bigr`, `\biggr`, `\Biggr` (right delimiters)
/// - `\bigm`, `\Bigm`, `\biggm`, `\Biggm` (middle delimiters)
/// - `\big`, `\Big`, `\bigg`, `\Bigg` (ordinary delimiters)
///
/// # Arguments
///
/// - Required: The delimiter symbol to be sized
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid delimiter symbol is provided
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_leftright`] for automatic delimiter sizing.
pub use delimsizing::define_delimsizing;

/// Registers leftright functions (\left, \right, \middle) in the KaTeX context.
///
/// This function defines LaTeX automatic delimiter sizing commands that create
/// delimiters that automatically scale to fit their enclosed content.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \left( \frac{a}{b} \right)     % Auto-sized parentheses
/// \left[ \sum_{i=1}^n x_i \right] % Auto-sized brackets
/// \left\{ x \middle| x > 0 \right\} % With middle delimiter
/// ```
///
/// # Supported Commands
///
/// - `\left`: Left delimiter with automatic sizing
/// - `\right`: Right delimiter with automatic sizing
/// - `\middle`: Middle delimiter within `\left...\right` pairs
///
/// # Arguments
///
/// - `\left`, `\right`, `\middle`: Delimiter symbol (required)
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid delimiter symbol is provided
/// - `\middle` is used outside `\left...\right` context
/// - Mismatched `\left` and `\right` delimiters
///
/// # See Also
///
/// - [`define_delimsizing`] for manual delimiter sizing.
pub use delimsizing::define_leftright;

/// Registers middle delimiter functions (\middle) in the KaTeX context.
///
/// This function defines the `\middle` command for creating middle delimiters
/// within `\left...\right` pairs.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \left\{ x \middle| x > 0 \right\}  % Middle vertical bar
/// ```
///
/// # Arguments
///
/// - Required: The middle delimiter symbol
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - `\middle` is used outside `\left...\right` context
/// - Invalid delimiter symbol is provided
///
/// # See Also
///
/// - [`define_leftright`] for left/right delimiter functions.
pub use delimsizing::define_middle;

/// Registers hbox functions (\hbox) in the KaTeX context.
///
/// This function defines the `\hbox` command that creates horizontal boxes
/// for text content in mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \hbox{text content}    % Horizontal box with text
/// ```
///
/// # Arguments
///
/// - Required: The text content to be placed in the horizontal box
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_vcenter`] for vertical centering.
pub use hbox::define_hbox;

/// Registers horizontal brace functions (\overbrace, \underbrace) in the KaTeX
/// context.
///
/// This function defines LaTeX horizontal brace commands that place stretchable
/// braces above or below mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \overbrace{a + b + c}      % Brace above expression
/// \underbrace{x + y + z}    % Brace below expression
/// ```
///
/// # Supported Commands
///
/// - `\overbrace`: Places a brace above the expression
/// - `\underbrace`: Places a brace below the expression
///
/// # Arguments
///
/// - Required: The mathematical expression to be braced
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent-related commands.
pub use horiz_brace::define_horiz_brace;

/// Registers HTML extension functions (\htmlClass, \htmlId, \htmlStyle,
/// \htmlData) in the KaTeX context.
///
/// This function defines LaTeX HTML extension commands that allow adding HTML
/// attributes and styling to mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \htmlClass{class-name}{content}    % Add CSS class
/// \htmlId{element-id}{content}       % Add HTML id
/// \htmlStyle{color: red}{content}    % Add inline styles
/// \htmlData{key=value}{content}      % Add data attributes
/// ```
///
/// # Supported Commands
///
/// - `\htmlClass`: Adds CSS class to the element
/// - `\htmlId`: Adds HTML id to the element
/// - `\htmlStyle`: Adds inline CSS styles
/// - `\htmlData`: Adds HTML data attributes
///
/// # Arguments
///
/// - Required: The attribute value
/// - Required: The mathematical expression content
///
/// # Security Considerations
///
/// These commands are subject to trust validation to prevent injection of
/// potentially malicious HTML attributes.
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
/// - The command is not trusted by security settings
///
/// # See Also
///
/// - [`define_href`] for hyperlink commands.
pub use html::define_html;

/// Registers kern functions (\kern) in the KaTeX context.
///
/// This function defines the `\kern` command for adding horizontal spacing
/// between elements in mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// a \kern{1em} b    % Add 1em horizontal space between a and b
/// ```
///
/// # Arguments
///
/// - Required: The amount of horizontal space to add
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid size specification is provided
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_rule`] for creating visible spacing elements.
pub use kern::define_kern;

/// Registers mathchoice functions (\mathchoice) in the KaTeX context.
///
/// This function defines the `\mathchoice` command that selects different
/// expressions based on the current math style (display, text, script,
/// scriptscript).
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \mathchoice{display}{text}{script}{scriptscript}
/// ```
///
/// # Arguments
///
/// - Required: Expression for display style
/// - Required: Expression for text style
/// - Required: Expression for script style
/// - Required: Expression for scriptscript style
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required arguments are missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_sizing`] for size-based styling.
pub use mathchoice::define_mathchoice;

/// Registers operator functions in the KaTeX context.
///
/// This function defines mathematical operators like integrals, sums, limits,
/// etc. with proper spacing and limit positioning behavior.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \int_a^b f(x) \, dx     % Integral with limits
/// \sum_{i=1}^n x_i        % Summation
/// \lim_{x \to 0} f(x)     % Limit
/// ```
///
/// # Supported Commands
///
/// - `\int`: Integral operator
/// - `\sum`: Summation operator
/// - `\prod`: Product operator
/// - `\lim`: Limit operator
/// - And many others
///
/// # Arguments
///
/// - Various operators may take limits as subscripts/superscripts
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Invalid argument types are provided
/// - Operator syntax is malformed
///
/// # See Also
///
/// - [`define_symbols_op`] for operator symbols.
pub use op::define_op;

/// Registers operator name functions in the KaTeX context.
///
/// This function defines operator name commands like `\operatorname` that
/// format custom operator names with proper spacing and limit positioning.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \operatorname{lim}     % Custom operator name
/// \operatorname*{argmax} % Custom operator with limits
/// ```
///
/// # Arguments
///
/// - Required: The operator name text
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_op`] for built-in operators.
pub use operatorname::define_operatorname;

/// Registers ordgroup functions (\@ordgroup) in the KaTeX context.
///
/// This function defines the `\@ordgroup` command for creating ordinary
/// mathematical groups with specific rendering behavior.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \@ordgroup{content}    % Ordinary mathematical group
/// ```
///
/// # Arguments
///
/// - Required: The mathematical expression content
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_mclass`] for math class commands.
pub use ordgroup::define_ordgroup;

/// Registers phantom functions (\phantom, \hphantom, \vphantom) in the KaTeX
/// context.
///
/// This function defines LaTeX phantom commands that create invisible elements
/// that occupy space without being visible, useful for spacing and alignment.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \phantom{content}     % Invisible but occupies full space
/// \hphantom{content}    % Invisible horizontal space only
/// \vphantom{content}    % Invisible vertical space only
/// ```
///
/// # Supported Commands
///
/// - `\phantom`: Occupies both horizontal and vertical space invisibly
/// - `\hphantom`: Occupies only horizontal space invisibly
/// - `\vphantom`: Occupies only vertical space invisibly
///
/// # Arguments
///
/// - Required: The content whose dimensions should be replicated invisibly
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_lap`] for overlapping content.
pub use phantom::define_phantom;

/// Registers the `\pmb` function in the KaTeX context.
///
/// The `\pmb` command creates poor man's bold text by overprinting characters
/// with slight offsets using CSS text-shadow. This simulates bold appearance
/// when proper bold fonts are not available.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \pmb{text}    % Poor man's bold text
/// ```
///
/// # Arguments
///
/// - Required: The text to be made bold
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_font`] for proper font styling commands.
pub use pmb::define_pmb;

/// Registers font styling functions in the KaTeX context.
///
/// This function defines LaTeX font commands that control the appearance of
/// text within mathematical expressions, including font families and styles.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \mathrm{roman text}      % Roman/upright font
/// \mathbf{bold text}       % Bold font
/// \mathit{italic text}     % Italic font
/// \mathsf{sans-serif}      % Sans-serif font
/// \mathtt{monospace}       % Monospace font
/// ```
///
/// # Supported Commands
///
/// - `\mathrm`: Roman/upright font
/// - `\mathbf`: Bold font
/// - `\mathit`: Italic font
/// - `\mathsf`: Sans-serif font
/// - `\mathtt`: Monospace font
/// - `\mathbb`: Blackboard bold (double-struck)
/// - `\mathfrak`: Fraktur font
/// - `\mathscr`: Script font
/// - `\mathcal`: Calligraphic font
///
/// # Arguments
///
/// - Required: The text to be styled
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_pmb`] for poor man's bold.
pub use font::define_font;

/// Registers sizing functions (\textstyle, \scriptstyle, etc.) in the KaTeX
/// context.
///
/// This function defines LaTeX sizing commands that change the font size
/// and style of mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \textstyle{expression}      % Text style size
/// \scriptstyle{expression}    % Script style size
/// \scriptscriptstyle{expr}    % Scriptscript style size
/// ```
///
/// # Supported Commands
///
/// - `\textstyle`: Text style sizing
/// - `\scriptstyle`: Script style sizing
/// - `\scriptscriptstyle`: Scriptscript style sizing
///
/// # Arguments
///
/// - Required: The mathematical expression to be sized
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_mathchoice`] for style-dependent content.
pub use sizing::define_sizing;

/// Registers smash functions (\smash) in the KaTeX context.
///
/// This function defines the `\smash` command that removes the height and/or
/// depth of mathematical expressions for alignment purposes.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \smash{expression}      % Remove both height and depth
/// \smash[t]{expression}   % Remove only height (top)
/// \smash[b]{expression}   % Remove only depth (bottom)
/// ```
///
/// # Arguments
///
/// - Required: The mathematical expression to be smashed
/// - Optional: Direction specifier `[t]` or `[b]`
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Invalid direction specifier is provided
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_phantom`] for invisible spacing.
pub use smash::define_smash;

/// Registers styling functions in the KaTeX context.
///
/// This function defines text styling commands that control the size and style
/// of mathematical expressions, such as display vs inline modes.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \textstyle{expression}      % Text style size
/// \scriptstyle{expression}    % Script style size
/// \scriptscriptstyle{expr}    % Scriptscript style size
/// ```
///
/// # Supported Commands
///
/// - `\textstyle`: Text style sizing
/// - `\scriptstyle`: Script style sizing
/// - `\scriptscriptstyle`: Scriptscript style sizing
///
/// # Arguments
///
/// - Required: The mathematical expression to be styled
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_sizing`] for size changes.
pub use styling::define_styling;

/// Registers symbol spacing functions (\,) in the KaTeX context.
///
/// This function defines LaTeX symbol spacing commands that add specific
/// amounts of horizontal spacing between mathematical elements.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// a \, b    % Thin space
/// a \: b    % Medium space
/// a \; b    % Thick space
/// a \! b    % Negative thin space
/// ```
///
/// # Supported Commands
///
/// - `\,`: Thin space (3/18 em)
/// - `\:`, `\>`: Medium space (4/18 em)
/// - `\;`: Thick space (5/18 em)
/// - `\!`: Negative thin space (-3/18 em)
/// - `\enspace`: Half em space
/// - `\quad`: 1 em space
/// - `\qquad`: 2 em space
///
/// # Arguments
///
/// - None (these are spacing commands)
///
/// # See Also
///
/// - [`define_kern`] for custom spacing amounts.
pub use symbols_spacing::define_spacing;

/// Registers text functions (\text, \textrm, \textsf, etc.) in the KaTeX
/// context.
///
/// This function defines LaTeX text commands that render text within
/// mathematical expressions with various font styles and families.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   functions are registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definitions.
///
/// # LaTeX Syntax
///
/// ```latex
/// \text{regular text}      % Regular text
/// \textrm{roman text}      % Roman font
/// \textsf{sans-serif}      % Sans-serif font
/// \texttt{monospace}       % Monospace font
/// \textbf{bold text}       % Bold text
/// \textit{italic text}     % Italic text
/// \emph{emphasized}        % Emphasized text
/// ```
///
/// # Supported Commands
///
/// Font families: `\text`, `\textrm`, `\textsf`, `\texttt`, `\textnormal`
/// Font weights: `\textbf`, `\textmd`
/// Font shapes: `\textit`, `\textup`, `\emph`
///
/// # Arguments
///
/// - Required: The text content to be rendered
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_accent`] for accent commands.
pub use text::define_text;

/// Registers underline functions (\underline) in the KaTeX context.
///
/// This function defines the `\underline` command that draws a horizontal
/// line below mathematical expressions.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   function is registered.
///
/// # Return Value
///
/// This function does not return a value; it modifies the provided context by
/// adding the function definition.
///
/// # LaTeX Syntax
///
/// ```latex
/// \underline{expression}    % Underline expression
/// ```
///
/// # Arguments
///
/// - Required: The mathematical expression to be underlined
///
/// # Error Handling
///
/// Errors may occur during parsing if:
/// - Required argument is missing
/// - Invalid argument types are provided
///
/// # See Also
///
/// - [`define_overline`] for overline commands.
pub use underline::define_underline;

/// Registers assemble_sup_sub utility functions in the KaTeX context.
///
/// This module provides utility functions for assembling superscript and
/// subscript elements with proper spacing and positioning for operators
/// with limits.
///
/// # Parameters
///
/// - `ctx`: A mutable reference to the [`crate::KatexContext`] where the
///   utilities are registered.
///
/// # Return Value
///
/// This function does not return a value; it provides utility functions for
/// other modules to use.
///
/// # See Also
pub use utils::assemble_sup_sub;
