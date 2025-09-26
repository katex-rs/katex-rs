//! Implementation of the \verb command in KaTeX
//!
//! This module implements the LaTeX `\verb` command, which creates verbatim
//! text that preserves exact formatting and spacing.

use crate::build_common::{make_span, make_symbol, try_combine_chars};
use crate::context::KatexContext;
use crate::define_function::{FunctionContext, FunctionDefSpec, FunctionPropSpec};
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use crate::options::Options;
use crate::parser::parse_node::{NodeType, ParseNode, ParseNodeVerb};
use crate::style::TEXT;
use crate::types::ParseError;

/// Register the \verb function in the KaTeX context.
pub fn define_verb(ctx: &mut KatexContext) {
    ctx.define_function(FunctionDefSpec {
        node_type: Some(NodeType::Verb),
        names: &["\\verb"],
        props: FunctionPropSpec {
            num_args: 0,
            allowed_in_text: true,
            ..Default::default()
        },
        handler: Some(
            |_context: FunctionContext,
             _args: Vec<ParseNode>,
             _opt_args: Vec<Option<ParseNode>>| {
                // \verb and \verb* are dealt with directly in Parser.js.
                // If we end up here, it's because of a failure to match the two delimiters
                // in the regex in Lexer.js.  LaTeX raises the following error when \verb is
                // terminated by end of line (or file).
                Err(ParseError::new(
                    "\\verb ended by end of line instead of matching delimiter",
                ))
            },
        ),
        html_builder: Some(html_builder),
        mathml_builder: Some(mathml_builder),
    });
}

/// HTML builder for the \verb function
fn html_builder(
    node: &ParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    if let ParseNode::Verb(verb_node) = node {
        let text = make_verb(verb_node);
        let mut body: Vec<HtmlDomNode> = Vec::new();

        // \verb enters text mode and therefore is sized like \textstyle
        let new_options = options.having_style(TEXT);

        for ch in text.chars() {
            let c = if ch == '~' {
                "\\textasciitilde".to_owned()
            } else {
                ch.to_string()
            };

            let symbol = make_symbol(
                ctx,
                &c,
                "Typewriter-Regular",
                verb_node.mode,
                Some(&new_options),
                Some(&["mord".to_owned(), "texttt".to_owned()]),
            )?;
            body.push(symbol.into());
        }

        try_combine_chars(&mut body);

        let mut classes = vec!["mord".to_owned(), "text".to_owned()];
        classes.extend(new_options.sizing_classes(options));

        let span_struct = make_span(classes, body, Some(&new_options), None);
        Ok(HtmlDomNode::DomSpan(span_struct))
    } else {
        Err(ParseError::new("Expected Verb node"))
    }
}

/// MathML builder for the \verb function
fn mathml_builder(
    node: &ParseNode,
    _options: &Options,
    _ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    if let ParseNode::Verb(verb_node) = node {
        let text = make_verb(verb_node);
        let text_node = TextNode { text };

        let mut mtext = MathNode::builder()
            .node_type(MathNodeType::Mtext)
            .children(vec![MathDomNode::Text(text_node)])
            .build();

        mtext
            .attributes
            .insert("mathvariant".to_owned(), "monospace".to_owned());

        Ok(MathDomNode::Math(mtext))
    } else {
        Err(ParseError::new("Expected Verb node"))
    }
}

/// Converts verb group into body string.
///
/// \verb* replaces each space with an open box \u2423
/// \verb replaces each space with a no-break space \xA0
fn make_verb(group: &ParseNodeVerb) -> String {
    group
        .body
        .replace(' ', if group.star { "\u{2423}" } else { "\u{A0}" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Mode;

    #[test]
    fn test_make_verb_no_star() {
        let verb_node = ParseNodeVerb {
            mode: Mode::Text,
            loc: None,
            body: "hello world".to_owned(),
            star: false,
        };
        let result = make_verb(&verb_node);
        assert_eq!(result, "hello\u{A0}world");
    }

    #[test]
    fn test_make_verb_with_star() {
        let verb_node = ParseNodeVerb {
            mode: Mode::Text,
            loc: None,
            body: "hello world".to_owned(),
            star: true,
        };
        let result = make_verb(&verb_node);
        assert_eq!(result, "hello\u{2423}world");
    }

    #[test]
    fn test_make_verb_no_spaces() {
        let verb_node = ParseNodeVerb {
            mode: Mode::Text,
            loc: None,
            body: "helloworld".to_owned(),
            star: false,
        };
        let result = make_verb(&verb_node);
        assert_eq!(result, "helloworld");
    }
}
