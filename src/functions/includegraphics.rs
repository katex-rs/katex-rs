//! Implementation of the \includegraphics command in KaTeX
//!
//! This module implements the LaTeX `\includegraphics` command, which includes
//! external images or graphics in mathematical expressions with specified
//! dimensions.

use crate::context::KatexContext;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::{HtmlDomNode, Img};
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{
    AnyParseNode, NodeType, ParseNode, ParseNodeColor, ParseNodeIncludegraphics, ParseNodeText,
    ParseNodeTextOrd,
};
use crate::spacing_data::MeasurementOwned;
use crate::types::{
    ArgType, CssProperty, CssStyle, Mode, ParseError, ParseErrorKind, TrustContext,
};
use crate::units::{make_em, valid_unit};

/// Check if a string matches the pattern for a plain number (no unit)
/// Equivalent to /^[-+]? *(\d+(\.\d*)?|\.\d+)$/ regex
fn is_plain_number(s: &str) -> bool {
    let trimmed = s.trim();
    let mut chars = trimmed.chars().peekable();

    // Optional sign
    if let Some(&c) = chars.peek()
        && (c == '+' || c == '-')
    {
        chars.next();
    }

    // Skip spaces after sign
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
        } else {
            break;
        }
    }

    let remaining: String = chars.collect();
    if remaining.is_empty() {
        return false;
    }

    // Check if it matches: \d+(\.\d*)? or \.\d+
    let bytes = remaining.as_bytes();
    let mut i = 0;
    let mut has_digit = false;

    // Parse digits before decimal point
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digit = true;
        i += 1;
    }

    // Check for decimal point and digits after
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        if !has_digit && i >= bytes.len() {
            return false; // Just a dot with no digits
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digit = true;
            i += 1;
        }
    }

    has_digit && i == bytes.len()
}

/// Parse a size string with pattern ([-+]?) *(\d+(?:\.\d*)?|\.\d+) *([a-z]{2})
fn parse_size_with_unit(s: &str) -> Option<(String, String, String)> {
    let mut chars = s.chars().peekable();
    let mut sign = String::new();
    let mut number = String::new();
    let mut unit = String::new();

    // Parse optional sign
    if let Some(&c) = chars.peek()
        && (c == '+' || c == '-')
    {
        sign.push(c);
        chars.next();
    }

    // Skip spaces after sign
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
        } else {
            break;
        }
    }

    // Parse number (\d+(?:\.\d*)?|\.\d+)
    let mut has_digit = false;

    // First, try to match digits
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_digit = true;
            number.push(c);
            chars.next();
        } else {
            break;
        }
    }

    // Check for decimal point
    if chars.peek() == Some(&'.') {
        number.push('.');
        chars.next();

        // Parse digits after decimal point
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                has_digit = true;
                number.push(c);
                chars.next();
            } else {
                break;
            }
        }
    } else if !has_digit {
        // Try to match .\d+ pattern
        if chars.peek() == Some(&'.') {
            number.push('.');
            chars.next();

            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    has_digit = true;
                    number.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
        }
    }

    if !has_digit {
        return None;
    }

    // Skip spaces before unit
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
        } else {
            break;
        }
    }

    // Parse exactly 2 lowercase letters for unit
    for _ in 0..2 {
        if let Some(&c) = chars.peek() {
            if c.is_ascii_lowercase() {
                unit.push(c);
                chars.next();
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    // Skip trailing spaces
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
        } else {
            break;
        }
    }

    // Should be at end of string
    if chars.peek().is_some() {
        return None;
    }

    Some((sign, number, unit))
}

/// Parse a size string for includegraphics options
/// Equivalent to the `sizeData` function in the original KaTeX JS code
fn size_data(str_val: &str) -> Result<MeasurementOwned, ParseError> {
    // Check if it's just a number with no unit specified
    if is_plain_number(str_val) {
        // Default unit is bp, per graphix package
        let number = str_val.trim().parse::<f64>().map_err(|_| {
            ParseError::new(ParseErrorKind::InvalidIncludeGraphicsNumber {
                value: str_val.to_owned(),
            })
        })?;
        return Ok(MeasurementOwned {
            number,
            unit: "bp".to_owned(),
        });
    }

    // Try to match pattern with optional sign, number, and unit
    if let Some((sign, number_str, unit_str)) = parse_size_with_unit(str_val) {
        // Combine sign and magnitude, then convert to number
        let number_with_sign = format!("{sign}{number_str}");
        let number = number_with_sign.parse::<f64>().map_err(|_| {
            ParseError::new(ParseErrorKind::InvalidIncludeGraphicsNumber {
                value: str_val.to_owned(),
            })
        })?;

        let measurement = MeasurementOwned {
            number,
            unit: unit_str,
        };

        // Validate the unit
        if !valid_unit(&measurement) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidIncludeGraphicsUnit {
                    unit: measurement.unit,
                },
            ));
        }

        Ok(measurement)
    } else {
        Err(ParseError::new(
            ParseErrorKind::InvalidIncludeGraphicsSize {
                size: str_val.to_owned(),
            },
        ))
    }
}

/// Convert textual input of an unsupported command into a color node containing
/// a text node Based on the format_unsupported_cmd function in parser/mod.rs
fn format_unsupported_cmd(text: &str, mode: Mode, error_color: &str) -> ParseNode {
    let mut textord_array: Vec<AnyParseNode> = Vec::with_capacity(text.chars().count());
    for ch in text.chars() {
        textord_array.push(AnyParseNode::TextOrd(ParseNodeTextOrd {
            mode: Mode::Text,
            loc: None,
            text: ch.to_string(),
        }));
    }
    let text_node = AnyParseNode::Text(ParseNodeText {
        mode,
        loc: None,
        body: textord_array,
        font: None,
    });
    ParseNode::Color(ParseNodeColor {
        mode,
        loc: None,
        color: error_color.to_owned(),
        body: vec![text_node],
    })
}

/// Register the \includegraphics function in the KaTeX context.
pub fn define_includegraphics(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Includegraphics),
        names: &["\\includegraphics"],
        props: FunctionPropSpec {
            num_args: 1,
            num_optional_args: 1,
            arg_types: Some(vec![ArgType::Raw, ArgType::Url]),
            allowed_in_text: false,
            ..Default::default()
        },
        handler: Some(
            |context: FunctionContext, args: Vec<ParseNode>, opt_args: Vec<Option<ParseNode>>| {
                // Default values
                let mut width = MeasurementOwned {
                    number: 0.0,
                    unit: "em".to_owned(),
                };
                let mut height = MeasurementOwned {
                    number: 0.9, // sorta character sized
                    unit: "em".to_owned(),
                };
                let mut total_height = MeasurementOwned {
                    number: 0.0,
                    unit: "em".to_owned(),
                };
                let mut alt = String::new();

                // Parse optional arguments
                if let Some(Some(opt_arg)) = opt_args.first()
                    && let ParseNode::Raw(raw_node) = opt_arg
                {
                    // Parser.js does not parse key/value pairs. We get a string.
                    let attributes: Vec<&str> = raw_node.string.split(',').collect();

                    for attribute in attributes {
                        let key_val: Vec<&str> = attribute.split('=').collect();
                        if key_val.len() == 2 {
                            let key = key_val[0].trim();
                            let value = key_val[1].trim();

                            match key {
                                "alt" => {
                                    value.clone_into(&mut alt);
                                }
                                "width" => {
                                    width = size_data(value)?;
                                }
                                "height" => {
                                    height = size_data(value)?;
                                }
                                "totalheight" => {
                                    total_height = size_data(value)?;
                                }
                                _ => {
                                    return Err(ParseError::new(
                                        ParseErrorKind::InvalidIncludeGraphicsKey {
                                            key: key.to_owned(),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }

                // Get the source URL
                let src = if let ParseNode::Url(url_node) = &args[0] {
                    url_node.url.clone()
                } else {
                    return Err(ParseError::new(
                        "Expected URL argument for \\includegraphics",
                    ));
                };

                // Generate alt text if not provided
                if alt.is_empty() {
                    // No alt given. Use the file name. Strip away the path.
                    alt.clone_from(&src);
                    // Remove path separators (both Unix and Windows style)
                    if let Some(last_slash) = alt.rfind(['/', '\\']) {
                        alt = alt[last_slash + 1..].to_string();
                    }
                    // Remove extension
                    if let Some(last_dot) = alt.rfind('.') {
                        alt = alt[..last_dot].to_string();
                    }
                }

                let mut trust_ctx = TrustContext {
                    command: "\\includegraphics".to_owned(),
                    url: Some(src.clone()),
                    protocol: None,
                    class: None,
                    id: None,
                    style: None,
                    attributes: None,
                };

                // Check if the command is trusted
                if !context.parser.settings.is_trusted(&mut trust_ctx) {
                    return Ok(format_unsupported_cmd(
                        "\\includegraphics",
                        context.parser.mode,
                        &context.parser.settings.error_color,
                    ));
                }

                Ok(ParseNode::Includegraphics(ParseNodeIncludegraphics {
                    mode: context.parser.mode,
                    loc: context.loc(),
                    alt,
                    width,
                    height,
                    total_height,
                    src,
                }))
            },
        ),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for the \includegraphics function
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    if let ParseNode::Includegraphics(includegraphics_node) = node {
        let height = ctx.calculate_size(&includegraphics_node.height, options)?;

        let depth = if includegraphics_node.total_height.number > 0.0 {
            ctx.calculate_size(&includegraphics_node.total_height, options)? - height
        } else {
            0.0
        };

        let width = if includegraphics_node.width.number > 0.0 {
            ctx.calculate_size(&includegraphics_node.width, options)?
        } else {
            0.0
        };

        let mut style = CssStyle::default();
        style.insert(CssProperty::Height, make_em(height + depth));
        if width > 0.0 {
            style.insert(CssProperty::Width, make_em(width));
        }
        if depth > 0.0 {
            style.insert(CssProperty::VerticalAlign, make_em(-depth));
        }

        let img = Img::new(
            includegraphics_node.src.clone(),
            includegraphics_node.alt.clone(),
            height,
            depth,
            0.0, // max_font_size
            style,
        );

        // The original JS code sets node.height and node.depth directly
        // In our Rust implementation, these are already set in the constructor

        Ok(HtmlDomNode::Img(img))
    } else {
        Err(ParseError::new("Expected Includegraphics node"))
    }
}

/// MathML builder for the \includegraphics function
fn mathml_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    if let ParseNode::Includegraphics(includegraphics_node) = node {
        let mut math_node = MathNode::builder().node_type(MathNodeType::Mglyph).build();
        math_node.set_attribute("alt", includegraphics_node.alt.clone());

        let height = ctx.calculate_size(&includegraphics_node.height, options)?;
        let mut depth = 0.0;
        if includegraphics_node.total_height.number > 0.0 {
            depth = ctx.calculate_size(&includegraphics_node.total_height, options)? - height;
            math_node.set_attribute("valign", make_em(-depth));
        }
        math_node.set_attribute("height", make_em(height + depth));

        if includegraphics_node.width.number > 0.0 {
            let width = ctx.calculate_size(&includegraphics_node.width, options)?;
            math_node.set_attribute("width", make_em(width));
        }
        math_node.set_attribute("src", includegraphics_node.src.clone());

        Ok(MathDomNode::Math(math_node))
    } else {
        Err(ParseError::new("Expected Includegraphics node"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_data_plain_number() {
        let result = size_data("10").unwrap();
        assert_eq!(result.number, 10.0);
        assert_eq!(result.unit, "bp");
    }

    #[test]
    fn test_size_data_decimal_number() {
        let result = size_data("10.5").unwrap();
        assert_eq!(result.number, 10.5);
        assert_eq!(result.unit, "bp");
    }

    #[test]
    fn test_size_data_negative_number() {
        let result = size_data("-5.2").unwrap();
        assert_eq!(result.number, -5.2);
        assert_eq!(result.unit, "bp");
    }

    #[test]
    fn test_size_data_with_unit() {
        let result = size_data("2.5em").unwrap();
        assert_eq!(result.number, 2.5);
        assert_eq!(result.unit, "em");
    }

    #[test]
    fn test_size_data_with_spaces() {
        let result = size_data(" 3.0 pt ").unwrap();
        assert_eq!(result.number, 3.0);
        assert_eq!(result.unit, "pt");
    }

    #[test]
    fn test_size_data_with_sign_and_spaces() {
        let result = size_data("+ 1.5 cm").unwrap();
        assert_eq!(result.number, 1.5);
        assert_eq!(result.unit, "cm");
    }

    #[test]
    fn test_size_data_invalid_unit() {
        let result = size_data("10xx");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid unit"));
    }

    #[test]
    fn test_size_data_invalid_format() {
        let result = size_data("abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid size"));
    }
}
