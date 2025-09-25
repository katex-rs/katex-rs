//! Utility functions for KaTeX Rust implementation
//! Provides common utility functions for string manipulation, type checking,
//! and helper operations.

use core::fmt;
use core::ops::{Deref, DerefMut};

/// Converts a camelCase string to hyphen-case.
///
/// This function is useful for converting CSS class names or identifiers
/// from camelCase to a hyphen-separated format, common in web-based
/// mathematical rendering for generating CSS classes from KaTeX element names.
///
/// # Parameters
/// - `s`: The input string in camelCase format.
///
/// # Returns
/// Returns a new `String` with uppercase letters converted to lowercase and
/// preceded by hyphens.
///
/// # Examples
/// ```
/// use katex::utils::hyphenate;
///
/// assert_eq!(hyphenate("camelCase"), "camel-case");
/// assert_eq!(hyphenate("XMLHttpRequest"), "x-m-l-http-request");
/// ```
///
/// # Notes
/// - The first character is always lowercase.
/// - Consecutive uppercase letters are treated as separate words.
#[must_use]
pub fn hyphenate(s: &str) -> String {
    let mut out = String::new();

    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }

    out
}

/// Escapes HTML special characters in a string to prevent XSS attacks.
///
/// This function replaces dangerous characters with their HTML entity
/// equivalents, essential for safely rendering user-provided content in
/// web-based KaTeX output, such as mathematical expressions containing special
/// characters.
///
/// # Parameters
/// - `text`: The input string that may contain HTML special characters.
///
/// # Returns
/// Returns a new `String` with special characters escaped.
///
/// # Examples
/// ```
/// use katex::utils::escape;
///
/// assert_eq!(escape("a & b"), "a &amp; b");
/// assert_eq!(escape("a > b"), "a &gt; b");
/// assert_eq!(escape("a < b"), "a &lt; b");
/// assert_eq!(escape("a \" b"), "a &quot; b");
/// assert_eq!(escape("a ' b"), "a &#x27; b");
/// ```
///
/// # Escaped Characters
/// - `&` → `&amp;`
/// - `>` → `&gt;`
/// - `<` → `&lt;`
/// - `"` → `&quot;`
/// - `'` → `&#x27;`
#[must_use]
pub fn escape(text: &str) -> String {
    let mut escaped = String::new();
    let _ = escape_into(&mut escaped, text);
    escaped
}

/// Writes the escaped HTML representation of `text` into the provided writer.
///
/// This helper avoids intermediate string allocations by emitting directly
/// into any `fmt::Write` implementor. It mirrors [`escape`] but is optimized
/// for streaming renderers where the output is already buffered elsewhere.
#[inline]
pub fn escape_into<W: fmt::Write>(writer: &mut W, text: &str) -> fmt::Result {
    let mut last = 0;
    for (idx, ch) in text.char_indices() {
        let replacement = match ch {
            '&' => Some("&amp;"),
            '>' => Some("&gt;"),
            '<' => Some("&lt;"),
            '"' => Some("&quot;"),
            '\'' => Some("&#x27;"),
            _ => None,
        };

        if let Some(rep) = replacement {
            if last < idx {
                writer.write_str(&text[last..idx])?;
            }
            writer.write_str(rep)?;
            last = idx + ch.len_utf8();
        }
    }

    if last < text.len() {
        writer.write_str(&text[last..])
    } else {
        Ok(())
    }
}

/// Extracts the protocol from a URL string.
///
/// This function parses the URL to determine its protocol (scheme),
/// handling various edge cases like HTML entities for colons.
/// Useful for validating URLs in mathematical content that may include links.
///
/// # Parameters
/// - `url`: The URL string to parse.
///
/// # Returns
/// - `Some(protocol)`: The lowercase protocol string if valid.
/// - `Some("_relative")`: If the URL is relative (no protocol).
/// - `None`: If the protocol is invalid.
///
/// # Examples
/// ```
/// use katex::utils::protocol_from_url;
///
/// assert_eq!(
///     protocol_from_url("https://example.com"),
///     Some("https".to_string())
/// );
/// assert_eq!(
///     protocol_from_url("/path/to/file"),
///     Some("_relative".to_string())
/// );
/// assert_eq!(protocol_from_url("1invalid://example.com"), None);
/// ```
///
/// # Error Handling
/// - Invalid schemes (not starting with letter, containing invalid chars)
///   return `None`.
/// - HTML entities for colon are handled (e.g., `&#58;`, `&colon;`).
///
/// # Cross-references
/// See validate_and_return for scheme validation logic.
/// See match_html_colon_entity for HTML entity parsing.
#[must_use]
pub fn protocol_from_url(url: &str) -> Option<String> {
    let mut s = url;

    while let Some(first) = s.chars().next() {
        if first <= '\u{20}' {
            s = &s[first.len_utf8()..];
        } else {
            break;
        }
    }

    for (i, ch) in s.char_indices() {
        if ch == ':' {
            return validate_and_return(&s[..i], ":", &s[i + 1..]);
        } else if ch == '&' {
            let rest = &s[i..];
            if let Some((entity, skip)) = match_html_colon_entity(rest) {
                return validate_and_return(&s[..i], entity, &s[i + skip..]);
            }
        } else if ch == '\\' || ch == '/' || ch == '#' || ch == '?' {
            return Some("_relative".into());
        }
    }

    Some("_relative".into())
}

fn match_html_colon_entity(s: &str) -> Option<(&'static str, usize)> {
    // &#0*58
    if let Some(decimal) = s.strip_prefix("&#") {
        let mut idx = 0;
        while decimal[idx..].starts_with('0') {
            idx += 1;
        }
        if decimal[idx..].starts_with("58") {
            let after = idx + 2;
            return Some(("&#0*58", 2 + after));
        }
        if decimal[idx..].starts_with('x') || decimal[idx..].starts_with('X') {
            let hexpart = &decimal[idx + 1..];
            let mut idx2 = 0;
            while hexpart[idx2..].starts_with('0') {
                idx2 += 1;
            }
            if hexpart[idx2..].to_ascii_lowercase().starts_with("3a") {
                let after = idx + 1 + idx2 + 2;
                return Some(("&#x0*3a", 2 + after));
            }
        }
    }
    // &colon
    if s.to_ascii_lowercase().starts_with("&colon") {
        return Some(("&colon", 6));
    }
    None
}

fn validate_and_return(scheme: &str, colon_match: &str, _rest: &str) -> Option<String> {
    if colon_match != ":" {
        return None;
    }

    let mut chars = scheme.chars();
    if !matches!(chars.next(), Some(c) if c.is_ascii_alphabetic()) {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
        return None;
    }
    Some(scheme.to_lowercase())
}

/// Push a value onto a vector and return a reference to the new element and the
/// vector.
pub fn push_and_get_ref<T>(vec: &mut Vec<T>, value: T) -> (&T, &Vec<T>) {
    vec.push(value);
    let idx = vec.len() - 1;
    (&vec[idx], vec)
}

/// Push a value onto a vector and return a mutable reference to the new
/// element.
pub fn push_and_get_mut<T>(vec: &mut Vec<T>, value: T) -> &mut T {
    vec.push(value);
    let idx = vec.len() - 1;
    &mut vec[idx]
}

/// A type that can either own a value or borrow a mutable reference to it.
pub enum OwnedOrMut<'a, T> {
    /// An owned value with its index in the original collection.
    Owned {
        /// The index of the owned value in the original collection.
        idx: usize,
        /// The owned value.
        val: T,
    },
    /// A mutable reference to a value.
    Borrowed(&'a mut T),
}

impl<T> Deref for OwnedOrMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            OwnedOrMut::Owned { val: arr, .. } => arr,
            OwnedOrMut::Borrowed(r) => r,
        }
    }
}

impl<T> DerefMut for OwnedOrMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            OwnedOrMut::Owned { val: arr, .. } => arr,
            OwnedOrMut::Borrowed(r) => r,
        }
    }
}

/// Sets a panic hook for better error messages in WebAssembly.
#[allow(clippy::missing_const_for_fn)]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "wasm")]
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyphenate() {
        assert_eq!(hyphenate("camelCase"), "camel-case");
        assert_eq!(hyphenate("XMLHttpRequest"), "x-m-l-http-request");
    }

    #[test]
    fn test_escape() {
        assert_eq!(escape("a & b"), "a &amp; b");
        assert_eq!(escape("a > b"), "a &gt; b");
        assert_eq!(escape("a < b"), "a &lt; b");
        assert_eq!(escape("a \" b"), "a &quot; b");
        assert_eq!(escape("a ' b"), "a &#x27; b");
    }

    #[test]
    fn test_protocol_from_url() {
        assert_eq!(
            protocol_from_url("https://example.com"),
            Some("https".to_owned())
        );
        assert_eq!(
            protocol_from_url("http://example.com"),
            Some("http".to_owned())
        );
        assert_eq!(
            protocol_from_url("ftp://example.com"),
            Some("ftp".to_owned())
        );
        assert_eq!(
            protocol_from_url("/path/to/file"),
            Some("_relative".to_owned())
        );
        assert_eq!(protocol_from_url("1invalid://example.com"), None);
        assert_eq!(protocol_from_url("weird:colon"), Some("weird".to_owned()));
    }

    #[test]
    fn test_protocol_from_url_misc() {
        assert_eq!(protocol_from_url("/path/to/file"), Some("_relative".into()));
        assert_eq!(
            protocol_from_url("  \u{0007}../rel"),
            Some("_relative".into())
        );

        assert_eq!(
            protocol_from_url("https://example.com"),
            Some("https".into())
        );
        assert_eq!(protocol_from_url("FTP://example.com"), Some("ftp".into()));
        assert_eq!(
            protocol_from_url("mailto:user@example.com"),
            Some("mailto".into())
        );

        assert_eq!(protocol_from_url("http&#058//foo"), None);
        assert_eq!(protocol_from_url("http&#x03a//foo"), None);
        assert_eq!(protocol_from_url("http&colon//foo"), None);

        assert_eq!(protocol_from_url("1abc://foo"), None);
        assert_eq!(protocol_from_url("abc^://foo"), None);
        assert_eq!(protocol_from_url("ht tp://foo"), None);
    }
}
