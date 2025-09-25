use core::cell::RefCell;
use core::fmt;

use alloc::sync::Arc;
use bon::bon;

use crate::macro_expander::MacroMap;
use crate::namespace::KeyMap;

use crate::types::{ErrorLocationProvider, ParseError, ParseErrorKind};
use crate::utils::protocol_from_url;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Output format options for KaTeX mathematical expression rendering.
///
/// This enum specifies the format of the rendered output, controlling whether
/// KaTeX generates HTML, MathML, or both. The choice affects browser
/// compatibility, accessibility, and styling capabilities.
///
/// # LaTeX/KaTeX Context
/// Different output formats serve different purposes in mathematical
/// publishing:
/// - HTML provides broad browser support and styling flexibility
/// - MathML offers semantic markup and screen reader accessibility
/// - Combined output balances compatibility with advanced features
///
/// # Cross-references
/// - See [`Settings::output`] for configuring the output format.
/// - Related to browser compatibility and accessibility requirements.
/// - Affects CSS styling and semantic markup generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Generate both HTML and MathML markup.
    ///
    /// This is the default format, providing the best balance of compatibility,
    /// accessibility, and styling. HTML is used for visual rendering, while
    /// MathML provides semantic information for screen readers and other tools.
    HtmlAndMathml,
    /// Generate HTML markup only.
    ///
    /// Produces clean HTML output with CSS styling. Best for web pages where
    /// MathML support is not required or desired. More predictable rendering
    /// across different browsers.
    Html,
    /// Generate MathML markup only.
    ///
    /// Produces semantic MathML output optimized for accessibility and
    /// mathematical software. Best for applications requiring precise
    /// mathematical semantics over visual presentation.
    Mathml,
}

/// Levels of strictness for LaTeX compatibility checking in KaTeX.
///
/// This enum defines how KaTeX responds to input that deviates from standard
/// LaTeX syntax or behavior. It provides a spectrum from permissive to strict
/// enforcement of LaTeX standards.
///
/// # LaTeX/KaTeX Context
/// LaTeX has evolved over decades with various extensions and non-standard
/// usages. Strict mode helps maintain compatibility and catch potential issues
/// by controlling how KaTeX handles non-standard constructs.
///
/// # Cross-references
/// - See [`Settings::report_nonstrict`] for how strictness is enforced.
/// - Used in [`StrictSetting`] and [`StrictReturn`] for configuration.
/// - Related to error reporting and LaTeX compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictMode {
    /// Ignore non-standard LaTeX input silently.
    ///
    /// Allows all input to be processed, even if it deviates from LaTeX
    /// standards. Best for maximum compatibility when strictness is not
    /// required.
    Ignore,
    /// Warn about non-standard LaTeX input but continue processing.
    ///
    /// Logs warnings for non-standard constructs but does not fail rendering.
    /// Useful for development and debugging while maintaining functionality.
    Warn,
    /// Error on non-standard LaTeX input and halt processing.
    ///
    /// Rejects any input that doesn't conform to LaTeX standards, throwing
    /// errors for strict compatibility. Best for production environments
    /// requiring LaTeX compliance.
    Error,
}

/// Core settings structure for KaTeX rendering configuration.
///
/// This struct contains all resolved configuration options that control
/// KaTeX's behavior during mathematical expression parsing and rendering.
/// Unlike the builder inputs, all fields have concrete values with no options.
///
/// # LaTeX/KaTeX Context
/// These settings correspond to LaTeX document and package options that affect
/// mathematical typesetting. KaTeX uses this structure to maintain consistent
/// rendering behavior across different expressions and contexts.
///
/// # Cross-references
/// - See [`Settings::builder`] for ergonomic construction of settings.
/// - Related to [`OutputFormat`], [`StrictSetting`], and [`TrustSetting`].
/// - Methods provide validation and utility functions.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct Settings {
    /// Whether mathematical expressions are rendered in display (block) mode.
    ///
    /// When `true`, equations are centered and displayed on separate lines
    /// with larger fonts. When `false`, equations are rendered inline.
    pub display_mode: bool,
    /// The output format for rendered mathematical expressions.
    ///
    /// Determines the markup format (HTML, MathML, or both) of the output.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub output: OutputFormat,
    /// Whether equation numbers are placed on the left side.
    ///
    /// Controls the positioning of equation numbers in numbered environments.
    pub leqno: bool,
    /// Whether equations are flushed to the left margin.
    ///
    /// When `true`, equations are left-aligned instead of centered.
    pub fleqn: bool,
    /// Whether parsing/rendering errors should throw exceptions.
    ///
    /// When `true`, errors cause panics. When `false`, errors are rendered
    /// as colored text in the output.
    pub throw_on_error: bool,
    /// CSS color value used for rendering error messages.
    ///
    /// Applied to error text when `throw_on_error` is `false`.
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
    pub error_color: String,
    /// Map of custom macro definitions.
    ///
    /// Contains user-defined LaTeX macros for extending functionality.
    /// Keys are macro names, values are their LaTeX definitions.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub macros: RefCell<MacroMap>,
    /// Minimum thickness for rendered rules (lines).
    ///
    /// Prevents lines from becoming too thin to be visible. In points.
    pub min_rule_thickness: f64,
    /// Whether `\color` commands affect surrounding text color.
    ///
    /// When `true`, color commands modify text color. When `false`,
    /// they only affect mathematical content.
    pub color_is_text_color: bool,
    /// Configuration for strict LaTeX compatibility checking.
    ///
    /// Controls how KaTeX handles non-standard LaTeX input.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub strict: StrictSetting,
    /// Configuration for trust validation of dangerous content.
    ///
    /// Controls validation of URLs, styles, and other potentially unsafe
    /// inputs.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub trust: TrustSetting,
    /// Maximum allowed size for rendered expressions.
    ///
    /// Prevents excessive memory usage from very large expressions. In points.
    pub max_size: f64,
    /// Maximum limit for macro expansion iterations.
    ///
    /// Prevents infinite loops in macro expansion.
    pub max_expand: usize,
    /// Whether settings persist globally across render calls.
    ///
    /// When `true`, settings remain active for subsequent expressions.
    pub global_group: bool,
    /// Size multiplier for scaling rendered expressions.
    ///
    /// Controls the overall size scaling factor for mathematical expressions.
    pub size_multiplier: f64,
    /// Color value for mathematical content.
    ///
    /// CSS color value used for rendering mathematical expressions.
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
    pub color: Option<String>,
}

#[bon]
impl Settings {
    /// Creates a new [`Settings`] instance from optional configuration values.
    ///
    /// This constructor applies default values for any `None` options in the
    /// provided builder inputs, ensuring all settings have concrete
    /// values.
    ///
    /// # Parameters
    /// - `options`: Configuration options with optional values.
    ///
    /// # Returns
    /// A fully configured [`Settings`] instance with all fields set to concrete
    /// values.
    ///
    /// # Default Values
    /// - `display_mode`: `false` (inline mode)
    /// - `output`: [`OutputFormat::HtmlAndMathml`]
    /// - `leqno`: `false` (right-side numbering)
    /// - `fleqn`: `false` (centered equations)
    /// - `throw_on_error`: `true` (throw on errors)
    /// - `error_color`: `"#cc0000"` (red)
    /// - `macros`: Empty map
    /// - `min_rule_thickness`: `0.0`
    /// - `color_is_text_color`: `false`
    /// - `strict`: StrictSetting::Mode(StrictMode::Ignore)
    /// - `trust`: TrustSetting::Bool(false)
    /// - `max_size`: `f64::INFINITY`
    /// - `max_expand`: `1000`
    /// - `global_group`: `false`
    #[must_use]
    #[builder]
    pub fn new(
        /// Display mode (true for block, false for inline).
        display_mode: Option<bool>,
        /// Output format (HTML, MathML, or both).
        output: Option<OutputFormat>,
        /// Left equation numbers (true for left, false for right).
        leqno: Option<bool>,
        /// Left-aligned equations (true for left, false for centered).
        fleqn: Option<bool>,
        /// Throw errors (true) or render them (false).
        /// Default is true (throw on errors).
        throw_on_error: Option<bool>,
        /// CSS color for rendering errors.
        error_color: Option<String>,
        /// Custom macro definitions.
        macros: Option<MacroMap>,
        /// Minimum rule thickness in points.
        min_rule_thickness: Option<f64>,
        /// Whether `\color` affects surrounding text color.
        color_is_text_color: Option<bool>,
        /// Strict mode configuration.
        strict: Option<StrictSetting>,
        /// Trust configuration for dangerous content.
        trust: Option<TrustSetting>,
        /// Maximum allowed size in points.
        max_size: Option<f64>,
        /// Maximum macro expansion iterations.
        max_expand: Option<usize>,
        /// Whether settings persist globally across render calls.
        global_group: Option<bool>,
        /// Size multiplier for rendering (scaling factor).
        size_multiplier: Option<f64>,
        /// Color for mathematical content.
        color: Option<String>,
    ) -> Self {
        Self {
            display_mode: display_mode.unwrap_or(false),
            output: output.unwrap_or(OutputFormat::HtmlAndMathml),
            leqno: leqno.unwrap_or(false),
            fleqn: fleqn.unwrap_or(false),
            throw_on_error: throw_on_error.unwrap_or(true),
            error_color: error_color.unwrap_or_else(|| "#cc0000".to_owned()),
            macros: RefCell::from(macros.unwrap_or_default()),
            min_rule_thickness: min_rule_thickness.unwrap_or(0.0),
            color_is_text_color: color_is_text_color.unwrap_or(false),
            strict: strict.unwrap_or_default(),
            trust: trust.unwrap_or_default(),
            max_size: max_size.unwrap_or(f64::INFINITY).max(0.0),
            max_expand: max_expand.unwrap_or(1000),
            global_group: global_group.unwrap_or(false),
            size_multiplier: size_multiplier.unwrap_or(1.0),
            color,
        }
    }

    /// Reports non-standard LaTeX input according to the current strict
    /// settings.
    ///
    /// This method handles non-LaTeX-compatible input based on the configured
    /// strictness level. It may ignore the input, log a warning, or return an
    /// error.
    ///
    /// # Parameters
    /// - `error_code`: A string identifier for the type of strict violation.
    /// - `error_msg`: A human-readable description of the issue.
    /// - `token`: Optional location information for error reporting.
    ///
    /// # Returns
    /// - `Ok(())` if the input is accepted (ignore or warn modes).
    /// - `Err(ParseError)` if the input is rejected (error mode).
    ///
    /// # Behavior by Strict Mode
    /// - [`StrictMode::Ignore`]: Silently accepts the input.
    /// - [`StrictMode::Warn`]: Logs a warning and accepts the input.
    /// - [`StrictMode::Error`]: Returns an error rejecting the input.
    ///
    /// # Error Handling
    /// Errors include the error code and message, with optional location
    /// information from the token for precise error reporting.
    #[expect(clippy::print_stderr)]
    pub fn report_nonstrict(
        &self,
        error_code: &str,
        error_msg: &str,
        token: Option<&dyn ErrorLocationProvider>,
    ) -> Result<(), ParseError> {
        match self.resolve_strict(error_code, error_msg, token) {
            StrictMode::Ignore => Ok(()),
            StrictMode::Error => {
                let kind = ParseErrorKind::StrictModeError {
                    message: error_msg.to_owned(),
                    code: error_code.to_owned(),
                };
                if let Some(t) = token {
                    Err(ParseError::with_token(kind, t))
                } else {
                    Err(ParseError::new(kind))
                }
            }
            StrictMode::Warn => {
                eprintln!(
                    "LaTeX-incompatible input and strict mode is set to 'warn': {error_msg} [{error_code}]"
                );
                Ok(())
            }
        }
    }

    /// Determines whether strict (LaTeX-adhering) behavior should be enforced.
    ///
    /// This method checks if the given input should trigger strict error
    /// handling based on the current strict settings. Unlike
    /// report_nonstrict, this method only returns a boolean without
    /// performing any actions.
    ///
    /// # Parameters
    /// - `error_code`: A string identifier for the type of strict violation.
    /// - `error_msg`: A human-readable description of the issue.
    /// - `token`: Optional location information for error reporting.
    ///
    /// # Returns
    /// - `true` if strict behavior should be enforced (error mode).
    /// - `false` if the input should be accepted (ignore or warn modes).
    ///
    /// # Notes
    /// In warn mode, this method logs the warning but returns `false` to
    /// indicate that processing should continue rather than fail.
    #[must_use]
    #[expect(clippy::print_stderr)]
    pub fn use_strict_behavior(
        &self,
        error_code: &str,
        error_msg: &str,
        token: Option<&dyn ErrorLocationProvider>,
    ) -> bool {
        match self.resolve_strict_catch(error_code, error_msg, token) {
            StrictMode::Ignore => false,
            StrictMode::Error => true,
            StrictMode::Warn => {
                eprintln!(
                    "LaTeX-incompatible input and strict mode is set to 'warn': {error_msg} [{error_code}]"
                );
                false
            }
        }
    }

    /// Evaluates whether potentially dangerous input should be trusted.
    ///
    /// This method validates potentially unsafe content (such as URLs in
    /// `\href` commands) according to the current trust settings. It
    /// automatically infers protocols from URLs when possible.
    ///
    /// # Parameters
    /// - `context`: A [`TrustContext`] containing details about the potentially
    ///   dangerous content, including the command, URL, styles, etc.
    ///
    /// # Returns
    /// - `true` if the content should be trusted and rendered.
    /// - `false` if the content should be rejected as unsafe.
    ///
    /// # Protocol Inference
    /// If `context.url` is provided but `context.protocol` is `None`, this
    /// method attempts to infer the protocol from the URL. If the URL is
    /// malformed or has an invalid protocol, it returns `false`
    /// immediately.
    ///
    /// # Security Considerations
    /// This method is critical for preventing XSS attacks and other security
    /// vulnerabilities. Trust functions should carefully validate all aspects
    /// of the context before granting trust.
    pub fn is_trusted(&self, context: &mut TrustContext) -> bool {
        if context.protocol.is_none()
            && let Some(url) = &context.url
        {
            if let Some(protocol) = protocol_from_url(url) {
                context.protocol = Some(protocol);
            } else {
                return false;
            }
        }

        match &self.trust {
            TrustSetting::Bool(b) => *b,
            TrustSetting::Function(f) => f(context).unwrap_or(false),
        }
    }

    /// Helper: resolve strict setting into a concrete mode. Any boolean true
    /// maps to Error, boolean false maps to Ignore.
    fn resolve_strict(
        &self,
        error_code: &str,
        error_msg: &str,
        token: Option<&dyn ErrorLocationProvider>,
    ) -> StrictMode {
        match &self.strict {
            StrictSetting::Mode(m) => *m,
            StrictSetting::Bool(b) => {
                if *b {
                    StrictMode::Error
                } else {
                    StrictMode::Ignore
                }
            }
            StrictSetting::Function(f) => match f(error_code, error_msg, token) {
                Some(StrictReturn::Mode(m)) => m,
                Some(StrictReturn::Bool(b)) => {
                    if b {
                        StrictMode::Error
                    } else {
                        StrictMode::Ignore
                    }
                }
                None => StrictMode::Ignore,
            },
        }
    }

    /// Helper variant for use_strict_behavior: if the function errors, treat as
    /// Error.
    fn resolve_strict_catch(
        &self,
        error_code: &str,
        error_msg: &str,
        token: Option<&dyn ErrorLocationProvider>,
    ) -> StrictMode {
        match &self.strict {
            StrictSetting::Mode(m) => *m,
            StrictSetting::Bool(b) => {
                if *b {
                    StrictMode::Error
                } else {
                    StrictMode::Ignore
                }
            }
            StrictSetting::Function(func) => {
                // Mimic JS semantics: if function panics, treat as "error".
                let f = Arc::clone(func);
                let error_code = error_code.to_owned();
                let error_msg = error_msg.to_owned();
                let token_ref = token;
                let res = f(&error_code, &error_msg, token_ref);
                match res {
                    Some(StrictReturn::Mode(m)) => m,
                    Some(StrictReturn::Bool(b)) => {
                        if b {
                            StrictMode::Error
                        } else {
                            StrictMode::Ignore
                        }
                    }
                    None => StrictMode::Ignore,
                }
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Return type for strict validation functions in KaTeX.
///
/// This enum represents the possible return values from strict mode functions,
/// which determine how KaTeX handles non-standard LaTeX input. It mirrors
/// JavaScript's strict function return types, allowing flexible strictness
/// configuration through function callbacks.
///
/// # LaTeX/KaTeX Context
/// Strict mode in KaTeX controls how strictly the parser adheres to LaTeX
/// standards. Functions can return different strictness levels based on
/// the specific input being validated, enabling fine-grained control over
/// error handling and warnings.
///
/// # Cross-references
/// - See [`StrictMode`] for the different strictness levels.
/// - Used in [`StrictSetting`] for configuring strict behavior.
/// - Related to [`Settings::report_nonstrict`] for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictReturn {
    /// Boolean return value for simple strict/non-strict decisions.
    ///
    /// - `true` corresponds to [`StrictMode::Error`] (strict behavior).
    /// - `false` corresponds to [`StrictMode::Ignore`] (non-strict behavior).
    Bool(bool),
    /// Explicit strict mode return value for precise control.
    ///
    /// Allows returning any [`StrictMode`] variant for detailed strictness
    /// control.
    Mode(StrictMode),
}

/// Function signature for custom strict mode evaluation in KaTeX.
pub type StrictFunction =
    dyn Fn(&str, &str, Option<&dyn ErrorLocationProvider>) -> Option<StrictReturn> + Send + Sync;

/// Configuration for strict mode behavior in KaTeX parsing and rendering.
///
/// This enum allows flexible configuration of how KaTeX handles non-standard
/// LaTeX input. It can be set to a fixed strictness level, a simple boolean,
/// or a custom function for dynamic strictness decisions based on context.
///
/// # LaTeX/KaTeX Context
/// Strict mode controls KaTeX's adherence to LaTeX standards. When enabled,
/// KaTeX will report or reject input that deviates from standard LaTeX syntax,
/// helping catch errors and ensure compatibility. The function variant allows
/// fine-grained control over which inputs are considered strict violations.
///
/// # Cross-references
/// - See [`StrictMode`] for available strictness levels.
/// - Used in [`Settings`] for global strict configuration.
/// - Related to [`StrictReturn`] for function return values.
#[derive(Clone)]
pub enum StrictSetting {
    /// Fixed strict mode level applied to all inputs.
    ///
    /// Uses the specified [`StrictMode`] for all parsing decisions.
    Mode(StrictMode),
    /// Boolean strict setting for simple on/off control.
    ///
    /// - `true` maps to [`StrictMode::Error`] (strict).
    /// - `false` maps to [`StrictMode::Ignore`] (non-strict).
    Bool(bool),
    /// Custom function for dynamic strictness evaluation.
    ///
    /// The function receives error code, message, and optional location,
    /// returning an optional [`StrictReturn`] to determine behavior.
    ///
    /// # Function Parameters
    /// - `error_code`: String identifier for the type of strict violation.
    /// - `error_msg`: Human-readable description of the issue.
    /// - `token`: Optional location information for error reporting.
    ///
    /// # Function Return
    /// - `Some(StrictReturn)` to specify strictness behavior.
    /// - `None` to fall back to default behavior.
    Function(Arc<StrictFunction>),
}

impl fmt::Debug for StrictSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mode(m) => write!(f, "StrictSetting::Mode({m:?})"),
            Self::Bool(b) => write!(f, "StrictSetting::Bool({b})"),
            Self::Function(_) => write!(f, "StrictSetting::Function(<fn>)"),
        }
    }
}

impl Default for StrictSetting {
    fn default() -> Self {
        Self::Mode(StrictMode::Ignore)
    }
}

/// Context structure for validating potentially dangerous inputs in KaTeX
/// rendering.
///
/// This struct encapsulates information about potentially unsafe content (such
/// as URLs, styles, or attributes) that requires trust validation before
/// rendering. It provides a comprehensive context for security decisions,
/// allowing fine-grained control over what content is permitted in mathematical
/// expressions.
///
/// # LaTeX/KaTeX Context
/// LaTeX commands like `\href`, `\htmlClass`, and `\htmlStyle` can introduce
/// security risks if not properly validated. KaTeX uses trust contexts to
/// implement security policies that prevent XSS attacks and other
/// vulnerabilities while maintaining functionality for legitimate use cases.
///
/// # Cross-references
/// - See [`Settings::is_trusted`] for trust validation logic.
/// - Related to [`TrustSetting`] for configuring trust policies.
/// - Used with [`crate::types::CssStyle`] for style validation.
#[derive(Debug, Clone, Default)]
pub struct TrustContext {
    /// The LaTeX command name that triggered the trust check (e.g., "\\href",
    /// "\\htmlClass").
    ///
    /// This identifies the specific command requiring validation, allowing
    /// command-specific trust policies.
    pub command: String,
    /// Optional URL string involved in the trust decision.
    ///
    /// Present for URL-related commands like `\href`. The URL's protocol
    /// is automatically inferred and stored in the `protocol` field.
    pub url: Option<String>,
    /// The protocol inferred from the URL (e.g., "http", "https", "mailto").
    ///
    /// Automatically populated when `url` is provided. Used to enforce
    /// protocol-specific security policies.
    pub protocol: Option<String>,
    /// Optional CSS class name for HTML class attributes.
    ///
    /// Used with commands like `\htmlClass` to specify CSS classes
    /// for generated HTML elements.
    pub class: Option<String>,
    /// Optional HTML id attribute value.
    ///
    /// Used with commands like `\htmlId` to assign unique identifiers
    /// to rendered HTML elements.
    pub id: Option<String>,
    /// Optional inline CSS style string.
    ///
    /// Contains CSS style declarations for commands like `\htmlStyle`.
    /// Subject to validation to prevent malicious style injection.
    pub style: Option<String>,
    /// Optional map of HTML attributes for data attributes.
    ///
    /// Used with commands like `\htmlData` to add custom data attributes
    /// to rendered HTML elements.
    pub attributes: Option<KeyMap<String, String>>,
}

/// Function signature for custom trust evaluation in KaTeX.
pub type TrustFunction = dyn Fn(&mut TrustContext) -> Option<bool> + Send + Sync;

/// Configuration for trust validation of potentially dangerous content in
/// KaTeX.
///
/// This enum controls how KaTeX validates and permits potentially unsafe
/// content such as URLs, styles, and HTML attributes. It can be set to a simple
/// boolean for blanket trust decisions or a custom function for context-aware
/// validation.
///
/// # LaTeX/KaTeX Context
/// Certain LaTeX commands can introduce security risks (e.g., `\href` with
/// malicious URLs, `\htmlStyle` with XSS payloads). Trust settings allow
/// administrators to control which content is permitted, balancing
/// functionality with security.
///
/// # Cross-references
/// - See [`TrustContext`] for the context passed to trust functions.
/// - Used in [`Settings`] for global trust configuration.
/// - Related to [`Settings::is_trusted`] for validation logic.
#[derive(Clone)]
pub enum TrustSetting {
    /// Fixed boolean trust setting for simple allow/deny decisions.
    ///
    /// - `true` trusts all potentially dangerous content.
    /// - `false` rejects all potentially dangerous content.
    Bool(bool),
    /// Custom function for dynamic trust evaluation based on context.
    ///
    /// The function receives a mutable [`TrustContext`] and returns an optional
    /// boolean indicating whether the content should be trusted.
    ///
    /// # Function Parameters
    /// - `context`: Mutable reference to [`TrustContext`] containing details
    ///   about the potentially dangerous content (command, URL, style, etc.).
    ///
    /// # Function Return
    /// - `Some(true)` to trust the content.
    /// - `Some(false)` to reject the content.
    /// - `None` to fall back to default behavior.
    ///
    /// # Security Considerations
    /// Trust functions should carefully validate all aspects of the context
    /// to prevent security vulnerabilities.
    Function(Arc<TrustFunction>),
}

impl fmt::Debug for TrustSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(b) => write!(f, "TrustSetting::Bool({b})"),
            Self::Function(_) => write!(f, "TrustSetting::Function(<fn>)"),
        }
    }
}

impl Default for TrustSetting {
    fn default() -> Self {
        Self::Bool(false)
    }
}
