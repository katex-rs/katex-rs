//! Symbol operator function implementations for KaTeX Rust
//!
//! This module handles atom symbols, migrated from KaTeX's symbolsOp.js.

use crate::ParseError;
use crate::build_common::mathsym;
use crate::build_mathml::{get_variant, make_text};
use crate::context::KatexContext;
use crate::dom_tree::HtmlDomNode;
use crate::mathml_tree::{MathDomNode, MathNode, MathNodeType};
use crate::options::Options;
use crate::parser::parse_node::{AnyParseNode, NodeType};
use crate::symbols::Atom;

/// Registers atom functions in the KaTeX context
pub fn define_symbols_op(ctx: &mut KatexContext) {
    // Register atom
    ctx.define_function_builders(
        NodeType::Atom,
        Some(atom_html_builder),
        Some(atom_mathml_builder),
    );
}

/// HTML builder for atom nodes
fn atom_html_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<HtmlDomNode, ParseError> {
    // Extract text, mode, and family from the node
    let (text, mode, family) = match node {
        AnyParseNode::Atom(atom) => (&atom.text, atom.mode, atom.family),
        _ => return Err(ParseError::new("Expected Atom node")),
    };

    // Create class name: "m" + family (e.g., "mbin", "mrel", etc.)
    let class_name = format!("m{}", family.as_ref());
    let classes = vec![class_name];

    Ok(mathsym(ctx, text, mode, options, Some(&classes))?.into())
}

/// MathML builder for atom nodes
fn atom_mathml_builder(
    node: &AnyParseNode,
    options: &Options,
    ctx: &KatexContext,
) -> Result<MathDomNode, ParseError> {
    // Extract text, mode, and family from the node
    let (text, mode, family) = match node {
        AnyParseNode::Atom(atom) => (&atom.text, atom.mode, atom.family),
        _ => return Err(ParseError::new("Expected Atom node")),
    };

    // Create the text node
    let text_node = make_text(text, mode, Some(options), &ctx.symbols);

    // Create mo element
    let mut mo_node = MathNode::builder()
        .node_type(MathNodeType::Mo)
        .children(vec![MathDomNode::Text(text_node)])
        .build();

    // Set attributes based on family
    match family {
        Atom::Bin => {
            // For bin atoms, set mathvariant if it's bold-italic
            if let Some(variant) = get_variant(ctx, node, options)?
                && variant == "bold-italic"
            {
                mo_node.set_attribute("mathvariant", variant);
            }
        }
        Atom::Punct => {
            // For punctuation, set separator
            mo_node.set_attribute("separator", "true");
        }
        Atom::Open | Atom::Close => {
            // For open/close delimiters, set stretchy to false
            mo_node.set_attribute("stretchy", "false");
        }
        _ => {
            // For other families (rel, etc.), no special attributes needed
        }
    }

    Ok(MathDomNode::Math(mo_node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Options;
    use crate::parser::parse_node::ParseNodeAtom;
    use crate::style;
    use crate::symbols::Atom;
    use crate::types::Mode;

    fn create_test_options() -> Options {
        Options::builder()
            .style(style::TEXT)
            .phantom(false)
            .max_size(1_000_000.0)
            .min_rule_thickness(0.04)
            .build()
    }

    fn create_test_context() -> KatexContext {
        let mut ctx = KatexContext::default();
        define_symbols_op(&mut ctx);
        ctx
    }

    #[test]
    fn test_atom_html_builder_bin() {
        let ctx = create_test_context();
        let options = create_test_options();

        let node = AnyParseNode::Atom(ParseNodeAtom {
            mode: Mode::Math,
            loc: None,
            text: "+".to_owned(),
            family: Atom::Bin,
        });

        let result = atom_html_builder(&node, &options, &ctx);
        assert!(result.is_ok());
        // The result should be an HtmlDomNode (Symbol node from mathsym)
        if let Ok(HtmlDomNode::Symbol(_)) = result {
            // Success
        } else {
            panic!("Expected Symbol node");
        }
    }

    #[test]
    fn test_atom_mathml_builder_bin() {
        let ctx = create_test_context();
        let options = create_test_options();

        let node = AnyParseNode::Atom(ParseNodeAtom {
            mode: Mode::Math,
            loc: None,
            text: "+".to_owned(),
            family: Atom::Bin,
        });

        let result = atom_mathml_builder(&node, &options, &ctx);
        assert!(result.is_ok());

        if let Ok(MathDomNode::Math(math_node)) = result {
            assert_eq!(math_node.node_type, MathNodeType::Mo);
            assert_eq!(math_node.children.len(), 1);
        } else {
            panic!("Expected Math node");
        }
    }

    #[test]
    fn test_atom_mathml_builder_punct() {
        let ctx = create_test_context();
        let options = create_test_options();

        let node = AnyParseNode::Atom(ParseNodeAtom {
            mode: Mode::Math,
            loc: None,
            text: ",".to_owned(),
            family: Atom::Punct,
        });

        let result = atom_mathml_builder(&node, &options, &ctx);
        assert!(result.is_ok());

        if let Ok(MathDomNode::Math(math_node)) = result {
            assert_eq!(math_node.node_type, MathNodeType::Mo);
            assert_eq!(math_node.children.len(), 1);
            // Should have separator attribute
            assert_eq!(
                math_node.attributes.get("separator"),
                Some(&"true".to_owned())
            );
        } else {
            panic!("Expected Math node");
        }
    }

    #[test]
    fn test_atom_mathml_builder_open() {
        let ctx = create_test_context();
        let options = create_test_options();

        let node = AnyParseNode::Atom(ParseNodeAtom {
            mode: Mode::Math,
            loc: None,
            text: "(".to_owned(),
            family: Atom::Open,
        });

        let result = atom_mathml_builder(&node, &options, &ctx);
        assert!(result.is_ok());

        if let Ok(MathDomNode::Math(math_node)) = result {
            assert_eq!(math_node.node_type, MathNodeType::Mo);
            assert_eq!(math_node.children.len(), 1);
            // Should have stretchy attribute set to false
            assert_eq!(
                math_node.attributes.get("stretchy"),
                Some(&"false".to_owned())
            );
        } else {
            panic!("Expected Math node");
        }
    }

    #[test]
    fn test_atom_mathml_builder_close() {
        let ctx = create_test_context();
        let options = create_test_options();

        let node = AnyParseNode::Atom(ParseNodeAtom {
            mode: Mode::Math,
            loc: None,
            text: ")".to_owned(),
            family: Atom::Close,
        });

        let result = atom_mathml_builder(&node, &options, &ctx);
        assert!(result.is_ok());

        if let Ok(MathDomNode::Math(math_node)) = result {
            assert_eq!(math_node.node_type, MathNodeType::Mo);
            assert_eq!(math_node.children.len(), 1);
            // Should have stretchy attribute set to false
            assert_eq!(
                math_node.attributes.get("stretchy"),
                Some(&"false".to_owned())
            );
        } else {
            panic!("Expected Math node");
        }
    }

    #[test]
    fn test_atom_mathml_builder_rel() {
        let ctx = create_test_context();
        let options = create_test_options();

        let node = AnyParseNode::Atom(ParseNodeAtom {
            mode: Mode::Math,
            loc: None,
            text: "=".to_owned(),
            family: Atom::Rel,
        });

        let result = atom_mathml_builder(&node, &options, &ctx);
        assert!(result.is_ok());

        if let Ok(MathDomNode::Math(math_node)) = result {
            assert_eq!(math_node.node_type, MathNodeType::Mo);
            assert_eq!(math_node.children.len(), 1);
            // Should not have special attributes for rel family
            assert!(!math_node.attributes.contains_key("separator"));
            assert!(!math_node.attributes.contains_key("stretchy"));
        } else {
            panic!("Expected Math node");
        }
    }
}
