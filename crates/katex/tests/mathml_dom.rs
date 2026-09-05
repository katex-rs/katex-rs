#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use katex::mathml_tree::{MathDomNode, MathNode, MathNodeType, TextNode};
use katex::tree::VirtualNode;
use katex::web_context::WebContext;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn consecutive_text_nodes_are_merged_across_the_entire_run() {
    let ctx = WebContext::from_window().unwrap();
    let text = |value: &str| {
        MathDomNode::Text(TextNode {
            text: value.to_owned(),
        })
    };
    for count in 1..=5 {
        let node = MathNode::builder()
            .node_type(MathNodeType::Mn)
            .children((0..count).map(|_| text("8")).collect())
            .build()
            .to_node(&ctx);
        let child = node.first_child().unwrap();
        assert_eq!(child.node_type(), web_sys::Node::TEXT_NODE);
        assert_eq!(child.node_value().unwrap(), "8".repeat(count));
        assert!(child.next_sibling().is_none());
    }

    // An element terminates the run; text on either side must stay separate.
    let node = MathNode::builder()
        .node_type(MathNodeType::Mrow)
        .children(vec![
            text("a"),
            text("b"),
            text("c"),
            MathNode::builder()
                .node_type(MathNodeType::Mspace)
                .build()
                .into(),
            text("d"),
            text("e"),
            text("f"),
        ])
        .build()
        .to_node(&ctx);
    let first = node.first_child().unwrap();
    assert_eq!(first.node_value().unwrap(), "abc");
    let separator = first.next_sibling().unwrap();
    assert_eq!(separator.node_type(), web_sys::Node::ELEMENT_NODE);
    let last = separator.next_sibling().unwrap();
    assert_eq!(last.node_value().unwrap(), "def");
    assert!(last.next_sibling().is_none());
}
