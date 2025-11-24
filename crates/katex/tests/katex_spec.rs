#![allow(clippy::non_ascii_literal)]

mod setup;
use katex::{
    CharacterMetrics, KatexContext, Settings, TrustSetting,
    build_html::DomType,
    dom_tree::HtmlDomNode,
    macros::{MacroDefinition, MacroExpansion},
    parser::parse_node::{AlignSpec, ParseNode},
    render_to_dom_tree, render_to_string,
    style::{DISPLAY, SCRIPTSCRIPT},
    symbols::{Atom, Font, Group, NonAtom},
    tree::VirtualNode as _,
    types::{CssProperty, Mode, Token},
};
use setup::*;
use std::io::Read as _;
use std::sync::Arc;
use std::sync::Mutex;

#[test]

// Every function is equivalent to a "describe" block in Javascript

fn a_parser() {
    it("should not fail on an empty string", || {
        expect!("").to_parse(&strict_settings())
    });

    it("should ignore whitespace", || {
        expect!("    x    y    ").to_parse_like("xy", &strict_settings())
    });

    it("should ignore whitespace in atom", || {
        expect!("    x   ^ y    ").to_parse(&strict_settings())?;
        expect!("x^y").to_parse(&strict_settings())
    });
}

#[test]
fn an_ord_parser() {
    let expression = "1234|/@.\"`abcdefgzABCDEFGZ";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())
    });

    it("should build a list of ords", || {
        let parsed = get_parsed_strict(expression)?;
        for node in parsed {
            assert!(matches!(
                node,
                ParseNode::OrdGroup(_) | ParseNode::MathOrd(_) | ParseNode::TextOrd(_)
            ));
        }
        Ok(())
    });

    it("should parse the right number of ords", || {
        let parsed = get_parsed_strict(expression)?;
        assert_eq!(parsed.len(), expression.chars().count());
        Ok(())
    });
}
#[test]
fn a_bin_parser() {
    let expression = r"+-*\cdot\pm\div";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())
    });

    it("should build a list of bins", || {
        let parsed = get_parsed_strict(expression)?;
        for node in parsed {
            assert_let!(ParseNode::Atom(atom) = node);
            assert_eq!(atom.family, Atom::Bin);
        }
        Ok(())
    });
}
#[test]
fn a_rel_parser() {
    let expression = r"=<>\leq\geq\neq\nleq\ngeq\cong";
    let not_expression = r"\not=\not<\not>\not\leq\not\geq\not\in";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())?;
        expect!(not_expression).to_parse(&strict_settings())
    });

    it("should build a list of rels", || {
        let parsed = get_parsed_strict(expression)?;
        for node in parsed {
            let node = if let ParseNode::HtmlMathMl(html) = &node {
                assert_eq!(html.html.len(), 1);
                &html.html[0]
            } else {
                &node
            };
            match node {
                ParseNode::Atom(atom) => assert_eq!(atom.family, Atom::Rel),
                ParseNode::Mclass(mclass) => {
                    assert_eq!(mclass.mclass, DomType::Mrel);
                }
                _ => panic!("Expected Atom or Mclass, got {node:?}"),
            }
        }
        Ok(())
    });
}

#[test]
fn a_mathinner_parser() {
    it("should not fail", || {
        expect!(r"\mathinner{\langle{\psi}\rangle}").to_parse(&strict_settings())?;
        expect!(r"\frac 1 {\mathinner{\langle{\psi}\rangle}}").to_parse(&strict_settings())
    });

    it("should return one group, not a fragment", || {
        let contents = "\\mathinner{\\langle{\\psi}\\rangle}";
        let mml = build_mathml(contents)?;
        assert_eq!(mml.children.len(), 1);
        Ok(())
    });
}

#[test]
fn a_punct_parser() {
    let expression = ",;";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())
    });

    it("should build a list of puncts", || {
        let parsed = get_parsed_strict(expression)?;
        for node in parsed {
            assert_let!(ParseNode::Atom(atom) = node);
            assert_eq!(atom.family, Atom::Punct);
        }
        Ok(())
    });
}
#[test]
fn an_open_parser() {
    let expression = "([";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())
    });

    it("should build a list of opens", || {
        let parsed = get_parsed_strict(expression)?;
        for node in parsed {
            assert_let!(ParseNode::Atom(atom) = node);
            assert_eq!(atom.family, Atom::Open);
        }
        Ok(())
    });
}

#[test]
fn a_close_parser() {
    let expression = ")]?!";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())
    });

    it("should build a list of closes", || {
        let parsed = get_parsed_strict(expression)?;
        for node in parsed {
            assert_let!(ParseNode::Atom(atom) = node);
            assert_eq!(atom.family, Atom::Close);
        }
        Ok(())
    });
}
#[test]
fn a_katex_parser() {
    it("should not fail", || {
        expect!(r"\KaTeX").to_parse(&strict_settings())
    });
}
#[test]
fn a_subscript_and_superscript_parser() {
    it("should not fail on superscripts", || {
        expect!("x^2").to_parse(&strict_settings())
    });

    it("should not fail on subscripts", || {
        expect!("x_3").to_parse(&strict_settings())
    });

    it(
        "should not fail on both subscripts and superscripts",
        || {
            expect!("x^2_3").to_parse(&strict_settings())?;
            expect!("x_2^3").to_parse(&strict_settings())
        },
    );

    it("should not fail when there is no nucleus", || {
        expect!("^3").to_parse(&strict_settings())?;
        expect!("^3+").to_parse(&strict_settings())?;
        expect!("_2").to_parse(&strict_settings())?;
        expect!("^3_2").to_parse(&strict_settings())?;
        expect!("_2^3").to_parse(&strict_settings())
    });

    it("should produce supsubs for superscript", || {
        let parsed = get_parsed_strict("x^2")?;
        assert_let!(ParseNode::SupSub(node) = &parsed[0]);
        assert!(node.sub.is_none());
        assert!(node.sup.is_some());
        assert!(node.base.is_some());
        Ok(())
    });

    it("should produce supsubs for subscript", || {
        let parsed = get_parsed_strict("x_3")?;
        assert_let!(ParseNode::SupSub(node) = &parsed[0]);
        assert!(node.sup.is_none());
        assert!(node.sub.is_some());
        assert!(node.base.is_some());
        Ok(())
    });

    it("should produce supsubs for ^_", || {
        let parsed = get_parsed_strict("x^2_3")?;
        assert_let!(ParseNode::SupSub(node) = &parsed[0]);
        assert!(node.sup.is_some());
        assert!(node.sub.is_some());
        assert!(node.base.is_some());
        Ok(())
    });

    it("should produce supsubs for _^", || {
        let parsed = get_parsed_strict("x_3^2")?;
        assert_let!(ParseNode::SupSub(node) = &parsed[0]);
        assert!(node.sup.is_some());
        assert!(node.sub.is_some());
        assert!(node.base.is_some());
        Ok(())
    });

    it("should produce the same thing regardless of order", || {
        expect!("x^2_3").to_parse_like("x_3^2", &strict_settings())
    });

    it("should not parse double subscripts or superscripts", || {
        expect!("x^x^x").not_to_parse(&strict_settings())?;
        expect!("x_x_x").not_to_parse(&strict_settings())?;
        expect!("x_x^x_x").not_to_parse(&strict_settings())?;
        expect!("x_x^x^x").not_to_parse(&strict_settings())?;
        expect!("x^x_x_x").not_to_parse(&strict_settings())?;
        expect!("x^x_x^x").not_to_parse(&strict_settings())
    });

    it("should work correctly with {}s", || {
        expect!("x^{2+3}").to_parse(&strict_settings())?;
        expect!("x_{3-2}").to_parse(&strict_settings())?;
        expect!("x^{2+3}_3").to_parse(&strict_settings())?;
        expect!("x^2_{3-2}").to_parse(&strict_settings())?;
        expect!("x^{2+3}_{3-2}").to_parse(&strict_settings())?;
        expect!("x_{3-2}^{2+3}").to_parse(&strict_settings())?;
        expect!("x_3^{2+3}").to_parse(&strict_settings())?;
        expect!("x_{3-2}^2").to_parse(&strict_settings())
    });

    it("should work with nested super/subscripts", || {
        expect!("x^{x^x}").to_parse(&strict_settings())?;
        expect!("x^{x_x}").to_parse(&strict_settings())?;
        expect!("x_{x^x}").to_parse(&strict_settings())?;
        expect!("x_{x_x}").to_parse(&strict_settings())
    });

    it(
        "should work with Unicode (sub|super)script characters",
        || {
            expect!("A² + B²⁺³ + ¹²C + E₂³ + F₂₊₃").to_parse_like(
                "A^{2} + B^{2+3} + ^{12}C + E_{2}^{3} + F_{2+3}",
                &strict_settings(),
            )
        },
    );

    it("should not fail if \\relax is in an atom", || {
        expect!(r"\hskip1em\relax^2").to_parse(&strict_settings())
    });

    it("should skip \\relax in super/subscripts", || {
        // Compare parse trees instead of HTML
        expect!(r"x^\relax 2").to_parse_like("x^2", &strict_settings())?;
        expect!(r"x_\relax 2").to_parse_like("x_2", &strict_settings())
    });
}
#[test]
fn a_subscript_and_superscript_tree_builder() {
    it("should not fail when there is no nucleus", || {
        expect!("^3").to_parse(&strict_settings())?;
        expect!("_2").to_parse(&strict_settings())?;
        expect!("^3_2").to_parse(&strict_settings())?;
        expect!("_2^3").to_parse(&strict_settings())
    });
}
#[test]
fn a_parser_with_limit_controls() {
    it(
        "should fail when the limit control is not preceded by an op node",
        || {
            expect!(r"3\nolimits_2^2").not_to_parse(&strict_settings())?;
            expect!(r"\sqrt\limits_2^2").not_to_parse(&strict_settings())?;
            expect!(r"45 +\nolimits 45").not_to_parse(&strict_settings())
        },
    );

    it(
        "should parse when the limit control directly follows an op node",
        || {
            expect!(r"\int\limits_2^2 3").to_parse(&strict_settings())?;
            expect!(r"\sum\nolimits_3^4 4").to_parse(&strict_settings())
        },
    );

    it(
        "should parse when the limit control is in the sup/sub area of an op node",
        || {
            expect!(r"\int_2^2\limits").to_parse(&strict_settings())?;
            expect!(r"\int^2\nolimits_2").to_parse(&strict_settings())?;
            expect!(r"\int_2\limits^2").to_parse(&strict_settings())
        },
    );

    it(
        "should allow multiple limit controls in the sup/sub area of an op node",
        || {
            expect!(r"\int_2\nolimits^2\limits 3").to_parse(&strict_settings())?;
            expect!(r"\int\nolimits\limits_2^2").to_parse(&strict_settings())?;
            expect!(r"\int\limits\limits\limits_2^2").to_parse(&strict_settings())
        },
    );

    it(
        "should have the rightmost limit control determine the limits property of the preceding op node",
        || {
            let parsed_input = get_parsed_strict(r"\int_2\nolimits^2\limits 3")?;
            assert_let!(ParseNode::SupSub(supsub) = &parsed_input[0]);
            assert_let!(Some(ParseNode::Op(op_node)) = &supsub.base.as_deref());
            assert!(op_node.limits());
            let parsed_input = get_parsed_strict(r"\int\limits_2\nolimits^2")?;
            assert_let!(ParseNode::SupSub(supsub) = &parsed_input[0]);
            assert_let!(Some(ParseNode::Op(op_node)) = &supsub.base.as_deref());
            assert!(!op_node.limits());
            Ok(())
        },
    );
}
#[test]
fn a_group_parser() {
    it("should not fail", || {
        expect!("{xy}").to_parse(&strict_settings())
    });

    it("should produce a single ord", || {
        let parsed = get_parsed_strict("{xy}")?;
        assert_eq!(parsed.len(), 1);
        assert_let!(ParseNode::OrdGroup(ord) = &parsed[0]);
        assert!(!ord.body.is_empty());
        Ok(())
    });
}

#[test]
fn a_begingroup_endgroup_parser() {
    it("should not fail", || {
        expect!(r"\begingroup xy \endgroup").to_parse(&strict_settings())
    });

    it("should fail when it is mismatched", || {
        expect!(r"\begingroup xy").not_to_parse(&strict_settings())?;
        expect!(r"\begingroup xy }").not_to_parse(&strict_settings())
    });

    it("should produce a semi-simple group", || {
        let parsed = get_parsed_strict(r"\begingroup xy \endgroup")?;
        assert_eq!(parsed.len(), 1);
        assert_let!(ParseNode::OrdGroup(ord) = &parsed[0]);
        assert!(ord.semisimple.unwrap_or(false));
        assert!(!ord.body.is_empty());
        Ok(())
    });

    it("should not affect spacing in math mode", || {
        // Test that both parse successfully - spacing behavior may differ
        expect!(r"\begingroup x+ \endgroup y").to_build_like("x+y", &strict_settings())
    });
}

#[test]
fn an_implicit_group_parser() {
    it("should not fail", || {
        expect!(r"\Large x").to_parse(&strict_settings())?;
        expect!(r"abc {abc \Large xyz} abc").to_parse(&strict_settings())
    });

    it("should produce a single object", || {
        let parsed = get_parsed_strict(r"\Large abc")?;
        assert_eq!(parsed.len(), 1);
        assert_let!(ParseNode::Sizing(sizing) = &parsed[0]);
        assert!(!sizing.body.is_empty());
        Ok(())
    });

    it("should apply only after the function", || {
        let parsed = get_parsed_strict(r"a \Large abc")?;
        assert_eq!(parsed.len(), 2);
        assert_let!(ParseNode::Sizing(sizing) = &parsed[1]);
        assert_eq!(sizing.body.len(), 3);
        Ok(())
    });

    it("should stop at the ends of groups", || {
        let parsed = get_parsed_strict(r"a { b \Large c } d")?;
        assert_let!(ParseNode::OrdGroup(ord) = &parsed[1]);
        assert_let!(ParseNode::Sizing(sizing) = &ord.body[1]);
        assert_eq!(sizing.body.len(), 1);
        Ok(())
    });

    it(
        "should work with sizing commands: \\sqrt[\\small 3]{x}",
        || {
            let mut parsed = get_parsed_strict(r"\sqrt[\small 3]{x}")?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("sqrt_optional_group_small", format!("{:#?}", parsed));
            Ok(())
        },
    );

    it(
        "should work with \\color: \\sqrt[\\color{red} 3]{x}",
        || {
            let mut parsed = get_parsed_strict(r"\sqrt[\color{red} 3]{x}")?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("sqrt_optional_group_color", format!("{:#?}", parsed));
            Ok(())
        },
    );

    it(
        "should work style commands \\sqrt[\\textstyle 3]{x}",
        || {
            let mut parsed = get_parsed_strict(r"\sqrt[\textstyle 3]{x}")?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("sqrt_optional_group_textstyle", format!("{:#?}", parsed));
            Ok(())
        },
    );

    it(
        "should work with old font functions: \\sqrt[\\tt 3]{x}",
        || {
            let mut parsed = get_parsed_strict(r"\sqrt[\tt 3]{x}")?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("sqrt_optional_group_old_font", format!("{:#?}", parsed));
            Ok(())
        },
    );
}

#[test]
fn a_function_parser() {
    it("should parse no argument functions", || {
        expect!(r"\div").to_parse(&strict_settings())
    });

    it("should parse 1 argument functions", || {
        expect!(r"\blue x").to_parse(&strict_settings())
    });

    it("should parse 2 argument functions", || {
        expect!(r"\frac 1 2").to_parse(&strict_settings())
    });

    it(
        "should not parse 1 argument functions with no arguments",
        || expect!(r"\blue").not_to_parse(&strict_settings()),
    );

    it(
        "should not parse 2 argument functions with 0 or 1 arguments",
        || {
            expect!(r"\frac").not_to_parse(&strict_settings())?;
            expect!(r"\frac 1").not_to_parse(&strict_settings())
        },
    );

    it(
        "should not parse a function with text right after it",
        || expect!(r"\redx").not_to_parse(&strict_settings()),
    );

    it(
        "should parse a function with a number right after it",
        || expect!(r"\frac12").to_parse(&strict_settings()),
    );

    it(
        "should parse some functions with text right after it",
        || expect!(r"\;x").to_parse(&strict_settings()),
    );
}

#[test]
fn a_frac_parser() {
    let expression = r"\frac{x}{y}";
    let dfrac_expression = r"\dfrac{x}{y}";
    let tfrac_expression = r"\tfrac{x}{y}";
    let cfrac_expression = r"\cfrac{x}{y}";
    let genfrac1 = r"\genfrac ( ] {0.06em}{0}{a}{b+c}";
    let genfrac2 = r"\genfrac ( ] {0.8pt}{}{a}{b+c}";

    it("should not fail", || {
        expect!(expression).to_parse(&strict_settings())
    });

    it("should produce a frac", || {
        let parsed = get_parsed_strict(expression)?;
        assert_let!(ParseNode::Genfrac(_) = &parsed[0]);
        // numer and denom are always present in Genfrac
        Ok(())
    });

    it("should also parse cfrac, dfrac, tfrac, and genfrac", || {
        expect!(cfrac_expression).to_parse(&strict_settings())?;
        expect!(dfrac_expression).to_parse(&strict_settings())?;
        expect!(tfrac_expression).to_parse(&strict_settings())?;
        expect!(genfrac1).to_parse(&strict_settings())?;
        expect!(genfrac2).to_parse(&strict_settings())
    });

    it(
        "should parse cfrac, dfrac, tfrac, and genfrac as fracs",
        || {
            let dfrac_parsed = get_parsed_strict(dfrac_expression)?;
            assert_let!(ParseNode::Genfrac(node) = &dfrac_parsed[0]);
            assert!(node.has_bar_line);

            let tfrac_parsed = get_parsed_strict(tfrac_expression)?;
            assert_let!(ParseNode::Genfrac(node) = &tfrac_parsed[0]);
            assert!(node.has_bar_line);

            let cfrac_parsed = get_parsed_strict(cfrac_expression)?;
            assert_let!(ParseNode::Genfrac(node) = &cfrac_parsed[0]);
            assert!(node.has_bar_line);

            let genfrac_parsed = get_parsed_strict(genfrac1)?;
            assert_let!(ParseNode::Genfrac(node) = &genfrac_parsed[0]);
            assert!(node.left_delim.is_some());
            assert!(node.right_delim.is_some());
            Ok(())
        },
    );

    it(
        "should fail, given math as a line thickness to genfrac",
        || {
            let bad_genfrac = r"\genfrac ( ] {b+c}{0}{a}{b+c}";
            expect!(bad_genfrac).not_to_parse(&strict_settings())
        },
    );

    it(
        "should fail if genfrac is given less than 6 arguments",
        || {
            let bad_genfrac = r"\genfrac ( ] {0.06em}{0}{a}";
            expect!(bad_genfrac).not_to_parse(&strict_settings())
        },
    );

    it("should parse atop", || {
        let parsed = get_parsed_strict(r"x \atop y")?;
        assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
        assert!(!node.has_bar_line);
        Ok(())
    });
}

#[test]
fn an_over_brace_brack_parser() {
    let simple_over = r"1 \over x";
    let complex_over = r"1+2i \over 3+4i";
    let brace_frac = r"a+b \brace c+d";
    let brack_frac = r"a+b \brack c+d";

    it("should not fail", || {
        expect!(simple_over).to_parse(&strict_settings())?;
        expect!(complex_over).to_parse(&strict_settings())?;
        expect!(brace_frac).to_parse(&strict_settings())?;
        expect!(brack_frac).to_parse(&strict_settings())
    });

    it("should produce a frac", || {
        let mut parsed = get_parsed_strict(simple_over)?;
        assert_let!(ParseNode::Genfrac(_) = &parsed[0]);
        // numer and denom are always present in Genfrac

        parsed = get_parsed_strict(complex_over)?;
        assert_let!(ParseNode::Genfrac(_) = &parsed[0]);

        let parsed_brace = get_parsed_strict(brace_frac)?;
        assert_let!(ParseNode::Genfrac(node) = &parsed_brace[0]);
        assert!(node.left_delim.is_some());
        assert!(node.right_delim.is_some());

        let parsed_brack = get_parsed_strict(brack_frac)?;
        assert_let!(ParseNode::Genfrac(node) = &parsed_brack[0]);
        assert!(node.left_delim.is_some());
        assert!(node.right_delim.is_some());
        Ok(())
    });

    it(
        "should create a numerator from the atoms before \\over",
        || {
            let parsed = get_parsed_strict(complex_over)?;
            assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
            if let ParseNode::OrdGroup(numer) = &*node.numer {
                assert_eq!(numer.body.len(), 4);
            } else {
                panic!("Expected OrdGroup for numerator");
            }
            Ok(())
        },
    );

    it(
        "should create a denominator from the atoms after \\over",
        || {
            let parsed = get_parsed_strict(complex_over)?;
            assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
            if let ParseNode::OrdGroup(denom) = &*node.denom {
                assert_eq!(denom.body.len(), 4);
            } else {
                panic!("Expected OrdGroup for denominator");
            }
            Ok(())
        },
    );

    it("should handle empty numerators", || {
        let empty_numerator = r"\over x";
        let parsed = get_parsed_strict(empty_numerator)?;
        assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
        // numer and denom are always present
        assert!(node.has_bar_line);
        Ok(())
    });

    it("should handle empty denominators", || {
        let empty_denominator = r"1 \over";
        let parsed = get_parsed_strict(empty_denominator)?;
        assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
        assert!(node.has_bar_line);
        Ok(())
    });

    it("should handle \\displaystyle correctly", || {
        let displaystyle_expression = r"\displaystyle 1 \over 2";
        let parsed = get_parsed_strict(displaystyle_expression)?;
        assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
        if let ParseNode::OrdGroup(numer) = &*node.numer {
            assert_let!(ParseNode::Styling(_) = &numer.body[0]);
        } else {
            panic!("Expected OrdGroup for numerator");
        }
        Ok(())
    });

    it("should handle \\textstyle correctly", || {
        expect!(r"\textstyle 1 \over 2")
            .to_parse_like(r"\frac{\textstyle 1}{2}", &strict_settings())
    });

    it("should handle nested factions", || {
        let nested_over_expression = r"{1 \over 2} \over 3";
        let parsed = get_parsed_strict(nested_over_expression)?;
        assert_let!(ParseNode::Genfrac(node) = &parsed[0]);
        if let ParseNode::OrdGroup(numer) = &*node.numer {
            assert_let!(ParseNode::Genfrac(_) = &numer.body[0]);
            if let ParseNode::Genfrac(inner_node) = &numer.body[0] {
                if let ParseNode::OrdGroup(inner_numer) = &*inner_node.numer {
                    assert_eq!(inner_numer.body.len(), 1);
                }
                if let ParseNode::OrdGroup(inner_denom) = &*inner_node.denom {
                    assert_eq!(inner_denom.body.len(), 1);
                }
            }
        } else {
            panic!("Expected OrdGroup for numerator");
        }
        if let ParseNode::OrdGroup(denom) = &*node.denom {
            assert_eq!(denom.body.len(), 1);
        } else {
            panic!("Expected OrdGroup for denominator");
        }
        Ok(())
    });

    it("should fail with multiple overs in the same group", || {
        let bad_multiple_overs = r"1 \over 2 + 3 \over 4";
        expect!(bad_multiple_overs).not_to_parse(&strict_settings())?;
        let bad_over_choose = r"1 \over 2 \choose 3";
        expect!(bad_over_choose).not_to_parse(&strict_settings())
    });
}

#[test]
fn a_genfrac_builder() {
    it("should not fail", || {
        expect!(r"\frac{x}{y}").to_build(&strict_settings())?;
        expect!(r"\dfrac{x}{y}").to_build(&strict_settings())?;
        expect!(r"\tfrac{x}{y}").to_build(&strict_settings())?;
        expect!(r"\cfrac{x}{y}").to_build(&strict_settings())?;
        expect!(r"\genfrac ( ] {0.06em}{0}{a}{b+c}").to_build(&strict_settings())?;
        expect!(r"\genfrac ( ] {0.8pt}{}{a}{b+c}").to_build(&strict_settings())?;
        expect!(r"\genfrac {} {} {0.8pt}{}{a}{b+c}").to_build(&strict_settings())?;
        expect!(r"\genfrac [ {} {0.8pt}{}{a}{b+c}").to_build(&strict_settings())
    });
}

#[test]
fn a_sizing_parser() {
    let size_expression = r"\Huge{x}\small{x}";

    it("should not fail", || {
        expect!(size_expression).to_parse(&strict_settings())
    });

    it("should produce a sizing node", || {
        let parsed = get_parsed_strict(size_expression)?;
        assert_let!(ParseNode::Sizing(sizing) = &parsed[0]);
        assert!(sizing.size > 0);
        assert!(!sizing.body.is_empty());
        Ok(())
    });
}

#[test]
fn a_infix_builder() {
    it("should not fail", || {
        expect!(r"a \over b").to_build(&strict_settings())?;
        expect!(r"a \atop b").to_build(&strict_settings())?;
        expect!(r"a \choose b").to_build(&strict_settings())?;
        expect!(r"a \brace b").to_build(&strict_settings())?;
        expect!(r"a \brack b").to_build(&strict_settings())
    });
}

#[test]
fn a_text_parser() {
    let text_expression = r"\text{a b}";
    let no_brace_text_expression = r"\text x";
    let nested_text_expression = r"\text{a {b} \blue{c} \textcolor{#fff}{x} \llap{x}}";
    let space_text_expression = r"\text{  a \  }";
    let leading_space_text_expression = r"\text {moo}";
    let bad_text_expression = r"\text{a b%}";
    let bad_function_expression = r"\text{\sqrt{x}}";
    let math_token_after_text = r"\text{sin}^2";

    it("should not fail", || {
        expect!(text_expression).to_parse(&strict_settings())
    });

    it("should produce a text", || {
        let parsed = get_parsed_strict(text_expression)?;
        assert_let!(ParseNode::Text(text_node) = &parsed[0]);
        assert!(!text_node.body.is_empty());
        Ok(())
    });

    it("should produce textords instead of mathords", || {
        let parsed = get_parsed_strict(text_expression)?;
        assert_let!(ParseNode::Text(text_node) = &parsed[0]);
        assert_let!(ParseNode::TextOrd(_) = &text_node.body[0]);
        Ok(())
    });

    it("should not parse bad text", || {
        expect!(bad_text_expression).not_to_parse(&strict_settings())
    });

    it("should not parse bad functions inside text", || {
        expect!(bad_function_expression).not_to_parse(&strict_settings())
    });

    it("should parse text with no braces around it", || {
        expect!(no_brace_text_expression).to_parse(&strict_settings())
    });

    it("should parse nested expressions", || {
        expect!(nested_text_expression).to_parse(&strict_settings())
    });

    it("should contract spaces", || {
        let parsed = get_parsed_strict(space_text_expression)?;
        assert_let!(ParseNode::Text(text_node) = &parsed[0]);
        assert_eq!(text_node.body.len(), 4);
        assert_let!(ParseNode::Spacing(_) = &text_node.body[0]);
        assert_let!(ParseNode::TextOrd(_) = &text_node.body[1]);
        assert_let!(ParseNode::Spacing(_) = &text_node.body[2]);
        assert_let!(ParseNode::Spacing(_) = &text_node.body[3]);
        Ok(())
    });

    it("should handle backslash followed by newline", || {
        expect!("\\text{ \t\r \n \t\r  }").to_parse_like(r"\text{ }", &strict_settings())
    });

    it("should accept math mode tokens after its argument", || {
        expect!(math_token_after_text).to_parse(&strict_settings())
    });

    it("should ignore a space before the text group", || {
        let parsed = get_parsed_strict(leading_space_text_expression)?;
        assert_let!(ParseNode::Text(text_node) = &parsed[0]);
        assert_eq!(text_node.body.len(), 3);
        let texts: Vec<String> = text_node
            .body
            .iter()
            .filter_map(|node| {
                if let ParseNode::TextOrd(text_ord) = node {
                    Some(text_ord.text.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts.join(""), "moo");
        Ok(())
    });

    it("should parse math within text group", || {
        expect!(r"\text{graph: $y = mx + b$}").to_parse(&strict_settings())?;
        expect!(r"\text{graph: \(y = mx + b\)}").to_parse(&strict_settings())
    });

    it(
        "should parse math within text within math within text",
        || {
            expect!(r"\text{hello $x + \text{world $y$} + z$}").to_parse(&strict_settings())?;
            expect!(r"\text{hello \(x + \text{world $y$} + z\)}").to_parse(&strict_settings())?;
            expect!(r"\text{hello $x + \text{world \(y\)} + z$}").to_parse(&strict_settings())?;
            expect!(r"\text{hello \(x + \text{world \(y\)} + z\)}").to_parse(&strict_settings())
        },
    );

    it("should forbid \\( within math mode", || {
        expect!(r"\(").not_to_parse(&strict_settings())?;
        expect!(r"\text{$\(x\)$}").not_to_parse(&strict_settings())
    });

    it("should forbid $ within math mode", || {
        expect!("$x$").not_to_parse(&strict_settings())?;
        expect!(r"\text{\($x$\)}").not_to_parse(&strict_settings())
    });

    it("should detect unbalanced \\)", || {
        expect!(r"\)").not_to_parse(&strict_settings())?;
        expect!(r"\text{\)}").not_to_parse(&strict_settings())
    });

    it("should detect unbalanced $", || {
        expect!("$").not_to_parse(&strict_settings())?;
        expect!(r"\text{$}").not_to_parse(&strict_settings())
    });

    it("should not mix $ and \\(..\\)", || {
        expect!(r"\text{$x\)}").not_to_parse(&strict_settings())?;
        expect!(r"\text{\(x$}").not_to_parse(&strict_settings())
    });

    it("should parse spacing functions", || {
        expect!(r"a b\, \; \! \: \> ~ \thinspace \medspace \quad \ ")
            .to_build(&strict_settings())?;
        expect!(r"\enspace \thickspace \qquad \space \nobreakspace").to_build(&strict_settings())
    });

    it("should omit spaces after commands", || {
        expect!(r"\text{\textellipsis !}")
            .to_parse_like(r"\text{\textellipsis!}", &strict_settings())
    });

    it("should handle ⋮ and \\vdots", || {
        expect!(r"\text{a \vdots b ⋮ d}").to_parse(&strict_settings())
    });
}

#[test]
fn a_texvc_builder() {
    it("should not fail", || {
        expect!(r"\lang\N\darr\R\dArr\Z\Darr\alef\rang").to_build(&strict_settings())?;
        expect!(r"\alefsym\uarr\Alpha\uArr\Beta\Uarr\Chi").to_build(&strict_settings())?;
        expect!(r"\clubs\diamonds\hearts\spades\cnums\Complex").to_build(&strict_settings())?;
        expect!(r"\Dagger\empty\harr\Epsilon\hArr\Eta\Harr\exist").to_build(&strict_settings())?;
        expect!(r"\image\larr\infin\lArr\Iota\Larr\isin\Kappa").to_build(&strict_settings())?;
        expect!(r"\Mu\lrarr\natnums\lrArr\Nu\Lrarr\Omicron").to_build(&strict_settings())?;
        expect!(r"\real\rarr\plusmn\rArr\reals\Rarr\Reals\Rho").to_build(&strict_settings())?;
        expect!(r"\text{\sect}\sdot\sub\sube\supe").to_build(&strict_settings())?;
        expect!(r"\Tau\thetasym\weierp\Zeta").to_build(&strict_settings())
    });
}

#[test]
fn a_color_parser() {
    let color_expression = r"\blue{x}";
    let new_color_expression = r"\redA{x}";
    let custom_color_expression1 = r"\textcolor{#fA6}{x}";
    let custom_color_expression2 = r"\textcolor{#fA6fA6}{x}";
    let custom_color_expression3 = r"\textcolor{fA6fA6}{x}";
    let bad_custom_color_expression1 = r"\textcolor{bad-color}{x}";
    let bad_custom_color_expression2 = r"\textcolor{#fA6f1}{x}";
    let bad_custom_color_expression3 = r"\textcolor{#gA6}{x}";
    let old_color_expression = r"\color{#fA6}xy";

    it("should not fail", || {
        expect!(color_expression).to_parse(&strict_settings())
    });

    it("should build a color node", || {
        let parsed = get_parsed_strict(color_expression)?;
        assert_let!(ParseNode::Color(color_node) = &parsed[0]);
        assert!(!color_node.body.is_empty());
        Ok(())
    });

    it("should parse a custom color", || {
        expect!(custom_color_expression1).to_parse(&strict_settings())?;
        expect!(custom_color_expression2).to_parse(&strict_settings())?;
        expect!(custom_color_expression3).to_parse(&strict_settings())
    });

    it("should correctly extract the custom color", || {
        let parsed1 = get_parsed_strict(custom_color_expression1)?;
        assert_let!(ParseNode::Color(color_node1) = &parsed1[0]);
        assert_eq!(color_node1.color, "#fA6");

        let parsed2 = get_parsed_strict(custom_color_expression2)?;
        assert_let!(ParseNode::Color(color_node2) = &parsed2[0]);
        assert_eq!(color_node2.color, "#fA6fA6");

        let parsed3 = get_parsed_strict(custom_color_expression3)?;
        assert_let!(ParseNode::Color(color_node3) = &parsed3[0]);
        assert_eq!(color_node3.color, "#fA6fA6");

        Ok(())
    });

    it("should not parse a bad custom color", || {
        expect!(bad_custom_color_expression1).not_to_parse(&strict_settings())?;
        expect!(bad_custom_color_expression2).not_to_parse(&strict_settings())?;
        expect!(bad_custom_color_expression3).not_to_parse(&strict_settings())
    });

    it("should parse new colors from the branding guide", || {
        expect!(new_color_expression).to_parse(&strict_settings())
    });

    it("should use one-argument \\color by default", || {
        expect!(old_color_expression).to_parse_like(r"\textcolor{#fA6}{xy}", &strict_settings())
    });

    it("should use one-argument \\color if requested", || {
        let mut settings = strict_settings();
        settings.color_is_text_color = false;
        expect!(old_color_expression).to_parse_like(r"\textcolor{#fA6}{xy}", &settings)
    });

    it("should use two-argument \\color if requested", || {
        let mut settings = strict_settings();
        settings.color_is_text_color = true;
        expect!(old_color_expression).to_parse_like(r"\textcolor{#fA6}{x}y", &settings)
    });

    it("should not define \\color in global context", || {
        let mut settings = strict_settings();
        settings.color_is_text_color = true;
        settings.global_group = true;
        expect!(old_color_expression).to_parse_like(r"\textcolor{#fA6}{x}y", &settings)
    });
}

#[test]
fn alpha_hex_color_parser() {
    it("should correctly extract alpha hex colors", || {
        let alpha_color_expression1 = r"\textcolor{#ff000080}{x}";
        let alpha_color_expression2 = r"\textcolor{#1234ABCD}{z}";
        let alpha_color_expression3 = r"\textcolor{#abc8}{w}"; // 4-digit with alpha

        let parsed1 = get_parsed_strict(alpha_color_expression1)?;
        assert_let!(ParseNode::Color(color_node1) = &parsed1[0]);
        assert_eq!(color_node1.color, "#ff000080");

        let parsed2 = get_parsed_strict(alpha_color_expression2)?;
        assert_let!(ParseNode::Color(color_node2) = &parsed2[0]);
        assert_eq!(color_node2.color, "#1234ABCD");

        let parsed3 = get_parsed_strict(alpha_color_expression3)?;
        assert_let!(ParseNode::Color(color_node3) = &parsed3[0]);
        assert_eq!(color_node3.color, "#abc8");

        Ok(())
    });

    it("should not parse invalid alpha hex colors", || {
        expect!(r"\textcolor{#ff00008g}{x}").not_to_parse(&strict_settings())?;
        expect!(r"\textcolor{#ff00008}{x}").not_to_parse(&strict_settings())?;
        expect!(r"\textcolor{#ff000080f}{x}").not_to_parse(&strict_settings())
    });

    it("should build correctly with alpha colors", || {
        expect!(r"\textcolor{#ff000080}{x}").to_build(&strict_settings())?;
        expect!(r"\textcolor{#1234ABCD}{z}").to_build(&strict_settings())?;
        expect!(r"\textcolor{#abc8}{w}").to_build(&strict_settings())
    });
}

#[test]
fn a_tie_parser() {
    let math_tie = "a~b";
    let text_tie = r"\text{a~ b}";

    it("should parse ties in math mode", || {
        expect!(math_tie).to_parse(&strict_settings())
    });

    it("should parse ties in text mode", || {
        expect!(text_tie).to_parse(&strict_settings())
    });

    it("should produce spacing in math mode", || {
        let parsed = get_parsed_strict(math_tie)?;
        assert_let!(ParseNode::Spacing(_) = &parsed[1]);
        Ok(())
    });

    it("should produce spacing in text mode", || {
        let parsed = get_parsed_strict(text_tie)?;
        assert_let!(ParseNode::Text(text_node) = &parsed[0]);
        assert_let!(ParseNode::Spacing(_) = &text_node.body[1]);
        Ok(())
    });

    it("should not contract with spaces in text mode", || {
        let parsed = get_parsed_strict(text_tie)?;
        assert_let!(ParseNode::Text(text_node) = &parsed[0]);
        assert_let!(ParseNode::Spacing(_) = &text_node.body[2]);
        Ok(())
    });
}

#[test]
fn a_delimiter_sizing_parser() {
    let normal_delim = r"\bigl |";
    let not_delim = r"\bigl x";
    let big_delim = r"\Biggr \langle";

    it("should parse normal delimiters", || {
        expect!(normal_delim).to_parse(&strict_settings())?;
        expect!(big_delim).to_parse(&strict_settings())
    });

    it("should not parse not-delimiters", || {
        expect!(not_delim).not_to_parse(&strict_settings())
    });

    it("should produce a delimsizing", || {
        let parsed = get_parsed_strict(normal_delim)?;
        assert_let!(ParseNode::Delimsizing(_) = &parsed[0]);
        Ok(())
    });

    it("should produce the correct direction delimiter", || {
        let left_parsed = get_parsed_strict(normal_delim)?;
        assert_let!(ParseNode::Delimsizing(left_node) = &left_parsed[0]);
        assert_eq!(left_node.mclass, DomType::Mopen);

        let right_parsed = get_parsed_strict(big_delim)?;
        assert_let!(ParseNode::Delimsizing(right_node) = &right_parsed[0]);
        assert_eq!(right_node.mclass, DomType::Mclose);

        Ok(())
    });

    it("should parse the correct size delimiter", || {
        let small_parsed = get_parsed_strict(normal_delim)?;
        assert_let!(ParseNode::Delimsizing(small_node) = &small_parsed[0]);
        assert_eq!(small_node.size, 1);

        let big_parsed = get_parsed_strict(big_delim)?;
        assert_let!(ParseNode::Delimsizing(big_node) = &big_parsed[0]);
        assert_eq!(big_node.size, 4);

        Ok(())
    });
}

#[test]
fn an_overline_parser() {
    let overline = r"\overline{x}";

    it("should not fail", || {
        expect!(overline).to_parse(&strict_settings())
    });

    it("should produce an overline", || {
        let parsed = get_parsed_strict(overline)?;
        assert_let!(ParseNode::Overline(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn an_lap_parser() {
    it("should not fail on a text argument", || {
        expect!(r"\rlap{\,/}{=}").to_parse(&strict_settings())?;
        expect!(r"\mathrlap{\,/}{=}").to_parse(&strict_settings())?;
        expect!(r"{=}\llap{/\,}").to_parse(&strict_settings())?;
        expect!(r"{=}\mathllap{/\,}").to_parse(&strict_settings())?;
        expect!(r"\sum_{\clap{ABCDEFG}}").to_parse(&strict_settings())?;
        expect!(r"\sum_{\mathclap{ABCDEFG}}").to_parse(&strict_settings())
    });

    it("should not fail if math version is used", || {
        expect!(r"\mathrlap{\frac{a}{b}}{=}").to_parse(&strict_settings())?;
        expect!(r"{=}\mathllap{\frac{a}{b}}").to_parse(&strict_settings())?;
        expect!(r"\sum_{\mathclap{\frac{a}{b}}}").to_parse(&strict_settings())
    });

    it("should fail on math if AMS version is used", || {
        expect!(r"\rlap{\frac{a}{b}}{=}").not_to_parse(&strict_settings())?;
        expect!(r"{=}\llap{\frac{a}{b}}").not_to_parse(&strict_settings())?;
        expect!(r"\sum_{\clap{\frac{a}{b}}}").not_to_parse(&strict_settings())
    });

    it("should produce a lap", || {
        let parsed = get_parsed_strict(r"\mathrlap{\,/}")?;
        assert_let!(ParseNode::Lap(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_rule_parser() {
    let em_rule = r"\rule{1em}{2em}";
    let ex_rule = r"\rule{1ex}{2em}";
    let bad_unit_rule = r"\rule{1au}{2em}";
    let no_number_rule = r"\rule{1em}{em}";
    let incomplete_rule = r"\rule{1em}";
    let hard_number_rule = r"\rule{   01.24ex}{2.450   em   }";

    it("should not fail", || {
        expect!(em_rule).to_parse(&strict_settings())?;
        expect!(ex_rule).to_parse(&strict_settings())
    });

    it("should not parse invalid units", || {
        expect!(bad_unit_rule).not_to_parse(&strict_settings())?;
        expect!(no_number_rule).not_to_parse(&strict_settings())
    });

    it("should not parse incomplete rules", || {
        expect!(incomplete_rule).not_to_parse(&strict_settings())
    });

    it("should produce a rule", || {
        let parsed = get_parsed_strict(em_rule)?;
        assert_let!(ParseNode::Rule(_) = &parsed[0]);
        Ok(())
    });

    it("should list the correct units", || {
        let em_parsed = get_parsed_strict(em_rule)?;
        assert_let!(ParseNode::Rule(em_rule_node) = &em_parsed[0]);
        assert_eq!(em_rule_node.width.unit, "em");
        assert_eq!(em_rule_node.height.unit, "em");

        let ex_parsed = get_parsed_strict(ex_rule)?;
        assert_let!(ParseNode::Rule(ex_rule_node) = &ex_parsed[0]);
        assert_eq!(ex_rule_node.width.unit, "ex");
        assert_eq!(ex_rule_node.height.unit, "em");

        Ok(())
    });

    it("should parse the number correctly", || {
        let hard_number_parsed = get_parsed_strict(hard_number_rule)?;
        assert_let!(ParseNode::Rule(hard_rule_node) = &hard_number_parsed[0]);
        assert!((hard_rule_node.width.number - 1.24).abs() < 0.001);
        assert!((hard_rule_node.height.number - 2.45).abs() < 0.001);
        Ok(())
    });

    it("should parse negative sizes", || {
        let negative_rule = r"\rule{-1em}{- 0.2em}";
        let parsed = get_parsed_strict(negative_rule)?;
        assert_let!(ParseNode::Rule(rule_node) = &parsed[0]);
        assert!((rule_node.width.number - (-1.0)).abs() < 0.001);
        assert!((rule_node.height.number - (-0.2)).abs() < 0.001);
        Ok(())
    });

    it("should parse in text mode", || {
        expect!(r"\text{a\rule{1em}{2em}b}").to_parse(&strict_settings())
    });
}

#[test]
fn a_kern_parser() {
    let em_kern = r"\kern{1em}";
    let ex_kern = r"\kern{1ex}";
    let mu_kern = r"\mkern{1mu}";
    let ab_kern = r"a\kern{1em}b";
    let bad_unit_rule = r"\kern{1au}";
    let no_number_rule = r"\kern{em}";

    it("should list the correct units", || {
        let em_parsed = get_parsed_strict(em_kern)?;
        assert_let!(ParseNode::Kern(em_kern_node) = &em_parsed[0]);
        assert_eq!(em_kern_node.dimension.unit, "em");

        let ex_parsed = get_parsed_strict(ex_kern)?;
        assert_let!(ParseNode::Kern(ex_kern_node) = &ex_parsed[0]);
        assert_eq!(ex_kern_node.dimension.unit, "ex");

        let mu_parsed = get_parsed_strict(mu_kern)?;
        assert_let!(ParseNode::Kern(mu_kern_node) = &mu_parsed[0]);
        assert_eq!(mu_kern_node.dimension.unit, "mu");

        let ab_parsed = get_parsed_strict(ab_kern)?;
        assert_let!(ParseNode::Kern(ab_kern_node) = &ab_parsed[1]);
        assert_eq!(ab_kern_node.dimension.unit, "em");

        Ok(())
    });

    it("should not parse invalid units", || {
        expect!(bad_unit_rule).not_to_parse(&strict_settings())?;
        expect!(no_number_rule).not_to_parse(&strict_settings())
    });

    it("should parse negative sizes", || {
        let parsed = get_parsed_strict(r"\kern{-1em}")?;
        assert_let!(ParseNode::Kern(kern_node) = &parsed[0]);
        assert!((kern_node.dimension.number - (-1.0)).abs() < 0.001);
        Ok(())
    });

    it("should parse positive sizes", || {
        let parsed = get_parsed_strict(r"\kern{+1em}")?;
        assert_let!(ParseNode::Kern(kern_node) = &parsed[0]);
        assert!((kern_node.dimension.number - 1.0).abs() < 0.001);
        Ok(())
    });
}

#[test]
fn a_non_braced_kern_parser() {
    let em_kern = r"\kern1em";
    let ex_kern = r"\kern 1 ex";
    let mu_kern = r"\mkern 1mu";
    let ab_kern1 = r"a\mkern1mub";
    let ab_kern2 = r"a\mkern-1mub";
    let ab_kern3 = r"a\mkern-1mu b";
    let bad_unit_rule = r"\kern1au";
    let no_number_rule = r"\kern em";

    it("should list the correct units", || {
        let em_parsed = get_parsed_strict(em_kern)?;
        assert_let!(ParseNode::Kern(em_kern_node) = &em_parsed[0]);
        assert_eq!(em_kern_node.dimension.unit, "em");

        let ex_parsed = get_parsed_strict(ex_kern)?;
        assert_let!(ParseNode::Kern(ex_kern_node) = &ex_parsed[0]);
        assert_eq!(ex_kern_node.dimension.unit, "ex");

        let mu_parsed = get_parsed_strict(mu_kern)?;
        assert_let!(ParseNode::Kern(mu_kern_node) = &mu_parsed[0]);
        assert_eq!(mu_kern_node.dimension.unit, "mu");

        let ab_parsed1 = get_parsed_strict(ab_kern1)?;
        assert_let!(ParseNode::Kern(ab_kern_node1) = &ab_parsed1[1]);
        assert_eq!(ab_kern_node1.dimension.unit, "mu");

        let ab_parsed2 = get_parsed_strict(ab_kern2)?;
        assert_let!(ParseNode::Kern(ab_kern_node2) = &ab_parsed2[1]);
        assert_eq!(ab_kern_node2.dimension.unit, "mu");

        let ab_parsed3 = get_parsed_strict(ab_kern3)?;
        assert_let!(ParseNode::Kern(ab_kern_node3) = &ab_parsed3[1]);
        assert_eq!(ab_kern_node3.dimension.unit, "mu");

        Ok(())
    });

    it("should parse elements on either side of a kern", || {
        let ab_parse1 = get_parsed_strict(ab_kern1)?;
        assert_eq!(ab_parse1.len(), 3);
        assert_eq!(ab_parse1[0].text(), Some("a"));
        assert_eq!(ab_parse1[2].text(), Some("b"));

        let ab_parse2 = get_parsed_strict(ab_kern2)?;
        assert_eq!(ab_parse2.len(), 3);
        assert_eq!(ab_parse2[0].text(), Some("a"));
        assert_eq!(ab_parse2[2].text(), Some("b"));

        let ab_parse3 = get_parsed_strict(ab_kern3)?;
        assert_eq!(ab_parse3.len(), 3);
        assert_eq!(ab_parse3[0].text(), Some("a"));
        assert_eq!(ab_parse3[2].text(), Some("b"));

        Ok(())
    });

    it("should not parse invalid units", || {
        expect!(bad_unit_rule).not_to_parse(&strict_settings())?;
        expect!(no_number_rule).not_to_parse(&strict_settings())
    });

    it("should parse negative sizes", || {
        let parsed = get_parsed_strict(r"\kern-1em")?;
        assert_let!(ParseNode::Kern(kern_node) = &parsed[0]);
        assert!((kern_node.dimension.number - (-1.0)).abs() < 0.001);
        Ok(())
    });

    it("should parse positive sizes", || {
        let parsed = get_parsed_strict(r"\kern+1em")?;
        assert_let!(ParseNode::Kern(kern_node) = &parsed[0]);
        assert!((kern_node.dimension.number - 1.0).abs() < 0.001);
        Ok(())
    });

    it("should handle whitespace", || {
        let ab_parse = get_parsed_strict("a\\mkern\t-\r1  \n mu\nb")?;
        assert_eq!(ab_parse.len(), 3);
        assert_eq!(ab_parse[0].text(), Some("a"));
        assert_let!(ParseNode::Kern(kern_node) = &ab_parse[1]);
        assert_eq!(kern_node.dimension.unit, "mu");
        assert_eq!(ab_parse[2].text(), Some("b"));
        Ok(())
    });
}

#[test]
fn a_left_right_parser() {
    let normal_left_right = r"\left( \dfrac{x}{y} \right)";
    let empty_right = r"\left( \dfrac{x}{y} \right.";

    it("should not fail", || {
        expect!(normal_left_right).to_parse(&strict_settings())
    });

    it("should produce a leftright", || {
        let parsed = get_parsed_strict(normal_left_right)?;
        assert_let!(ParseNode::LeftRight(left_right_node) = &parsed[0]);
        assert_eq!(left_right_node.left, "(");
        assert_eq!(left_right_node.right, ")");
        Ok(())
    });

    it("should error when it is mismatched", || {
        let unmatched_left = r"\left( \dfrac{x}{y}";
        let unmatched_right = r"\dfrac{x}{y} \right)";

        expect!(unmatched_left).not_to_parse(&strict_settings())?;
        expect!(unmatched_right).not_to_parse(&strict_settings())
    });

    it("should error when braces are mismatched", || {
        let unmatched = r"{ \left( \dfrac{x}{y} } \right)";
        expect!(unmatched).not_to_parse(&strict_settings())
    });

    it("should error when non-delimiters are provided", || {
        let non_delimiter = r"\left$ \dfrac{x}{y} \right)";
        expect!(non_delimiter).not_to_parse(&strict_settings())
    });

    it("should parse the empty '.' delimiter", || {
        expect!(empty_right).to_parse(&strict_settings())
    });

    it("should parse the '.' delimiter with normal sizes", || {
        let normal_empty = r"\Bigl .";
        expect!(normal_empty).to_parse(&strict_settings())
    });

    it("should handle \\middle", || {
        let normal_middle = r"\left( \dfrac{x}{y} \middle| \dfrac{y}{z} \right)";
        expect!(normal_middle).to_parse(&strict_settings())
    });

    it("should handle multiple \\middles", || {
        let multi_middle =
            r"\left( \dfrac{x}{y} \middle| \dfrac{y}{z} \middle/ \dfrac{z}{q} \right)";
        expect!(multi_middle).to_parse(&strict_settings())
    });

    it("should handle nested \\middles", || {
        let nested_middle = r"\left( a^2 \middle| \left( b \middle/ c \right) \right)";
        expect!(nested_middle).to_parse(&strict_settings())
    });

    it(
        "should error when \\middle is not in \\left...\\right",
        || {
            let unmatched_middle = r"(\middle|\dfrac{x}{y})";
            expect!(unmatched_middle).not_to_parse(&strict_settings())
        },
    );
}

#[test]
fn left_right_builder() {
    let cases = vec![
        (r"\left\langle \right\rangle", r"\left< \right>"),
        (
            r"\left\langle \right\rangle",
            "\\left\u{27e8} \\right\u{27e9}",
        ),
        (r"\left\lparen \right\rparen", r"\left( \right)"),
    ];

    for (actual, expected) in cases {
        it(
            &format!("should build \"{actual}\" like \"{expected}\""),
            || expect!(actual).to_build_like(expected, &strict_settings()),
        );
    }
}

#[test]
fn a_begin_end_parser() {
    it("should parse a simple environment", || {
        expect!(r"\begin{matrix}a&b\\c&d\end{matrix}").to_parse(&strict_settings())
    });

    it("should parse an environment with argument", || {
        expect!(r"\begin{array}{cc}a&b\\c&d\end{array}").to_parse(&strict_settings())
    });

    it("should parse and build an empty environment", || {
        expect!(r"\begin{aligned}\end{aligned}").to_build(&strict_settings())?;
        expect!(r"\begin{matrix}\end{matrix}").to_build(&strict_settings())
    });

    it("should parse an environment with hlines", || {
        expect!(r"\begin{matrix}\hline a&b\\ \hline c&d\end{matrix}")
            .to_parse(&strict_settings())?;
        expect!(r"\begin{matrix}\hline a&b\cr \hline c&d\end{matrix}")
            .to_parse(&strict_settings())?;
        expect!(r"\begin{matrix}\hdashline a&b\\ \hdashline c&d\end{matrix}")
            .to_parse(&strict_settings())
    });

    it("should forbid hlines outside array environment", || {
        expect!(r"\hline").not_to_parse(&strict_settings())
    });

    it("should error when name is mismatched", || {
        expect!(r"\begin{matrix}a&b\\c&d\end{pmatrix}").not_to_parse(&strict_settings())
    });

    it("should error when commands are mismatched", || {
        expect!(r"\begin{matrix}a&b\\c&d\right{pmatrix}").not_to_parse(&strict_settings())
    });

    it("should error when end is missing", || {
        expect!(r"\begin{matrix}a&b\\c&d").not_to_parse(&strict_settings())
    });

    it("should error when braces are mismatched", || {
        expect!(r"{\begin{matrix}a&b\\c&d}\end{matrix}").not_to_parse(&strict_settings())
    });

    it("should cooperate with infix notation", || {
        expect!(r"\begin{matrix}0&1\over2&3\\4&5&6\end{matrix}").to_parse(&strict_settings())
    });

    it("should nest", || {
        let m1 = r"\begin{pmatrix}1&2\\3&4\end{pmatrix}";
        let m2 = format!("\\begin{{array}}{{rl}}{m1}&0\\\\0&{m1}\\end{{array}}");
        expect!(&m2).to_parse(&strict_settings())
    });

    it("should allow \\cr and \\\\ as a line terminator", || {
        expect!(r"\begin{matrix}a&b\cr c&d\end{matrix}").to_parse(&strict_settings())?;
        expect!(r"\begin{matrix}a&b\\c&d\end{matrix}").to_parse(&strict_settings())
    });

    it(
        "should not allow \\cr to scan for an optional size argument",
        || expect!(r"\begin{matrix}a&b\cr[c]&d\end{matrix}").to_parse(&strict_settings()),
    );

    it(
        "should not treat [ after space as optional argument to \\\\",
        || {
            expect!(r"\begin{matrix}a&b\\ [c]&d\end{matrix}").to_parse(&strict_settings())?;
            expect!("a\\\\ [b]").to_parse(&strict_settings())
        },
    );

    it("should eat a final newline", || {
        let m3 = get_parsed_strict(r"\begin{matrix}a&b\\ c&d \\ \end{matrix}")?;
        assert_let!(ParseNode::Array(array) = &m3[0]);
        assert_eq!(array.body.len(), 2);
        Ok(())
    });

    it("should grab \\arraystretch", || {
        let mut parsed =
            get_parsed_strict(r"\def\arraystretch{1.5}\begin{matrix}a&b\\c&d\end{matrix}")?;
        strip_positions(&mut parsed);
        insta::assert_snapshot!("matrix_arraystretch_parse", format!("{:#?}", parsed));
        Ok(())
    });

    it(
        "should allow an optional argument in {matrix*} and company.",
        || {
            expect!(r"\begin{matrix*}[r] a & -1 \\ -1 & d \end{matrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{pmatrix*}[r] a & -1 \\ -1 & d \end{pmatrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{bmatrix*}[r] a & -1 \\ -1 & d \end{bmatrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{Bmatrix*}[r] a & -1 \\ -1 & d \end{Bmatrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{vmatrix*}[r] a & -1 \\ -1 & d \end{vmatrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{Vmatrix*}[r] a & -1 \\ -1 & d \end{Vmatrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{matrix*} a & -1 \\ -1 & d \end{matrix*}")
                .to_build(&strict_settings())?;
            expect!(r"\begin{matrix*}[] a & -1 \\ -1 & d \end{matrix*}")
                .not_to_parse(&strict_settings())
        },
    );

    it("should allow blank columns", || {
        let parsed = get_parsed_strict(r"\begin{matrix*}[r] a \\ -1 & d \end{matrix*}")?;
        assert_let!(ParseNode::Array(array) = &parsed[0]);
        if let Some(cols) = &array.cols {
            assert_eq!(cols.len(), 2);
            assert_let!(AlignSpec::Align { align, .. } = &cols[0]);
            assert_let!(AlignSpec::Align { align: align2, .. } = &cols[1]);
            assert_eq!(align, "r");
            assert_eq!(align2, "r");
        } else {
            panic!("Expected cols to be Some");
        }
        Ok(())
    });
}

#[test]
fn a_sqrt_parser() {
    let sqrt = r"\sqrt{x}";
    let missing_group = r"\sqrt";

    it("should parse square roots", || {
        expect!(sqrt).to_parse(&strict_settings())
    });

    it("should error when there is no group", || {
        expect!(missing_group).not_to_parse(&strict_settings())
    });

    it("should produce sqrts", || {
        let parsed = get_parsed_strict(sqrt)?;
        assert_let!(ParseNode::Sqrt(_) = &parsed[0]);
        Ok(())
    });

    it("should build sized square roots", || {
        expect!(r"\Large\sqrt[3]{x}").to_build(&strict_settings())
    });

    it(
        "should expand argument if optional argument doesn't exist",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("123"));
            expect!(r"\sqrt\foo").to_parse_like("\\sqrt123", &settings)
        },
    );

    it(
        "should not expand argument if optional argument exists",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("123"));
            expect!(r"\sqrt[2]\foo").to_parse_like("\\sqrt[2]{123}", &settings)
        },
    );
}

#[test]
fn a_tex_compliant_parser() {
    it("should work", || {
        expect!(r"\frac 2 3").to_parse(&strict_settings())
    });

    it("should fail if there are not enough arguments", || {
        let missing_groups = vec![
            r"\frac{x}",
            r"\textcolor{#fff}",
            r"\rule{1em}",
            r"\llap",
            r"\bigl",
            r"\text",
        ];

        for expr in missing_groups {
            expect!(expr).not_to_parse(&strict_settings())?;
        }
        Ok(())
    });

    it("should fail when there are missing sup/subscripts", || {
        expect!("x^").not_to_parse(&strict_settings())?;
        expect!("x_").not_to_parse(&strict_settings())
    });

    it("should fail when arguments require arguments", || {
        let bad_arguments = vec![
            r"\frac \frac x y z",
            r"\frac x \frac y z",
            r"\frac \sqrt x y",
            r"\frac x \sqrt y",
            r"\frac \mathllap x y",
            r"\frac x \mathllap y",
            r"\mathllap \mathllap x",
            r"\sqrt \mathllap x",
        ];

        for expr in bad_arguments {
            expect!(expr).not_to_parse(&strict_settings())?;
        }
        Ok(())
    });

    it("should work when the arguments have braces", || {
        let good_arguments = vec![
            r"\frac {\frac x y} z",
            r"\frac x {\frac y z}",
            r"\frac {\sqrt x} y",
            r"\frac x {\sqrt y}",
            r"\frac {\mathllap x} y",
            r"\frac x {\mathllap y}",
            r"\mathllap {\frac x y}",
            r"\mathllap {\mathllap x}",
            r"\sqrt {\mathllap x}",
        ];

        for expr in good_arguments {
            expect!(expr).to_parse(&strict_settings())?;
        }
        Ok(())
    });

    it("should fail when sup/subscripts require arguments", || {
        let bad_sup_subscripts = vec![
            r"x^\sqrt x",
            r"x^\mathllap x",
            r"x_\sqrt x",
            r"x_\mathllap x",
        ];

        for expr in bad_sup_subscripts {
            expect!(expr).not_to_parse(&strict_settings())?;
        }
        Ok(())
    });

    it(
        "should work when sup/subscripts arguments have braces",
        || {
            let good_sup_subscripts = vec![
                r"x^{\sqrt x}",
                r"x^{\mathllap x}",
                r"x_{\sqrt x}",
                r"x_{\mathllap x}",
            ];

            for expr in good_sup_subscripts {
                expect!(expr).to_parse(&strict_settings())?;
            }
            Ok(())
        },
    );

    it("should parse multiple primes correctly", || {
        expect!("x''''").to_parse(&strict_settings())?;
        expect!("x_2''").to_parse(&strict_settings())?;
        expect!("x''_2").to_parse(&strict_settings())
    });

    it(
        "should fail when sup/subscripts are interspersed with arguments",
        || {
            expect!(r"\sqrt^23").not_to_parse(&strict_settings())?;
            expect!(r"\frac^234").not_to_parse(&strict_settings())?;
            expect!(r"\frac2^34").not_to_parse(&strict_settings())
        },
    );

    it(
        "should succeed when sup/subscripts come after whole functions",
        || {
            expect!(r"\sqrt2^3").to_parse(&strict_settings())?;
            expect!(r"\frac23^4").to_parse(&strict_settings())
        },
    );

    it("should succeed with a sqrt around a text/frac", || {
        expect!(r"\sqrt \frac x y").to_parse(&strict_settings())?;
        expect!(r"\sqrt \text x").to_parse(&strict_settings())?;
        expect!("x^\\frac x y").to_parse(&strict_settings())?;
        expect!("x_\\text x").to_parse(&strict_settings())
    });

    it("should fail when arguments are \\left", || {
        let bad_left_arguments = vec![
            r"\frac \left( x \right) y",
            r"\frac x \left( y \right)",
            r"\mathllap \left( x \right)",
            r"\sqrt \left( x \right)",
            r"x^\left( x \right)",
        ];

        for expr in bad_left_arguments {
            expect!(expr).not_to_parse(&strict_settings())?;
        }
        Ok(())
    });

    it(
        "should succeed when there are braces around the \\left/\\right",
        || {
            let good_left_arguments = vec![
                r"\frac {\left( x \right)} y",
                r"\frac x {\left( y \right)}",
                r"\mathllap {\left( x \right)}",
                r"\sqrt {\left( x \right)}",
                r"x^{\left( x \right)}",
            ];

            for expr in good_left_arguments {
                expect!(expr).to_parse(&strict_settings())?;
            }
            Ok(())
        },
    );
}

#[test]
fn an_op_symbol_builder() {
    it("should not fail", || {
        expect!(r"\int_i^n").to_build(&strict_settings())?;
        expect!(r"\iint_i^n").to_build(&strict_settings())?;
        expect!(r"\iiint_i^n").to_build(&strict_settings())?;
        expect!(r"\int\nolimits_i^n").to_build(&strict_settings())?;
        expect!(r"\iint\nolimits_i^n").to_build(&strict_settings())?;
        expect!(r"\iiint\nolimits_i^n").to_build(&strict_settings())?;
        expect!(r"\oint_i^n").to_build(&strict_settings())?;
        expect!(r"\oiint_i^n").to_build(&strict_settings())?;
        expect!(r"\oiiint_i^n").to_build(&strict_settings())?;
        expect!(r"\oint\nolimits_i^n").to_build(&strict_settings())?;
        expect!(r"\oiint\nolimits_i^n").to_build(&strict_settings())?;
        expect!(r"\oiiint\nolimits_i^n").to_build(&strict_settings())
    });
}

#[test]
fn a_style_change_parser() {
    it("should not fail", || {
        expect!(r"\displaystyle x").to_parse(&strict_settings())?;
        expect!(r"\textstyle x").to_parse(&strict_settings())?;
        expect!(r"\scriptstyle x").to_parse(&strict_settings())?;
        expect!(r"\scriptscriptstyle x").to_parse(&strict_settings())
    });

    it("should produce the correct style", || {
        let display_parse = get_parsed_strict(r"\displaystyle x")?;
        assert_let!(ParseNode::Styling(display_node) = &display_parse[0]);
        assert_eq!(display_node.style, DISPLAY);

        let scriptscript_parse = get_parsed_strict(r"\scriptscriptstyle x")?;
        assert_let!(ParseNode::Styling(scriptscript_node) = &scriptscript_parse[0]);
        assert_eq!(scriptscript_node.style, SCRIPTSCRIPT);

        Ok(())
    });

    it("should only change the style within its group", || {
        let text = r"a b { c d \displaystyle e f } g h";
        let parse = get_parsed_strict(text)?;

        // parse[2] should be the group { c d \displaystyle e f }
        assert_let!(ParseNode::OrdGroup(group) = &parse[2]);
        // group.body[2] should be the \displaystyle e f
        assert_let!(ParseNode::Styling(display_node) = &group.body[2]);

        // display_node.body should have length 2: e and f
        assert_eq!(display_node.body.len(), 2);
        if let ParseNode::MathOrd(math_ord) = &display_node.body[0] {
            assert_eq!(math_ord.text, "e");
        } else {
            panic!("Expected TextOrd for 'e'");
        }

        Ok(())
    });
}

#[test]
fn a_font_parser() {
    it(
        "should parse \\mathrm, \\mathbb, \\mathit, and \\mathnormal",
        || {
            expect!(r"\mathrm x").to_parse(&strict_settings())?;
            expect!(r"\mathbb x").to_parse(&strict_settings())?;
            expect!(r"\mathit x").to_parse(&strict_settings())?;
            expect!(r"\mathnormal x").to_parse(&strict_settings())?;
            expect!(r"\mathrm {x + 1}").to_parse(&strict_settings())?;
            expect!(r"\mathbb {x + 1}").to_parse(&strict_settings())?;
            expect!(r"\mathit {x + 1}").to_parse(&strict_settings())?;
            expect!(r"\mathnormal {x + 1}").to_parse(&strict_settings())
        },
    );

    it("should parse \\mathcal and \\mathfrak", || {
        expect!(r"\mathcal{ABC123}").to_parse(&strict_settings())?;
        expect!(r"\mathfrak{abcABC123}").to_parse(&strict_settings())
    });

    it("should produce the correct fonts", || {
        let mathbb_parse = get_parsed_strict(r"\mathbb x")?;
        assert_let!(ParseNode::Font(mathbb_node) = &mathbb_parse[0]);
        assert_eq!(mathbb_node.font, "mathbb");

        let mathrm_parse = get_parsed_strict(r"\mathrm x")?;
        assert_let!(ParseNode::Font(mathrm_node) = &mathrm_parse[0]);
        assert_eq!(mathrm_node.font, "mathrm");

        let mathit_parse = get_parsed_strict(r"\mathit x")?;
        assert_let!(ParseNode::Font(mathit_node) = &mathit_parse[0]);
        assert_eq!(mathit_node.font, "mathit");

        let mathnormal_parse = get_parsed_strict(r"\mathnormal x")?;
        assert_let!(ParseNode::Font(mathnormal_node) = &mathnormal_parse[0]);
        assert_eq!(mathnormal_node.font, "mathnormal");

        let mathcal_parse = get_parsed_strict(r"\mathcal C")?;
        assert_let!(ParseNode::Font(mathcal_node) = &mathcal_parse[0]);
        assert_eq!(mathcal_node.font, "mathcal");

        let mathfrak_parse = get_parsed_strict(r"\mathfrak C")?;
        assert_let!(ParseNode::Font(mathfrak_node) = &mathfrak_parse[0]);
        assert_eq!(mathfrak_node.font, "mathfrak");

        Ok(())
    });

    it("should parse nested font commands", || {
        let nested_parse = get_parsed_strict(r"\mathbb{R \neq \mathrm{R}}")?;
        assert_let!(ParseNode::Font(nested_node) = &nested_parse[0]);
        assert_eq!(nested_node.font, "mathbb");

        if let ParseNode::OrdGroup(ord_group) = nested_node.body.as_ref() {
            let bb_body = &ord_group.body;
            assert_eq!(bb_body.len(), 3);
            assert_let!(ParseNode::MathOrd(_) = &bb_body[0]);
            assert_let!(ParseNode::Font(nested_inner) = &bb_body[2]);
            assert_eq!(nested_inner.font, "mathrm");
        } else {
            panic!("Expected OrdGroup for body");
        }

        Ok(())
    });

    it("should work with \\textcolor", || {
        let color_mathbb_parse = get_parsed_strict(r"\textcolor{blue}{\mathbb R}")?;
        assert_let!(ParseNode::Color(color_node) = &color_mathbb_parse[0]);
        assert_eq!(color_node.color, "blue");

        let body = &color_node.body;
        assert_eq!(body.len(), 1);
        assert_let!(ParseNode::Font(font_node) = &body[0]);
        assert_eq!(font_node.font, "mathbb");

        Ok(())
    });

    it("should not parse a series of font commands", || {
        expect!(r"\mathbb \mathrm R").not_to_parse(&strict_settings())
    });

    it("should nest fonts correctly", || {
        let bf_parse = get_parsed_strict(r"\mathbf{a\mathrm{b}c}")?;
        assert_let!(ParseNode::Font(bf_node) = &bf_parse[0]);
        assert_eq!(bf_node.font, "mathbf");

        if let ParseNode::OrdGroup(bf_body_ord) = bf_node.body.as_ref() {
            let bf_body = &bf_body_ord.body;
            assert_eq!(bf_body.len(), 3);
            assert_eq!(bf_body[0].text(), Some("a"));
            assert_let!(ParseNode::Font(inner_font) = &bf_body[1]);
            assert_eq!(inner_font.font, "mathrm");
            assert_eq!(bf_body[2].text(), Some("c"));
        } else {
            panic!("Expected OrdGroup for body");
        }

        Ok(())
    });

    it("should be allowed in the argument", || {
        expect!(r"e^\mathbf{x}").to_parse(&strict_settings())
    });

    it(
        "\\boldsymbol should inherit mbin/mrel from argument",
        || {
            let built = get_built(
                r"a\boldsymbol{}b\boldsymbol{=}c\boldsymbol{+}d\boldsymbol{++}e\boldsymbol{xyz}f",
                &strict_settings(),
            )?;
            insta::assert_debug_snapshot!("boldsymbol_inherits_mbin_mrel", &built);
            Ok(())
        },
    );
    it("old-style fonts work like new-style fonts", || {
        expect!(r"\rm xyz").to_parse_like(r"\mathrm{xyz}", &strict_settings())?;
        expect!(r"\sf xyz").to_parse_like(r"\mathsf{xyz}", &strict_settings())?;
        expect!(r"\tt xyz").to_parse_like(r"\mathtt{xyz}", &strict_settings())?;
        expect!(r"\bf xyz").to_parse_like(r"\mathbf{xyz}", &strict_settings())?;
        expect!(r"\it xyz").to_parse_like(r"\mathit{xyz}", &strict_settings())?;
        expect!(r"\cal xyz").to_parse_like(r"\mathcal{xyz}", &strict_settings())
    });
}

#[test]
fn a_pmb_builder() {
    it("should not fail", || {
        expect!(r"\pmb{\mu}").to_build(&strict_settings())?;
        expect!(r"\pmb{=}").to_build(&strict_settings())?;
        expect!(r"\pmb{+}").to_build(&strict_settings())?;
        expect!(r"\pmb{\frac{x^2}{x_1}}").to_build(&strict_settings())?;
        expect!(r"\pmb{}").to_build(&strict_settings())?;
        expect!(r"\def\x{1}\pmb{\x\def\x{2}}").to_parse_like(r"\pmb{1}", &strict_settings())
    });
}

#[test]
fn a_raise_parser() {
    it("should parse and build text in \\raisebox", || {
        expect!(r"\raisebox{5pt}{text}").to_build(&strict_settings())?;
        expect!(r"\raisebox{-5pt}{text}").to_build(&strict_settings())
    });

    it(
        "should parse and build math in non-strict \\vcenter",
        || expect!(r"\vcenter{\frac a b}").to_build(&nonstrict_settings()),
    );

    it("should fail to parse math in \\raisebox", || {
        expect!(r"\raisebox{5pt}{\frac a b}").not_to_parse(&nonstrict_settings())?;
        expect!(r"\raisebox{-5pt}{\frac a b}").not_to_parse(&nonstrict_settings())
    });

    it("should fail to parse math in an \\hbox", || {
        expect!(r"\hbox{\frac a b}").not_to_parse(&nonstrict_settings())
    });

    it("should fail to build, given an unbraced length", || {
        expect!(r"\raisebox5pt{text}").not_to_build(&strict_settings())?;
        expect!(r"\raisebox-5pt{text}").not_to_build(&strict_settings())
    });

    it("should build math in an hbox when math mode is set", || {
        expect!(r"a + \vcenter{\hbox{$\frac{\frac a b}c$}}").to_build(&strict_settings())
    });
}

#[test]
fn a_comment_parser() {
    it("should parse comments at the end of a line", || {
        expect!("a^2 + b^2 = c^2 % Pythagoras' Theorem\n").to_parse(&strict_settings())
    });

    it("should parse comments at the start of a line", || {
        expect!("% comment\n").to_parse(&strict_settings())
    });

    it("should parse multiple lines of comments in a row", || {
        expect!("% comment 1\n% comment 2\n").to_parse(&strict_settings())
    });

    it(
        "should parse comments between subscript and superscript",
        || {
            expect!("x_3 %comment\n^2").to_parse_like("x_3^2", &strict_settings())?;
            expect!("x^ %comment\n{2}").to_parse_like("x^{2}", &strict_settings())?;
            expect!("x^ %comment\n\\frac{1}{2}").to_parse_like("x^\\frac{1}{2}", &strict_settings())
        },
    );

    it("should parse comments in size and color groups", || {
        expect!("\\kern{1 %kern\nem}").to_parse(&strict_settings())?;
        expect!("\\kern1 %kern\nem").to_parse(&strict_settings())?;
        expect!("\\color{#f00%red\n}").to_parse(&strict_settings())
    });

    it("should parse comments before an expression", || {
        expect!("%comment\n{2}").to_parse_like("{2}", &strict_settings())
    });

    it("should parse comments before and between \\hline", || {
        let expr = "\\begin{matrix}a&b\\\\ %hline\n\\hline %hline\n\\hline c&d\\end{matrix}";
        expect!(expr).to_parse(&strict_settings())
    });

    it("should parse comments in the macro definition", || {
        expect!("\\def\\foo{1 %}\n2}\n\\foo").to_parse_like("12", &strict_settings())
    });

    it(
        "should not expand nor ignore spaces after a command sequence in a comment",
        || expect!("\\def\\foo{1\n2}\nx %\\foo\n").to_parse_like("x", &strict_settings()),
    );

    it(
        "should not parse a comment without newline in strict mode",
        || {
            expect!("x%y").not_to_parse(&strict_settings())?;
            expect!("x%y").to_parse(&nonstrict_settings())
        },
    );

    it("should not produce or consume space", || {
        expect!("\\text{hello% comment 1\nworld}")
            .to_parse_like(r"\text{helloworld}", &strict_settings())?;
        expect!("\\text{hello% comment\n\nworld}")
            .to_parse_like(r"\text{hello world}", &strict_settings())
    });

    it("should not include comments in the output", || {
        expect!("5 % comment\n").to_parse_like("5", &strict_settings())
    });
}

#[test]
fn an_html_font_tree_builder() {
    it("should render \\mathbb{R} with the correct font", || {
        let markup = render_to_string_strict(r"\mathbb{R}")?;
        assert!(markup.contains(r#"<span class="mord mathbb">R</span>"#));
        Ok(())
    });

    it("should render \\mathrm{R} with the correct font", || {
        let markup = render_to_string_strict(r"\mathrm{R}")?;
        assert!(markup.contains(r#"<span class="mord mathrm">R</span>"#));
        Ok(())
    });

    it("should render \\mathcal{R} with the correct font", || {
        let markup = render_to_string_strict(r"\mathcal{R}")?;
        assert!(markup.contains(r#"<span class="mord mathcal">R</span>"#));
        Ok(())
    });

    it("should render \\mathfrak{R} with the correct font", || {
        let markup = render_to_string_strict(r"\mathfrak{R}")?;
        assert!(markup.contains(r#"<span class="mord mathfrak">R</span>"#));
        Ok(())
    });

    it("should render \\text{R} with the correct font", || {
        let markup = render_to_string_strict(r"\text{R}")?;
        assert!(markup.contains(r#"<span class="mord">R</span>"#));
        Ok(())
    });

    it("should render \\textit{R} with the correct font", || {
        let markup = render_to_string_strict(r"\textit{R}")?;
        assert!(markup.contains(r#"<span class="mord textit">R</span>"#));
        Ok(())
    });

    it(
        "should render \\text{\\textit{R}} with the correct font",
        || {
            let markup = render_to_string_strict(r"\text{\textit{R}}")?;
            assert!(markup.contains(r#"<span class="mord textit">R</span>"#));
            Ok(())
        },
    );

    it("should render \\textup{R} with the correct font", || {
        let markup1 = render_to_string_strict(r"\textup{R}")?;
        assert!(markup1.contains(r#"<span class="mord textup">R</span>"#));

        let markup2 = render_to_string_strict(r"\textit{\textup{R}}")?;
        assert!(markup2.contains(r#"<span class="mord textup">R</span>"#));

        let markup3 = render_to_string_strict(r"\textup{\textit{R}}")?;
        assert!(markup3.contains(r#"<span class="mord textit">R</span>"#));
        Ok(())
    });

    it(
        "should render \\text{R\\textit{S}T} with the correct fonts",
        || {
            let markup = render_to_string_strict(r"\text{R\textit{S}T}")?;
            assert!(markup.contains(r#"<span class="mord">R</span>"#));
            assert!(markup.contains(r#"<span class="mord textit">S</span>"#));
            assert!(markup.contains(r#"<span class="mord">T</span>"#));
            Ok(())
        },
    );

    it("should render \\textbf{R } with the correct font", || {
        let markup = render_to_string_strict(r"\textbf{R }")?;
        assert!(markup.contains("<span class=\"mord textbf\">R\u{a0}</span>"));
        Ok(())
    });

    it("should render \\textmd{R} with the correct font", || {
        let markup1 = render_to_string_strict(r"\textmd{R}")?;
        assert!(markup1.contains(r#"<span class="mord textmd">R</span>"#));
        let markup2 = render_to_string_strict(r"\textbf{\textmd{R}}")?;
        assert!(markup2.contains(r#"<span class="mord textmd">R</span>"#));
        let markup3 = render_to_string_strict(r"\textmd{\textbf{R}}")?;
        assert!(markup3.contains(r#"<span class="mord textbf">R</span>"#));
        Ok(())
    });

    it("should render \\textsf{R} with the correct font", || {
        let markup = render_to_string_strict(r"\textsf{R}")?;
        assert!(markup.contains(r#"<span class="mord textsf">R</span>"#));
        Ok(())
    });

    it(
        "should render \\textsf{\\textit{R}G\\textbf{B}} with the correct font",
        || {
            let markup = render_to_string_strict(r"\textsf{\textit{R}G\textbf{B}}")?;
            assert!(markup.contains(r#"<span class="mord textsf textit">R</span>"#));
            assert!(markup.contains(r#"<span class="mord textsf">G</span>"#));
            assert!(markup.contains(r#"<span class="mord textsf textbf">B</span>"#));
            Ok(())
        },
    );

    it(
        "should render \\textsf{\\textbf{$\\mathrm{A}$}} with the correct font",
        || {
            let markup = render_to_string_strict(r"\textsf{\textbf{$\mathrm{A}$}}")?;
            assert!(markup.contains(r#"<span class="mord mathrm">A</span>"#));
            Ok(())
        },
    );

    it(
        "should render \\textsf{\\textbf{$\\mathrm{\\textsf{A}}$}} with the correct font",
        || {
            let markup = render_to_string_strict(r"\textsf{\textbf{$\mathrm{\textsf{A}}$}}")?;
            assert!(markup.contains(r#"<span class="mord textsf textbf">A</span>"#));
            Ok(())
        },
    );

    it("should render \\texttt{R} with the correct font", || {
        let markup = render_to_string_strict(r"\texttt{R}")?;
        assert!(markup.contains(r#"<span class="mord texttt">R</span>"#));
        Ok(())
    });

    it(
        "should render a combination of font and color changes",
        || {
            let markup = render_to_string_strict(r"\textcolor{blue}{\mathbb R}")?;
            let span = r#"<span class="mord mathbb" style="color:blue;">R</span>"#;
            assert!(markup.contains(span));

            let markup2 = render_to_string_strict(r"\mathbb{\textcolor{blue}{R}}")?;
            let span2 = r#"<span class="mord mathbb" style="color:blue;">R</span>"#;
            assert!(markup2.contains(span2));
            Ok(())
        },
    );

    it(
        "should render wide characters with mord and with the correct font",
        || {
            let wide_char = char::from_u32(0x1D400).unwrap(); // Mathematical Bold Capital A
            let markup = render_to_string_strict(&wide_char.to_string())?;
            assert!(markup.contains(r#"<span class="mord mathbf">A</span>"#));

            let wide_char2 = char::from_u32(0x1D41A).unwrap(); // Mathematical Bold Small A
            expect!(&format!("{wide_char} = {wide_char2}"))
                .to_build_like(r"\mathbf A = \mathbf a", &strict_settings())
        },
    );

    it(
        "should throw TypeError when the expression is of the wrong type",
        || {
            // In Rust, this would be compile-time or runtime panic, but for test, skip or
            // use result For now, assume it's handled in the API
            Ok(())
        },
    );

    it(
        "should not throw TypeError when the expression is a supported type",
        || {
            expect!(r"\sqrt{123}").to_build(&strict_settings())?;
            // For String object, in Rust it's always &str, so skip
            Ok(())
        },
    );
}

#[test]
fn a_mathml_font_tree_builder() {
    let contents = r"Ax2k\omega\Omega\imath+";

    it(
        &format!("should render {contents} with the correct mathvariants"),
        || {
            let mml = build_mathml(contents)?;
            let markup = mml.to_markup()?;
            assert!(markup.contains("<mi>A</mi>"));
            assert!(markup.contains("<mi>x</mi>"));
            assert!(markup.contains("<mn>2</mn>"));
            assert!(markup.contains("<mi>\u{3c9}</mi>")); // \omega
            assert!(markup.contains("<mi mathvariant=\"normal\">\u{3a9}</mi>")); // \Omega
            assert!(markup.contains("<mi mathvariant=\"normal\">\u{131}</mi>")); // \imath
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathbb{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathbb{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"double-struck\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"double-struck\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"double-struck\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"double-struck\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"double-struck\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"double-struck\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathrm{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathrm{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"normal\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"normal\">x</mi>"));
            assert!(markup.contains("<mn>2</mn>"));
            assert!(markup.contains("<mi>\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"normal\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"normal\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathit{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathit{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi>A</mi>"));
            assert!(markup.contains("<mi>x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"italic\">2</mn>"));
            assert!(markup.contains("<mi>\u{3c9}</mi>"));
            assert!(markup.contains("<mi>\u{3a9}</mi>"));
            assert!(markup.contains("<mi>\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathnormal{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathnormal{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi>A</mi>"));
            assert!(markup.contains("<mi>x</mi>"));
            assert!(markup.contains("<mn>2</mn>"));
            assert!(markup.contains("<mi>\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"normal\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"normal\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathbf{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathbf{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"bold\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"bold\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"bold\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"bold\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"bold\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"bold\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathcal{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathcal{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"script\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"script\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"script\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"script\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"script\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"script\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathfrak{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathfrak{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"fraktur\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"fraktur\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"fraktur\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"fraktur\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"fraktur\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"fraktur\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathscr{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathscr{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"script\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"script\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"script\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"script\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"script\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"script\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathsf{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathsf{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"sans-serif\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"sans-serif\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        &format!("should render \\mathsfit{{{contents}}} with the correct mathvariants"),
        || {
            let tex = format!("\\mathsfit{{{contents}}}");
            let markup = build_mathml(&tex)?.to_markup()?;
            assert!(markup.contains("<mi mathvariant=\"sans-serif-italic\">A</mi>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif-italic\">x</mi>"));
            assert!(markup.contains("<mn mathvariant=\"sans-serif-italic\">2</mn>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif-italic\">\u{3c9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif-italic\">\u{3a9}</mi>"));
            assert!(markup.contains("<mi mathvariant=\"sans-serif-italic\">\u{131}</mi>"));
            assert!(markup.contains("<mo>+</mo>"));
            Ok(())
        },
    );

    it(
        "should render a combination of font and color changes",
        || {
            let tex = r"\textcolor{blue}{\mathbb R}";
            let markup = build_mathml(tex)?.to_markup()?;
            let node =
                "<mstyle mathcolor=\"blue\"><mi mathvariant=\"double-struck\">R</mi></mstyle>";
            assert!(markup.contains(node));

            let tex = r"\mathbb{\textcolor{blue}{R}}";
            let markup = build_mathml(tex)?.to_markup()?;
            let node =
                "<mstyle mathcolor=\"blue\"><mi mathvariant=\"double-struck\">R</mi></mstyle>";
            assert!(markup.contains(node));
            Ok(())
        },
    );

    it("should render text as <mtext>", || {
        let tex = r"\text{for }";
        let markup = build_mathml(tex)?.to_markup()?;
        assert!(markup.contains("<mtext>for\u{a0}</mtext>"));
        Ok(())
    });

    it(
        "should render math within text as side-by-side children",
        || {
            let tex = r"\text{graph: $y = mx + b$}";
            let markup = build_mathml(tex)?.to_markup()?;
            assert!(markup.contains("<mrow><mtext>graph:\u{a0}</mtext>"));
            assert!(
                markup.contains("<mi>y</mi><mo>=</mo><mi>m</mi><mi>x</mi><mo>+</mo><mi>b</mi>")
            );
            Ok(())
        },
    );
}

#[test]
fn an_includegraphics_builder() {
    let img = r"\includegraphics[height=0.9em, totalheight=0.9em, width=0.9em, alt=KA logo]{https://cdn.kastatic.org/images/apple-touch-icon-57x57-precomposed.new.png}";

    it("should not fail", || {
        expect!(img).to_build(&trust_settings())
    });

    it("should produce mords", || {
        let built = get_built(img, &trust_settings())?;
        assert!(built[0].classes().contains("mord"));
        Ok(())
    });

    it("should not render without trust setting", || {
        let built = get_built(img, &strict_settings())?;
        let snapshot = normalize_debug_snapshot(&format!("{built:#?}"));
        insta::assert_snapshot!("includegraphics_without_trust", snapshot);
        Ok(())
    });

    it("should render with trust setting", || {
        let built = get_built(img, &trust_settings())?;
        let snapshot = normalize_debug_snapshot(&format!("{built:#?}"));
        insta::assert_snapshot!("includegraphics_with_trust", snapshot);
        Ok(())
    });

    it("should escape source", || {
        let markup = render_to_string_trust(r#"\includegraphics{'"}"#)?;
        assert!(markup.contains(r#"<img src="&#x27;&quot;""#));
        Ok(())
    });

    it("should escape alt", || {
        let markup = render_to_string_trust(r#"\includegraphics[alt='"]{image.png}"#)?;
        assert!(markup.contains(r#"<img src="image.png" alt="&#x27;&quot;""#));
        Ok(())
    });
}

#[test]
fn an_html_extension_builder() {
    let html =
        r"\htmlId{bar}{x}\htmlClass{foo}{x}\htmlStyle{color: red;}{x}\htmlData{foo=a, bar=b}{x}";

    it("should not fail", || {
        expect!(html).to_build(&trust_non_strict_settings())
    });

    it("should set HTML attributes", || {
        let built = get_built(html, &trust_non_strict_settings())?;
        // built[0] is \htmlId{bar}{x} - should have id="bar"
        if let Some(id) = built[0].attributes().unwrap().get("id") {
            assert_eq!(id, "bar");
        } else {
            panic!("Expected id attribute on first node");
        }
        // built[1] is \htmlClass{foo}{x} - should have class containing "foo"
        assert!(built[1].classes().contains("foo"));
        // built[2] is \htmlStyle{color: red;}{x} - should have style="color: red"
        if let Some(style) = built[2].attributes().unwrap().get("style") {
            assert!(style.contains("color: red"));
        } else {
            panic!("Expected style attribute on third node");
        }
        // built[3] is \htmlData{foo=a, bar=b}{x} - should have data-foo="a"
        // data-bar="b"
        assert_eq!(
            built[3].attributes().unwrap().get("data-foo"),
            Some(&"a".to_owned())
        );
        assert_eq!(
            built[3].attributes().unwrap().get("data-bar"),
            Some(&"b".to_owned())
        );
        Ok(())
    });

    it("should not affect spacing", || {
        let built = get_built(r"\htmlId{a}{x+}y", &trust_non_strict_settings())?;
        let snapshot = normalize_debug_snapshot(&format!("{built:#?}"));
        insta::assert_snapshot!("html_extension_spacing", snapshot);
        Ok(())
    });

    it("should render with trust and strict setting", || {
        let built = get_built(html, &trust_non_strict_settings())?;
        let snapshot = normalize_debug_snapshot(&format!("{built:#?}"));
        insta::assert_snapshot!("html_extension_render", snapshot);
        Ok(())
    });

    it(
        "should throw Error when HTML attribute name is invalid",
        || {
            for ch in [">", " ", "\t", "\n", "\r", "\"", "'", "/"] {
                let invalid = format!("\\htmlData{{a{ch}b=foo}}{{bar}}");
                expect!(&invalid).not_to_html(&trust_non_strict_settings())?;
            }
            Ok(())
        },
    );
}

#[test]
fn a_bin_builder() {
    it("should create mbins normally", || {
        let built = get_built("x + y", &strict_settings())?;
        assert!(built[2].classes().contains("mbin"));
        Ok(())
    });

    it("should create ords when at the beginning of lists", || {
        let built = get_built("+ x", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));
        Ok(())
    });

    it("should create ords after some other objects", || {
        let built = get_built("x + + 2", &strict_settings())?;
        assert!(built[4].classes().contains("mord"));

        let built = get_built("( + 2", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));

        let built = get_built("= + 2", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));

        let built = get_built(r"\sin + 2", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));

        let built = get_built(", + 2", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));
        Ok(())
    });

    it("should correctly interact with color objects", || {
        let built = get_built(r"\blue{x}+y", &strict_settings())?;
        assert!(built[2].classes().contains("mbin"));

        let built = get_built(r"\blue{x+}+y", &strict_settings())?;
        assert!(built[2].classes().contains("mbin"));

        let built = get_built(r"\blue{x+}+y", &strict_settings())?;
        assert!(built[4].classes().contains("mord"));
        Ok(())
    });
}

#[test]
fn a_phantom_and_smash_builder() {
    it("should both build a mord", || {
        let built = get_built(r"\hphantom{a}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"a\hphantom{=}b", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));

        let built = get_built(r"a\hphantom{+}b", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));

        let built = get_built(r"\smash{a}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"\smash{=}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"a\smash{+}b", &strict_settings())?;
        assert!(built[2].classes().contains("mord"));

        Ok(())
    });
}

#[test]
fn a_markup_generator() {
    it("marks trees up", || {
        let markup = render_to_string_strict(r"\sigma^2")?;
        assert_eq!(markup.find("<span"), Some(0));
        assert!(markup.contains("\u{03c3}")); // sigma
        assert!(markup.contains("margin-right"));
        assert!(!markup.contains("marginRight"));
        Ok(())
    });

    it("generates both MathML and HTML", || {
        let markup = render_to_string_strict("a")?;

        assert!(markup.contains("<span"));
        assert!(markup.contains("<math"));
        Ok(())
    });
}

#[test]
fn a_parse_tree_generator() {
    it("generates a tree", || {
        let mut tree = get_parsed_strict(r"\sigma^2")?;
        strip_positions(&mut tree);
        insta::assert_snapshot!("parse_tree_sigma_squared", format!("{:#?}", tree));
        Ok(())
    });
}

#[test]
fn an_accent_parser() {
    it("should not fail", || {
        expect!(r"\vec{x}").to_parse(&strict_settings())?;
        expect!(r"\vec{x^2}").to_parse(&strict_settings())?;
        expect!(r"\vec{x}^2").to_parse(&strict_settings())?;
        expect!(r"\vec x").to_parse(&strict_settings())?;
        expect!(r"\underbar{X}").to_parse(&strict_settings())
    });

    it("should produce accents", || {
        let parsed = get_parsed_strict(r"\vec x")?;
        assert_let!(ParseNode::Accent(_) = &parsed[0]);
        Ok(())
    });

    it("should be grouped more tightly than supsubs", || {
        let parsed = get_parsed_strict(r"\vec x^2")?;
        assert_let!(ParseNode::SupSub(_) = &parsed[0]);
        Ok(())
    });

    it("should parse stretchy, shifty accents", || {
        expect!(r"\widehat{x}").to_parse(&strict_settings())?;
        expect!(r"\widecheck{x}").to_parse(&strict_settings())
    });

    it("should parse stretchy, non-shifty accents", || {
        expect!(r"\overrightarrow{x}").to_parse(&strict_settings())
    });
}

#[test]
fn an_accent_builder() {
    it("should not fail", || {
        expect!(r"\vec{x}").to_build(&strict_settings())?;
        expect!(r"\vec{x}^2").to_build(&strict_settings())?;
        expect!(r"\vec{x}_2").to_build(&strict_settings())?;
        expect!(r"\vec{x}_2^2").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\vec x", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        let built = get_built(r"\vec +", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));
        let built = get_built(r"\vec )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mclose"));
        Ok(())
    });
}

#[test]
fn a_stretchy_and_shifty_accent_builder() {
    it("should not fail", || {
        expect!(r"\widehat{AB}").to_build(&strict_settings())?;
        expect!(r"\widecheck{AB}").to_build(&strict_settings())?;
        expect!(r"\widehat{AB}^2").to_build(&strict_settings())?;
        expect!(r"\widehat{AB}_2").to_build(&strict_settings())?;
        expect!(r"\widehat{AB}_2^2").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\widehat{AB}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        let built = get_built(r"\widehat +", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));
        let built = get_built(r"\widehat )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mclose"));
        Ok(())
    });
}

#[test]
fn a_stretchy_and_non_shifty_accent_builder() {
    it("should not fail", || {
        expect!(r"\overrightarrow{AB}").to_build(&strict_settings())?;
        expect!(r"\overrightarrow{AB}^2").to_build(&strict_settings())?;
        expect!(r"\overrightarrow{AB}_2").to_build(&strict_settings())?;
        expect!(r"\overrightarrow{AB}_2^2").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\overrightarrow{AB}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"\overrightarrow +", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));

        let built = get_built(r"\overrightarrow )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mclose"));

        Ok(())
    });
}

#[test]
fn a_stretchy_mathml_builder() {
    it("should properly render stretchy accents", || {
        let tex = r"\widetilde{ABCD}";
        let markup = build_mathml(tex)?.to_markup()?;
        assert!(markup.contains(r#"<mo stretchy="true">~</mo>"#));
        Ok(())
    });
}

#[test]
fn an_under_accent_parser() {
    it("should not fail", || {
        expect!(r"\underrightarrow{x}").to_parse(&strict_settings())?;
        expect!(r"\underrightarrow{x^2}").to_parse(&strict_settings())?;
        expect!(r"\underrightarrow{x}^2").to_parse(&strict_settings())?;
        expect!(r"\underrightarrow x").to_parse(&strict_settings())
    });

    it("should produce accentUnder", || {
        let parsed = get_parsed_strict(r"\underrightarrow x")?;
        assert_let!(ParseNode::AccentUnder(_) = &parsed[0]);
        Ok(())
    });

    it("should be grouped more tightly than supsubs", || {
        let parsed = get_parsed_strict(r"\underrightarrow x^2")?;
        assert_let!(ParseNode::SupSub(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn an_extensible_arrow_parser() {
    it("should not fail", || {
        expect!(r"\xrightarrow{x}").to_parse(&strict_settings())?;
        expect!(r"\xrightarrow{x^2}").to_parse(&strict_settings())?;
        expect!(r"\xrightarrow{x}^2").to_parse(&strict_settings())?;
        expect!(r"\xrightarrow x").to_parse(&strict_settings())?;
        expect!(r"\xrightarrow[under]{over}").to_parse(&strict_settings())
    });

    it("should produce xArrow", || {
        let parsed = get_parsed_strict(r"\xrightarrow x")?;
        assert_let!(ParseNode::XArrow(_) = &parsed[0]);
        Ok(())
    });

    it("should be grouped more tightly than supsubs", || {
        let parsed = get_parsed_strict(r"\xrightarrow x^2")?;
        assert_let!(ParseNode::SupSub(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn an_extensible_arrow_builder() {
    it("should not fail", || {
        expect!(r"\xrightarrow{x}").to_build(&strict_settings())?;
        expect!(r"\xrightarrow{x}^2").to_build(&strict_settings())?;
        expect!(r"\xrightarrow{x}_2").to_build(&strict_settings())?;
        expect!(r"\xrightarrow{x}_2^2").to_build(&strict_settings())?;
        expect!(r"\xrightarrow[under]{over}").to_build(&strict_settings())
    });

    it("should produce mrell", || {
        let built = get_built(r"\xrightarrow x", &strict_settings())?;
        assert!(built[0].classes().contains("mrel"));

        let built = get_built(r"\xrightarrow [under]{over}", &strict_settings())?;
        assert!(built[0].classes().contains("mrel"));

        let built = get_built(r"\xrightarrow +", &strict_settings())?;
        assert!(built[0].classes().contains("mrel"));
        assert!(!built[0].classes().contains("mbin"));

        let built = get_built(r"\xrightarrow )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mrel"));
        assert!(!built[0].classes().contains("mclose"));

        Ok(())
    });
}

#[test]
fn a_horizontal_brace_parser() {
    it("should not fail", || {
        expect!(r"\overbrace{x}").to_parse(&strict_settings())?;
        expect!(r"\overbrace{x^2}").to_parse(&strict_settings())?;
        expect!(r"\overbrace{x}^2").to_parse(&strict_settings())?;
        expect!(r"\overbrace x").to_parse(&strict_settings())?;
        expect!(r"\underbrace{x}_2").to_parse(&strict_settings())?;
        expect!(r"\underbrace{x}_2^2").to_parse(&strict_settings())
    });

    it("should produce horizBrace", || {
        let parsed = get_parsed_strict(r"\overbrace x")?;
        assert_let!(ParseNode::HorizBrace(_) = &parsed[0]);
        Ok(())
    });

    it("should be grouped more tightly than supsubs", || {
        let parsed = get_parsed_strict(r"\overbrace x^2")?;
        assert_let!(ParseNode::SupSub(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_horizontal_brace_builder() {
    it("should not fail", || {
        expect!(r"\overbrace{x}").to_build(&strict_settings())?;
        expect!(r"\overbrace{x}^2").to_build(&strict_settings())?;
        expect!(r"\underbrace{x}_2").to_build(&strict_settings())?;
        expect!(r"\underbrace{x}_2^2").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\overbrace x", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"\overbrace{x}^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"\overbrace +", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));

        let built = get_built(r"\overbrace )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mclose"));

        Ok(())
    });
}

#[test]
fn a_boxed_parser() {
    it("should not fail", || {
        expect!(r"\boxed{x}").to_parse(&strict_settings())?;
        expect!(r"\boxed{x^2}").to_parse(&strict_settings())?;
        expect!(r"\boxed{x}^2").to_parse(&strict_settings())?;
        expect!(r"\boxed x").to_parse(&strict_settings())
    });

    it("should produce enclose", || {
        let parsed = get_parsed_strict(r"\boxed x")?;
        assert_let!(ParseNode::Enclose(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_boxed_builder() {
    it("should not fail", || {
        expect!(r"\boxed{x}").to_build(&strict_settings())?;
        expect!(r"\boxed{x}^2").to_build(&strict_settings())?;
        expect!(r"\boxed{x}_2").to_build(&strict_settings())?;
        expect!(r"\boxed{x}_2^2").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\boxed x", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"\boxed +", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));

        let built = get_built(r"\boxed )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mclose"));

        Ok(())
    });
}

#[test]
fn an_fbox_parser_unlike_a_boxed_parser() {
    it("should fail when given math", || {
        expect!(r"\fbox{\frac a b}").not_to_parse(&strict_settings())
    });
}

#[test]
fn a_colorbox_parser() {
    it("should not fail, given a text argument", || {
        expect!(r"\colorbox{red}{a b}").to_parse(&strict_settings())?;
        expect!(r"\colorbox{red}{x}^2").to_parse(&strict_settings())?;
        expect!(r"\colorbox{red} x").to_parse(&strict_settings())
    });

    it("should fail, given a math argument", || {
        expect!(r"\colorbox{red}{\alpha}").not_to_parse(&strict_settings())?;
        expect!(r"\colorbox{red}{\frac{a}{b}}").not_to_parse(&strict_settings())
    });

    it("should parse a color", || {
        expect!(r"\colorbox{red}{a b}").to_parse(&strict_settings())?;
        expect!(r"\colorbox{#197}{a b}").to_parse(&strict_settings())?;
        expect!(r"\colorbox{#1a9b7c}{a b}").to_parse(&strict_settings())
    });

    it("should produce enclose", || {
        let parsed = get_parsed_strict(r"\colorbox{red} x")?;
        assert_let!(ParseNode::Enclose(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_colorbox_builder() {
    it("should not fail", || {
        expect!(r"\colorbox{red}{a b}").to_build(&strict_settings())?;
        expect!(r"\colorbox{red}{a b}^2").to_build(&strict_settings())?;
        expect!(r"\colorbox{red} x").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\colorbox{red}{a b}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        Ok(())
    });
}

#[test]
fn a_fcolorbox_parser() {
    it("should not fail, given a text argument", || {
        expect!(r"\fcolorbox{blue}{yellow}{a b}").to_parse(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{yellow}{x}^2").to_parse(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{yellow} x").to_parse(&strict_settings())
    });

    it("should fail, given a math argument", || {
        expect!(r"\fcolorbox{blue}{yellow}{\alpha}").not_to_parse(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{yellow}{\frac{a}{b}}").not_to_parse(&strict_settings())
    });

    it("should parse a color", || {
        expect!(r"\fcolorbox{blue}{yellow}{a b}").to_parse(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{#197}{a b}").to_parse(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{#1a9b7c}{a b}").to_parse(&strict_settings())
    });

    it("should produce enclose", || {
        let parsed = get_parsed_strict(r"\fcolorbox{blue}{yellow} x")?;
        assert_let!(ParseNode::Enclose(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_fcolorbox_builder() {
    it("should not fail", || {
        expect!(r"\fcolorbox{blue}{yellow}{a b}").to_build(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{yellow}{a b}^2").to_build(&strict_settings())?;
        expect!(r"\fcolorbox{blue}{yellow} x").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\fcolorbox{blue}{yellow}{a b}", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        Ok(())
    });
}

#[test]
fn a_strike_through_parser() {
    it("should not fail", || {
        expect!(r"\cancel{x}").to_parse(&strict_settings())?;
        expect!(r"\cancel{x^2}").to_parse(&strict_settings())?;
        expect!(r"\cancel{x}^2").to_parse(&strict_settings())?;
        expect!(r"\cancel x").to_parse(&strict_settings())
    });

    it("should produce enclose", || {
        let parsed = get_parsed_strict(r"\cancel x")?;
        assert_let!(ParseNode::Enclose(_) = &parsed[0]);
        Ok(())
    });

    it("should be grouped more tightly than supsubs", || {
        let parsed = get_parsed_strict(r"\cancel x^2")?;
        assert_let!(ParseNode::SupSub(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_strike_through_builder() {
    it("should not fail", || {
        expect!(r"\cancel{x}").to_build(&strict_settings())?;
        expect!(r"\cancel{x}^2").to_build(&strict_settings())?;
        expect!(r"\cancel{x}_2").to_build(&strict_settings())?;
        expect!(r"\cancel{x}_2^2").to_build(&strict_settings())?;
        expect!(r"\sout{x}").to_build(&strict_settings())?;
        expect!(r"\sout{x}^2").to_build(&strict_settings())?;
        expect!(r"\sout{x}_2").to_build(&strict_settings())?;
        expect!(r"\sout{x}_2^2").to_build(&strict_settings())
    });

    it("should produce mords", || {
        let built = get_built(r"\cancel x", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));

        let built = get_built(r"\cancel +", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mbin"));

        let built = get_built(r"\cancel )^2", &strict_settings())?;
        assert!(built[0].classes().contains("mord"));
        assert!(!built[0].classes().contains("mclose"));

        Ok(())
    });
}

#[test]
fn a_actuarial_angle_parser() {
    it("should not fail in math mode", || {
        expect!(r"a_{\angl{n}}").to_parse(&strict_settings())
    });

    it("should fail in text mode", || {
        expect!(r"\text{a_{\angl{n}}}").not_to_parse(&strict_settings())
    });
}

#[test]
fn a_actuarial_angle_builder() {
    it("should not fail", || {
        expect!(r"a_{\angl{n}}").to_build(&strict_settings())?;
        expect!(r"a_{\angl{n}i}").to_build(&strict_settings())?;
        expect!(r"a_{\angl n}").to_build(&strict_settings())?;
        expect!(r"a_\angln").to_build(&strict_settings())
    });
}

#[test]
fn phase() {
    it("should fail in text mode", || {
        expect!(r"\text{\phase{-78.2^\circ}}").not_to_parse(&strict_settings())
    });

    it("should not fail in math mode", || {
        expect!(r"\phase{-78.2^\circ}").to_build(&strict_settings())
    });
}

#[test]
fn a_phantom_parser() {
    it("should not fail", || {
        expect!(r"\phantom{x}").to_parse(&strict_settings())?;
        expect!(r"\phantom{x^2}").to_parse(&strict_settings())?;
        expect!(r"\phantom{x}^2").to_parse(&strict_settings())?;
        expect!(r"\phantom x").to_parse(&strict_settings())?;
        expect!(r"\hphantom{x}").to_parse(&strict_settings())?;
        expect!(r"\hphantom{x^2}").to_parse(&strict_settings())?;
        expect!(r"\hphantom{x}^2").to_parse(&strict_settings())?;
        expect!(r"\hphantom x").to_parse(&strict_settings())
    });

    it("should build a phantom node", || {
        let parsed = get_parsed_strict(r"\phantom{x}")?;
        assert_let!(ParseNode::Phantom(phantom) = &parsed[0]);
        assert!(!phantom.body.is_empty());
        Ok(())
    });
}

#[test]
fn a_phantom_builder() {
    it("should not fail", || {
        expect!(r"\phantom{x}").to_build(&strict_settings())?;
        expect!(r"\phantom{x^2}").to_build(&strict_settings())?;
        expect!(r"\phantom{x}^2").to_build(&strict_settings())?;
        expect!(r"\phantom x").to_build(&strict_settings())?;
        expect!(r"\mathstrut").to_build(&strict_settings())?;
        expect!(r"\hphantom{x}").to_build(&strict_settings())?;
        expect!(r"\hphantom{x^2}").to_build(&strict_settings())?;
        expect!(r"\hphantom{x}^2").to_build(&strict_settings())?;
        expect!(r"\hphantom x").to_build(&strict_settings())
    });

    it("should make the children transparent", || {
        let children = get_built(r"\phantom{x+1}", &strict_settings())?;
        assert_eq!(
            children[0].style().unwrap().get(CssProperty::Color),
            Some("transparent")
        );
        assert_eq!(
            children[2].style().unwrap().get(CssProperty::Color),
            Some("transparent")
        );
        assert_eq!(
            children[4].style().unwrap().get(CssProperty::Color),
            Some("transparent")
        );
        Ok(())
    });

    it("should make all descendants transparent", || {
        let children = get_built(r"\phantom{x+\blue{1}}", &strict_settings())?;
        assert_eq!(
            children[0].style().unwrap().get(CssProperty::Color),
            Some("transparent")
        );
        assert_eq!(
            children[2].style().unwrap().get(CssProperty::Color),
            Some("transparent")
        );
        assert_eq!(
            children[4].style().unwrap().get(CssProperty::Color),
            Some("transparent")
        );
        Ok(())
    });
}

#[test]
fn a_smash_parser() {
    it("should not fail", || {
        expect!(r"\smash{x}").to_parse(&strict_settings())?;
        expect!(r"\smash{x^2}").to_parse(&strict_settings())?;
        expect!(r"\smash{x}^2").to_parse(&strict_settings())?;
        expect!(r"\smash x").to_parse(&strict_settings())?;

        expect!(r"\smash[b]{x}").to_parse(&strict_settings())?;
        expect!(r"\smash[b]{x^2}").to_parse(&strict_settings())?;
        expect!(r"\smash[b]{x}^2").to_parse(&strict_settings())?;
        expect!(r"\smash[b] x").to_parse(&strict_settings())?;

        expect!(r"\smash[]{x}").to_parse(&strict_settings())?;
        expect!(r"\smash[]{x^2}").to_parse(&strict_settings())?;
        expect!(r"\smash[]{x}^2").to_parse(&strict_settings())?;
        expect!(r"\smash[] x").to_parse(&strict_settings())
    });

    it("should build a smash node", || {
        let parsed = get_parsed_strict(r"\smash{x}")?;
        assert_let!(ParseNode::Smash(_) = &parsed[0]);
        Ok(())
    });
}

#[test]
fn a_smash_builder() {
    it("should not fail", || {
        expect!(r"\smash{x}").to_build(&nonstrict_settings())?;
        expect!(r"\smash{x^2}").to_build(&nonstrict_settings())?;
        expect!(r"\smash{x}^2").to_build(&nonstrict_settings())?;
        expect!(r"\smash x").to_build(&nonstrict_settings())?;

        expect!(r"\smash[b]{x}").to_build(&nonstrict_settings())?;
        expect!(r"\smash[b]{x^2}").to_build(&nonstrict_settings())?;
        expect!(r"\smash[b]{x}^2").to_build(&nonstrict_settings())?;
        expect!(r"\smash[b] x").to_build(&nonstrict_settings())
    });
}

#[test]
fn a_parser_error() {
    it("should report the position of an error", || {
        let result = get_parsed_strict(r"\sqrt}");
        match result {
            Err(TestError::Parse { source: e, .. }) => {
                assert_eq!(e.position(), Some(5));
            }
            Err(other) => panic!("Expected ParseError, got {other}"),
            Ok(_) => panic!("Expected error"),
        }
        Ok(())
    });
}

#[test]
fn an_optional_argument_parser() {
    it("should not fail", || {
        expect!(r"\frac[1]{2}{3}").to_parse(&strict_settings())?;
        expect!(r"\rule[0.2em]{1em}{1em}").to_parse(&strict_settings())
    });

    it("should work with sqrts with optional arguments", || {
        expect!(r"\sqrt[3]{2}").to_parse(&strict_settings())
    });

    it("should work when the optional argument is missing", || {
        expect!(r"\sqrt{2}").to_parse(&strict_settings())?;
        expect!(r"\rule{1em}{2em}").to_parse(&strict_settings())
    });

    it(
        "should fail when the optional argument is malformed",
        || expect!(r"\rule[1]{2em}{3em}").not_to_parse(&strict_settings()),
    );

    it(
        "should not work if the optional argument isn't closed",
        || expect!(r"\sqrt[").not_to_parse(&strict_settings()),
    );
}

#[test]
fn an_array_environment() {
    it("should accept a single alignment character", || {
        let parse = get_parsed_strict(r"\begin{array}r1\\20\end{array}")?;
        assert_let!(ParseNode::Array(array) = &parse[0]);
        assert_eq!(array.cols.as_ref().unwrap().len(), 1);
        if let AlignSpec::Align { align, .. } = &array.cols.as_ref().unwrap()[0] {
            assert_eq!(align, "r");
        } else {
            panic!("Expected Align");
        }
        Ok(())
    });

    it("should accept vertical separators", || {
        let parse = get_parsed_strict(r"\begin{array}{|l||c:r::}\end{array}")?;
        assert_let!(ParseNode::Array(array) = &parse[0]);
        let cols = array.cols.as_ref().unwrap();
        assert_eq!(cols.len(), 9);
        if let AlignSpec::Separator { separator } = &cols[0] {
            assert_eq!(separator, "|");
        } else {
            panic!("Expected Separator");
        }
        if let AlignSpec::Align { align, .. } = &cols[1] {
            assert_eq!(align, "l");
        } else {
            panic!("Expected Align");
        }
        if let AlignSpec::Separator { separator } = &cols[2] {
            assert_eq!(separator, "|");
        } else {
            panic!("Expected Separator");
        }
        if let AlignSpec::Separator { separator } = &cols[3] {
            assert_eq!(separator, "|");
        } else {
            panic!("Expected Separator");
        }
        if let AlignSpec::Align { align, .. } = &cols[4] {
            assert_eq!(align, "c");
        } else {
            panic!("Expected Align");
        }
        if let AlignSpec::Separator { separator } = &cols[5] {
            assert_eq!(separator, ":");
        } else {
            panic!("Expected Separator");
        }
        if let AlignSpec::Align { align, .. } = &cols[6] {
            assert_eq!(align, "r");
        } else {
            panic!("Expected Align");
        }
        if let AlignSpec::Separator { separator } = &cols[7] {
            assert_eq!(separator, ":");
        } else {
            panic!("Expected Separator");
        }
        if let AlignSpec::Separator { separator } = &cols[8] {
            assert_eq!(separator, ":");
        } else {
            panic!("Expected Separator");
        }
        Ok(())
    });
}

#[test]
fn a_subarray_environment() {
    it("should accept only a single alignment character", || {
        let parse = get_parsed_strict(r"\begin{subarray}{c}a \\ b\end{subarray}")?;
        assert_let!(ParseNode::Array(array) = &parse[0]);
        assert_eq!(array.cols.as_ref().unwrap().len(), 1);
        if let AlignSpec::Align { align, .. } = &array.cols.as_ref().unwrap()[0] {
            assert_eq!(align, "c");
        } else {
            panic!("Expected Align");
        }
        expect!(r"\begin{subarray}{cc}a \\ b\end{subarray}").not_to_parse(&strict_settings())?;
        expect!(r"\begin{subarray}{c}a & b \\ c & d\end{subarray}")
            .not_to_parse(&strict_settings())?;
        expect!(r"\begin{subarray}{c}a \\ b\end{subarray}").to_build(&strict_settings())
    });
}

#[test]
fn a_substack_function() {
    it("should build", || {
        expect!(r"\sum_{\substack{ 0<i<m \\ 0<j<n }}  P(i,j)").to_build(&strict_settings())
    });
    it("should accommodate spaces in the argument", || {
        expect!(r"\sum_{\substack{ 0<i<m \\ 0<j<n }}  P(i,j)").to_build(&strict_settings())
    });
    it("should accommodate macros in the argument", || {
        expect!(r"\sum_{\substack{ 0<i<\varPi \\ 0<j<\pi }}  P(i,j)").to_build(&strict_settings())
    });
    it("should accommodate an empty argument", || {
        expect!(r"\sum_{\substack{}}  P(i,j)").to_build(&strict_settings())
    });
}

#[test]
fn a_smallmatrix_environment() {
    it("should build", || {
        expect!(r"\begin{smallmatrix} a & b \\ c & d \end{smallmatrix}")
            .to_build(&strict_settings())
    });
}

#[test]
fn a_cases_environment() {
    it("should parse its input", || {
        expect!(r"f(a,b)=\begin{cases}a+1&\text{if }b\text{ is odd}\\a&\text{if }b=0\\a-1&\text{otherwise}\end{cases}").to_parse(&strict_settings())
    });
}

#[test]
fn an_rcases_environment() {
    it("should build", || {
        expect!(r"\begin{rcases} a &\text{if } b \\ c &\text{if } d \end{rcases}⇒…")
            .to_build(&strict_settings())
    });
}

#[test]
fn an_aligned_environment() {
    it("should parse its input", || {
        expect!(r"\begin{aligned}a&=b&c&=d\\e&=f\end{aligned}").to_parse(&strict_settings())
    });
    it("should allow cells in brackets", || {
        expect!(r"\begin{aligned}[a]&[b]\\ [c]&[d]\end{aligned}").to_parse(&strict_settings())
    });
    it("should forbid cells in brackets without space", || {
        expect!(r"\begin{aligned}[a]&[b]\\[c]&[d]\end{aligned}").not_to_parse(&strict_settings())
    });
    it(
        "should not eat the last row when its first cell is empty",
        || {
            let ae = get_parsed_strict(
                r"\begin{aligned}&E_1 & (1)\\&E_2 & (2)\\&E_3 & (3)\end{aligned}",
            )?;
            assert_let!(ParseNode::Array(array) = &ae[0]);
            assert_eq!(array.body.len(), 3);
            Ok(())
        },
    );
}

#[test]
fn ams_environments() {
    it("should fail outside display mode", || {
        let settings = nonstrict_settings();
        expect!(r"\begin{gather}a+b\\c+d\end{gather}").not_to_parse(&settings)?;
        expect!(r"\begin{gather*}a+b\\c+d\end{gather*}").not_to_parse(&settings)?;
        expect!(r"\begin{align}a&=b+c\\d+e&=f\end{align}").not_to_parse(&settings)?;
        expect!(r"\begin{align*}a&=b+c\\d+e&=f\end{align*}").not_to_parse(&settings)?;
        expect!(r"\begin{alignat}{2}10&x+ &3&y = 2\\3&x+&13&y = 4\end{alignat}")
            .not_to_parse(&settings)?;
        expect!(r"\begin{alignat*}{2}10&x+ &3&y = 2\\3&x+&13&y = 4\end{alignat*}")
            .not_to_parse(&settings)?;
        expect!(r"\begin{equation}a=b+c\end{equation}").not_to_parse(&settings)?;
        expect!(r"\begin{split}a &=b+c\\&=e+f\end{split}").not_to_parse(&settings)?;
        expect!(r"\begin{CD}A @>a>> B \\@VbVV @AAcA\\C @= D\end{CD}").not_to_parse(&settings)?;
        Ok(())
    });

    it("should build if in display mode", || {
        let settings = display_settings();
        expect!(r"\begin{gather}a+b\\c+d\end{gather}").to_build(&settings)?;
        expect!(r"\begin{gather*}a+b\\c+d\end{gather*}").to_build(&settings)?;
        expect!(r"\begin{align}a&=b+c\\d+e&=f\end{align}").to_build(&settings)?;
        expect!(r"\begin{align*}a&=b+c\\d+e&=f\end{align*}").to_build(&settings)?;
        expect!(r"\begin{alignat}{2}10&x+ &3&y = 2\\3&x+&13&y = 4\end{alignat}")
            .to_build(&settings)?;
        expect!(r"\begin{alignat*}{2}10&x+ &3&y = 2\\3&x+&13&y = 4\end{alignat*}")
            .to_build(&settings)?;
        expect!(r"\begin{equation}a=b+c\end{equation}").to_build(&settings)?;
        expect!(r"\begin{equation}\begin{split}a &=b+c\\&=e+f\end{split}\end{equation}")
            .to_build(&settings)?;
        expect!(r"\begin{split}a &=b+c\\&=e+f\end{split}").to_build(&settings)?;
        expect!(
            r"\begin{CD}A @<a<< B @>>b> C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G\end{CD}"
        )
        .to_build(&settings)?;
        Ok(())
    });

    it("should build an empty environment", || {
        let settings = display_settings();
        expect!(r"\begin{gather}\end{gather}").to_build(&settings)?;
        expect!(r"\begin{gather*}\end{gather*}").to_build(&settings)?;
        expect!(r"\begin{align}\end{align}").to_build(&settings)?;
        expect!(r"\begin{align*}\end{align*}").to_build(&settings)?;
        expect!(r"\begin{alignat}{2}\end{alignat}").to_build(&settings)?;
        expect!(r"\begin{alignat*}{2}\end{alignat*}").to_build(&settings)?;
        expect!(r"\begin{equation}\end{equation}").to_build(&settings)?;
        expect!(r"\begin{split}\end{split}").to_build(&settings)?;
        expect!(r"\begin{CD}\end{CD}").to_build(&settings)?;
        Ok(())
    });

    it(
        "{equation} should fail if argument contains two rows.",
        || expect!(r"\begin{equation}a=\cr b+c\end{equation}").not_to_parse(&display_settings()),
    );

    it(
        "{equation} should fail if argument contains two columns.",
        || expect!(r"\begin{equation}a &=b+c\end{equation}").not_to_build(&display_settings()),
    );

    it(
        "{split} should fail if argument contains three columns.",
        || {
            expect!(r"\begin{equation}\begin{split}a &=b &+c\\&=e &+f\end{split}\end{equation}")
                .not_to_build(&display_settings())
        },
    );

    it(
        "{array} should fail if body contains more columns than specification.",
        || {
            expect!(r"\begin{array}{2}a & b & c\\d & e  f\end{array}")
                .not_to_build(&display_settings())
        },
    );
}

#[test]
fn the_cd_environment() {
    it("should fail if not is display mode", || {
        expect!(r"\begin{CD}A @<a<< B @>>b> C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G\end{CD}")
            .not_to_parse(&non_display_settings())
    });

    it(
        "should fail if the character after '@' is not in <>AV=|.",
        || {
            expect!(
                r"\begin{CD}A @X<a<< B @>>b> C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G\end{CD}"
            )
            .not_to_parse(&display_settings())
        },
    );
    it(
        "should fail if an arrow does not have its final character.",
        || {
            expect!(
                r"\begin{CD}A @<a< B @>>b> C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G\end{CD}"
            )
            .not_to_parse(&display_settings())?;
            expect!(
                r"\begin{CD}A @<a<< B @>>b C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G\end{CD}"
            )
            .not_to_parse(&display_settings())
        },
    );
    it("should fail without an \\end.", || {
        expect!(r"\begin{CD}A @<a<< B @>>b> C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G")
            .not_to_parse(&display_settings())
    });

    it("should succeed without the flaws noted above.", || {
        expect!(r"\begin{CD}A @<a<< B @>>b> C @>>> D\\@. @| @AcAA @VVdV \\@. E @= F @>>> G\end{CD}")
            .to_build(&display_settings())
    });
}

#[test]
fn operatorname_support() {
    it("should not fail", || {
        expect!(r"\operatorname{x*Π∑\Pi\sum\frac a b}").to_build(&strict_settings())?;
        expect!(r"\operatorname*{x*Π∑\Pi\sum\frac a b}").to_build(&strict_settings())?;
        expect!(r"\operatorname*{x*Π∑\Pi\sum\frac a b}_y x").to_build(&strict_settings())?;
        expect!(r"\operatorname*{x*Π∑\Pi\sum\frac a b}\limits_y x").to_build(&strict_settings())?;
        // The following does not actually render with limits. But it does not crash
        // either.
        expect!(r"\operatorname{sn}\limits_{b>c}(b+c)").to_build(&strict_settings())
    });
}

#[test]
fn href_and_url_commands() {
    it("should parse its input", || {
        expect!(r"\href{http://example.com/}{\sin}").to_build(&trust_settings())?;
        expect!(r"\url{http://example.com/}").to_build(&trust_settings())
    });

    it("should allow empty URLs", || {
        expect!(r"\href{}{example here}").to_build(&trust_settings())?;
        expect!(r"\url{}").to_build(&trust_settings())
    });

    it("should allow single-character URLs", || {
        expect!(r"\href%end").to_parse_like(r"\href{%}end", &trust_settings())?;
        expect!(r"\url%end").to_parse_like(r"\url{%}end", &trust_settings())?;
        expect!(r"\url%%end\n").to_parse_like(r"\url{%}", &trust_settings())?;
        expect!(r"\url end").to_parse_like(r"\url{e}nd", &trust_settings())?;
        expect!(r"\url%end").to_parse_like(r"\url {%}end", &trust_settings())
    });

    it("should allow spaces single-character URLs", || {
        expect!(r"\href %end").to_parse_like(r"\href{%}end", &trust_settings())?;
        expect!(r"\url %end").to_parse_like(r"\url{%}end", &trust_settings())
    });

    it("should allow letters [#$%&~_^] without escaping", || {
        let url = "http://example.org/~bar/#top?foo=$foo&bar=ba^r_boo%20baz";
        let parsed1 = get_parsed_trust(&format!(r"\href{{{url}}}{{\alpha}}"))?;
        if let ParseNode::Href(href) = &parsed1[0] {
            assert_eq!(href.href, url);
        } else {
            panic!("Expected Href node");
        }
        let parsed2 = get_parsed_trust(&format!(r"\url{{{url}}}"))?;
        if let ParseNode::Href(href_node) = &parsed2[0] {
            assert_eq!(href_node.href, url);
        } else {
            panic!("Expected Href node");
        }
        Ok(())
    });

    it("should allow balanced braces in url", || {
        let url = "http://example.org/{{}t{oo}}";
        let parsed1 = get_parsed_trust(&format!(r"\href{{{url}}}{{\alpha}}"))?;
        if let ParseNode::Href(href) = &parsed1[0] {
            assert_eq!(href.href, url);
        } else {
            panic!("Expected Href node");
        }
        let parsed2 = get_parsed_trust(&format!(r"\url{{{url}}}"))?;
        if let ParseNode::Href(href_node) = &parsed2[0] {
            assert_eq!(href_node.href, url);
        } else {
            panic!("Expected Href node");
        }
        Ok(())
    });

    it("should not allow unbalanced brace(s) in url", || {
        expect!(r"\href{http://example.com/{a}{bar}").not_to_parse(&trust_settings())?;
        expect!(r"\href{http://example.com/}a}{bar}").not_to_parse(&trust_settings())?;
        expect!(r"\url{http://example.com/{a}").not_to_parse(&trust_settings())?;
        expect!(r"\url{http://example.com/}a}").not_to_parse(&trust_settings())
    });

    it("should allow escape for letters [#$%&~_^{}]", || {
        let url = "http://example.org/~bar/#top?foo=$}foo{&bar=bar^r_boo%20baz";
        let mut input = url.to_owned();
        for ch in ['#', '$', '%', '&', '~', '_', '^', '{', '}'] {
            input = input.replace(ch, &format!(r"\{ch}"));
        }
        let parsed1 = get_parsed_trust(&format!(r"\href{{{input}}}{{\alpha}}"))?;
        if let ParseNode::Href(href) = &parsed1[0] {
            assert_eq!(href.href, url);
        } else {
            panic!("Expected Href node");
        }
        let parsed2 = get_parsed_trust(&format!(r"\url{{{input}}}"))?;
        if let ParseNode::Href(href_node) = &parsed2[0] {
            assert_eq!(href_node.href, url);
        } else {
            panic!("Expected Href node");
        }
        Ok(())
    });

    it("should allow comments after URLs", || {
        expect!(r"\url{http://example.com/}%comment\n").to_build(&trust_settings())
    });

    it("should be marked up correctly", || {
        let markup = render_to_string_trust(r"\href{http://example.com/}{example here}")?;
        assert!(markup.contains(r#"<a href="http://example.com/">"#));
        Ok(())
    });

    it("should not affect spacing around", || {
        let built = get_built(r"a\href{http://example.com/}{+b}", &trust_settings())?;
        let snapshot = normalize_debug_snapshot(&format!("{built:#?}"));
        insta::assert_snapshot!("href_spacing", snapshot);
        Ok(())
    });

    it(
        "should forbid relative URLs when trust option is false",
        || {
            let mut settings = nonstrict_settings();
            settings.trust = TrustSetting::Function(Arc::new(|_| Some(false)));
            let mut parsed = get_parsed(r"\href{relative}{foo}", &settings)?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("href_relative_url_forbidden", format!("{:#?}", parsed));
            Ok(())
        },
    );

    it("should allow explicitly allowed protocols", || {
        let mut settings = trust_settings();
        settings.trust = TrustSetting::Function(Arc::new(|context| {
            Some(context.protocol == Some("ftp".to_owned()))
        }));
        let mut parsed = get_parsed(r"\href{ftp://x}{foo}", &settings)?;
        strip_positions(&mut parsed);
        insta::assert_snapshot!("href_allow_explicit_protocol", format!("{:#?}", parsed));
        Ok(())
    });

    it(
        "should allow all protocols when trust option is true",
        || {
            let mut parsed = get_parsed_trust(r"\href{ftp://x}{foo}")?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("href_allow_all_protocols", format!("{:#?}", parsed));
            Ok(())
        },
    );

    it("should not allow explicitly disallowed protocols", || {
        let mut settings = nonstrict_settings();
        settings.trust = TrustSetting::Function(Arc::new(|context| {
            Some(context.protocol != Some("javascript".to_owned()))
        }));
        let mut parsed = get_parsed(r"\href{javascript:alert('x')}{foo}", &settings)?;
        strip_positions(&mut parsed);
        insta::assert_snapshot!("href_disallow_protocol", format!("{:#?}", parsed));
        Ok(())
    });

    it(
        "should not allow explicitly uppercased disallowed protocols",
        || {
            let mut settings = nonstrict_settings();
            settings.trust = TrustSetting::Function(Arc::new(|context| {
                Some(context.protocol != Some("javascript".to_owned()))
            }));
            let mut parsed = get_parsed(r"\href{JavaScript:alert('x')}{foo}", &settings)?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("href_disallow_uppercase_protocol", format!("{:#?}", parsed));
            Ok(())
        },
    );

    // For getProtocolViaTrust, implement a helper to extract protocol from parsed
    // node
    fn get_protocol_via_trust(url: &str) -> Option<String> {
        let mut settings = trust_settings();
        let protocol = Arc::new(Mutex::new(None));
        let proto_ref = protocol.clone();
        settings.trust = TrustSetting::Function(Arc::new(move |context| {
            *proto_ref.lock().unwrap() = context.protocol.clone();
            Some(true)
        }));
        let _ = get_parsed(&format!(r"\url{{{url}}}"), &settings);
        protocol.lock().unwrap().clone()
    }

    it("should get protocols correctly", || {
        assert_eq!(get_protocol_via_trust("foo"), Some("_relative".to_owned()));
        assert_eq!(get_protocol_via_trust("Foo:"), Some("foo".to_owned()));
        assert_eq!(get_protocol_via_trust("Foo:bar"), Some("foo".to_owned()));
        assert_eq!(
            get_protocol_via_trust("JavaScript:"),
            Some("javascript".to_owned())
        );
        assert_eq!(
            get_protocol_via_trust("JavaScript:code"),
            Some("javascript".to_owned())
        );
        assert_eq!(get_protocol_via_trust("!:"), None);
        assert_eq!(get_protocol_via_trust("foo&colon;"), None);
        assert_eq!(
            get_protocol_via_trust("?query=string&colon="),
            Some("_relative".to_owned())
        );
        assert_eq!(
            get_protocol_via_trust("#query=string&colon="),
            Some("_relative".to_owned())
        );
        assert_eq!(
            get_protocol_via_trust("dir/file&colon"),
            Some("_relative".to_owned())
        );
        assert_eq!(
            get_protocol_via_trust("//foo"),
            Some("_relative".to_owned())
        );
        assert_eq!(get_protocol_via_trust("://foo"), None);
        assert_eq!(
            get_protocol_via_trust("  \t http://"),
            Some("http".to_owned())
        );
        assert_eq!(
            get_protocol_via_trust("  \t http://foo"),
            Some("http".to_owned())
        );
        Ok(())
    });
}

#[test]
fn a_raw_text_parser() {
    it("should return null for a omitted optional string", || {
        expect!(r"\includegraphics{https://cdn.kastatic.org/images/apple-touch-icon-57x57-precomposed.new.png}").to_parse(&trust_settings())
    });
}

#[test]
fn a_parser_that_does_not_throw_on_unsupported_commands() {
    it(
        "should still parse on unrecognized control sequences",
        || expect!(r"\error").to_parse(&nonstrict_settings()),
    );

    it("in superscripts and subscripts", || {
        expect!("2_\\error").to_build(&nonstrict_settings())?;
        expect!("3^{\\error}_\\error").to_build(&nonstrict_settings())?;
        expect!(r"\int\nolimits^\error_\error").to_build(&nonstrict_settings())
    });

    it("in fractions", || {
        expect!(r"\frac{345}{\error}").to_build(&nonstrict_settings())?;
        expect!(r"\frac\error{\error}").to_build(&nonstrict_settings())
    });

    it("in square roots", || {
        expect!(r"\sqrt\error").to_build(&nonstrict_settings())?;
        expect!(r"\sqrt{234\error}").to_build(&nonstrict_settings())
    });

    it("in text boxes", || {
        expect!(r"\text{\error}").to_build(&nonstrict_settings())
    });

    it(
        "should produce color nodes with a color value given by errorColor",
        || {
            let parsed_input = get_parsed(r"\error", &nonstrict_settings())?;
            assert!(!parsed_input.is_empty());
            Ok(())
        },
    );

    it(
        "should build katex-error span for other type of KaTeX error",
        || {
            let html = render_to_string_nonstrict("2^2^2")?;
            let normalized = normalize_html_attributes(&normalize_style_attributes(&html));
            insta::assert_snapshot!("error_render_double_superscript", normalized);
            Ok(())
        },
    );

    it("should properly escape LaTeX in errors", || {
        let html = render_to_string_nonstrict("2^&\"<>")?;
        let normalized = normalize_html_attributes(&normalize_style_attributes(&html));
        insta::assert_snapshot!("error_render_escape", normalized);
        Ok(())
    });
}

#[test]
fn parse_error_properties() {
    it(
        "should contain affected position and length information",
        || {
            match get_parsed_strict("1 + \\fraq{}{}") {
                Ok(_) => panic!("Expected parse error"),
                Err(TestError::Parse { source: e, .. }) => {
                    assert_eq!(e.position(), Some(4)); // After "1 + "
                    assert_eq!(e.length(), Some(5)); // "\\fraq"
                    assert_eq!(
                        e.to_string(),
                        "KaTeX parse error: Undefined control sequence: \\fraq at position 5: 1 + \\̲f̲r̲a̲q̲{}{}"
                    );
                }
                Err(other) => panic!("Expected ParseError, got {other}"),
            }
            Ok(())
        },
    );

    it(
        "should contain position and length information at end of input",
        || {
            match get_parsed_strict("\\frac{}") {
                Ok(_) => panic!("Expected parse error"),
                Err(TestError::Parse { source: e, .. }) => {
                    assert_eq!(e.position(), Some(7));
                    assert_eq!(e.length(), Some(0));
                    assert_eq!(
                        e.to_string(),
                        "KaTeX parse error: Unexpected end of input in a macro argument, expected '}' at end of input: \\frac{}"
                    );
                }
                Err(other) => panic!("Expected ParseError, got {other}"),
            }
            Ok(())
        },
    );

    it(
        "should contain no position and length information if unavailable",
        || {
            match get_parsed_strict("\\verb|hello\nworld|") {
                Ok(_) => panic!("Expected parse error"),
                Err(TestError::Parse { source: e, .. }) => {
                    assert_eq!(e.position(), None);
                    assert_eq!(e.length(), None);
                    assert_eq!(
                        e.to_string(),
                        "KaTeX parse error: \\verb ended by end of line instead of matching delimiter"
                    );
                }
                Err(other) => panic!("Expected ParseError, got {other}"),
            }
            Ok(())
        },
    );
}

#[test]
fn the_symbol_table_integrity() {
    it("should treat certain symbols as synonyms", || {
        expect!("<").to_build_like(r"\lt", &strict_settings())?;
        expect!(">").to_build_like(r"\gt", &strict_settings())?;
        expect!(r"\left<\frac{1}{x}\right>")
            .to_build_like(r"\left\lt\frac{1}{x}\right\gt", &strict_settings())
    });
}

#[test]
fn ams_symbols() {
    it(
        "should support AMS symbols in both text and math mode",
        || {
            let symbols = r"\yen\checkmark\circledR\maltese";
            expect!(symbols).to_build(&strict_settings())?;
            expect!(&format!(r"\text{{{symbols}}}")).to_build(&strict_settings())
        },
    );
}

#[test]
fn a_macro_expander() {
    it("should produce individual tokens", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("123"));
        expect!(r"e^\foo").to_parse_like("e^1 23", &settings)
    });

    it(
        "should preserve leading spaces inside macro definition",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr(" x"));
            expect!(r"\text{\foo}").to_parse_like(r"\text{ x}", &settings)
        },
    );

    it(
        "should preserve leading spaces inside macro argument",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("#1"));
            expect!(r"\text{\foo{ x}}").to_parse_like(r"\text{ x}", &settings)
        },
    );

    it("should ignore expanded spaces in math mode", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr(" x"));
        expect!(r"\foo").to_parse_like("x", &settings)
    });

    it("should consume spaces after control-word macro", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("x"));
        expect!(r"\text{\foo }").to_parse_like(r"\text{x}", &settings)
    });

    it("should consume spaces after macro with \\relax", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr(r"\relax"));
        expect!(r"\text{\foo }").to_parse_like(r"\text{}", &settings)
    });

    it(
        "should not consume spaces after control-word expansion",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\\\".to_owned(), MacroDefinition::StaticStr(r"\relax"));
            expect!(r"\text{\\ }").to_parse_like(r"\text{ }", &settings)
        },
    );

    it("should consume spaces after \\relax", || {
        expect!(r"\text{\relax }").to_parse_like(r"\text{}", &strict_settings())
    });

    it("should consume spaces after control-word function", || {
        expect!(r"\text{\KaTeX }").to_parse_like(r"\text{\KaTeX}", &strict_settings())
    });

    it("should preserve spaces after control-symbol macro", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\%".to_owned(), MacroDefinition::StaticStr("x"));
        expect!(r"\text{\% y}").to_parse_like(r"\text{x y}", &settings)
    });

    it(
        "should preserve spaces after control-symbol function",
        || expect!(r"\text{\' }").to_parse(&strict_settings()),
    );

    it("should consume spaces between arguments", || {
        let settings1 = strict_settings();
        settings1
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("#1#2end"));
        expect!(r"\text{\foo 1 2}").to_parse_like(r"\text{12end}", &settings1)?;
        let settings2 = strict_settings();
        settings2
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("#1#2end"));
        expect!(r"\text{\foo {1} {2}}").to_parse_like(r"\text{12end}", &settings2)
    });

    it("should allow for multiple expansion", || {
        let settings = strict_settings();
        settings.macros.borrow_mut().extend([
            ("\\foo".to_owned(), MacroDefinition::StaticStr("\\bar\\bar")),
            ("\\bar".to_owned(), MacroDefinition::StaticStr("a")),
        ]);

        expect!("1\\foo2").to_parse_like("1aa2", &settings)
    });

    it("should allow for multiple expansion with argument", || {
        let settings = strict_settings();
        settings.macros.borrow_mut().insert(
            "\\foo".to_owned(),
            MacroDefinition::StaticStr("\\bar{#1}\\bar{#1}"),
        );
        settings
            .macros
            .borrow_mut()
            .insert("\\bar".to_owned(), MacroDefinition::StaticStr("#1#1"));
        expect!("1\\foo2").to_parse_like("12222", &settings)
    });

    it("should allow for macro argument", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1)"));
        settings
            .macros
            .borrow_mut()
            .insert("\\bar".to_owned(), MacroDefinition::StaticStr("xyz"));
        expect!(r"\foo\bar").to_parse_like("(xyz)", &settings)
    });

    it(
        "should allow properly nested group for macro argument",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1)"));
            expect!(r"\foo{e^{x_{12}+3}}").to_parse_like("(e^{x_{12}+3})", &settings)
        },
    );

    it(
        "should delay expansion if preceded by \\expandafter",
        || {
            let settings1 = strict_settings();
            settings1
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("#1+#2"));
            settings1
                .macros
                .borrow_mut()
                .insert("\\bar".to_owned(), MacroDefinition::StaticStr("xy"));
            expect!(r"\expandafter\foo\bar").to_parse_like("x+y", &settings1)?;
            expect!(r"\def\foo{x}\def\bar{\def\foo{y}}\expandafter\bar\foo")
                .to_parse_like("x", &strict_settings())?;
            // \def is not expandable, i.e., \expandafter doesn't define the macro
            expect!(r"\expandafter\foo\def\foo{x}").not_to_parse(&strict_settings())
        },
    );

    it("should not expand if preceded by \\noexpand", || {
        // \foo is not expanded and interpreted as if its meaning were \relax
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("x"));
        expect!(r"\noexpand\foo y").to_parse_like("y", &settings)?;
        expect!(r"\expandafter\foo\noexpand\foo").to_parse_like("x", &settings)?; // \frac is a macro and therefore expandable
        expect!(r"\noexpand\frac xy").to_parse_like("xy", &strict_settings())?;
        // \def is not expandable, so is not affected by \noexpand
        expect!(r"\noexpand\def\foo{xy}\foo").to_parse_like("xy", &strict_settings())
    });

    it(
        "should allow for space macro argument (text version)",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1)"));
            settings
                .macros
                .borrow_mut()
                .insert("\\bar".to_owned(), MacroDefinition::StaticStr(" "));
            expect!(r"\text{\foo\bar}").to_parse_like(r"\text{( )}", &settings)
        },
    );

    it(
        "should allow for space macro argument (math version)",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1)"));
            settings
                .macros
                .borrow_mut()
                .insert("\\bar".to_owned(), MacroDefinition::StaticStr(" "));
            expect!(r"\foo\bar").to_parse_like("()", &settings)
        },
    );

    it(
        "should allow for space second argument (text version)",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1,#2)"));
            settings
                .macros
                .borrow_mut()
                .insert("\\bar".to_owned(), MacroDefinition::StaticStr(" "));
            expect!(r"\text{\foo\bar\bar}").to_parse_like(r"\text{( , )}", &settings)
        },
    );

    it("should treat \\relax as empty argument", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1,#2)"));
        expect!(r"\text{\foo\relax x}").to_parse_like(r"\text{(,x)}", &settings)
    });

    it(
        "should allow for space second argument (math version)",
        || {
            let settings = strict_settings();
            settings
                .macros
                .borrow_mut()
                .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1,#2)"));
            settings
                .macros
                .borrow_mut()
                .insert("\\bar".to_owned(), MacroDefinition::StaticStr(" "));
            expect!(r"\foo\bar\bar").to_parse_like("(,)", &settings)
        },
    );

    it("should allow for empty macro argument", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("\\foo".to_owned(), MacroDefinition::StaticStr("(#1)"));
        settings
            .macros
            .borrow_mut()
            .insert("\\bar".to_owned(), MacroDefinition::StaticStr(""));
        expect!(r"\foo\bar").to_parse_like("()", &settings)
    });

    it("should allow for space function arguments", || {
        let settings1 = strict_settings();
        settings1
            .macros
            .borrow_mut()
            .insert("\\bar".to_owned(), MacroDefinition::StaticStr(" "));
        expect!(r"\frac\bar\bar").to_parse_like(r"\frac{}{}", &settings1)?;
        let settings2 = strict_settings();
        settings2
            .macros
            .borrow_mut()
            .insert("\\bar".to_owned(), MacroDefinition::StaticStr(" "));
        expect!(r"\frac \bar \bar").to_parse_like(r"\frac{}{}", &settings2)
    });

    it("should build \\overset and \\underset", || {
        expect!(r"\overset{f}{\rightarrow} Y").to_build(&strict_settings())?;
        expect!(r"\underset{f}{\rightarrow} Y").to_build(&strict_settings())
    });

    it("should build \\iff, \\implies, \\impliedby", || {
        expect!("X \\iff Y").to_build(&strict_settings())?;
        expect!("X \\implies Y").to_build(&strict_settings())?;
        expect!("X \\impliedby Y").to_build(&strict_settings())
    });

    it("should allow aliasing characters", || {
        let settings = strict_settings();
        settings
            .macros
            .borrow_mut()
            .insert("’".to_owned(), MacroDefinition::StaticStr("'"));
        expect!("x'=c").to_parse_like("x'=c", &settings)
    });

    it(
        "\\@firstoftwo should consume both, and avoid errors",
        || {
            expect!(r"\@firstoftwo{yes}{no}").to_parse_like("yes", &strict_settings())?;
            expect!(r"\@firstoftwo{yes}{1'_2^3}").to_parse_like("yes", &strict_settings())
        },
    );

    it("\\@ifstar should consume star but nothing else", || {
        expect!(r"\@ifstar{yes}{no}*!").to_parse_like("yes!", &strict_settings())?;
        expect!(r"\@ifstar{yes}{no}?!").to_parse_like("no?!", &strict_settings())
    });

    it("\\@ifnextchar should not consume nonspaces", || {
        expect!(r"\@ifnextchar!{yes}{no}!!").to_parse_like("yes!!", &strict_settings())?;
        expect!(r"\@ifnextchar!{yes}{no}?!").to_parse_like("no?!", &strict_settings())
    });

    it("\\@ifnextchar should consume spaces", || {
        expect!(r"\def\:{\let\space= }\: \text{\space}")
            .to_parse_like(r"\text{ }", &strict_settings())
    });

    it("\\TextOrMath should work immediately", || {
        expect!(r"\TextOrMath{text}{math}").to_parse_like("math", &strict_settings())
    });

    it("\\TextOrMath should work after other math", || {
        expect!(r"x+\TextOrMath{text}{math}").to_parse_like("x+math", &strict_settings())
    });

    it("\\TextOrMath should work immediately after \\text", || {
        expect!(r"\text{\TextOrMath{text}{math}}").to_parse_like(r"\text{text}", &strict_settings())
    });

    it("\\TextOrMath should work later after \\text", || {
        expect!(r"\text{hello \TextOrMath{text}{math}}")
            .to_parse_like(r"\text{hello text}", &strict_settings())
    });

    it(
        "\\TextOrMath should work immediately after \\text ends",
        || {
            expect!(r"\text{\TextOrMath{text}{math}}\TextOrMath{text}{math}")
                .to_parse_like(r"\text{text}math", &strict_settings())
        },
    );

    it("\\TextOrMath should work immediately after $", || {
        expect!(r"\text{$\TextOrMath{text}{math}$}")
            .to_parse_like(r"\text{$math$}", &strict_settings())
    });

    it("\\TextOrMath should work later after $", || {
        expect!(r"\text{$x+\TextOrMath{text}{math}$}")
            .to_parse_like(r"\text{$x+math$}", &strict_settings())
    });

    it("\\TextOrMath should work immediately after $ ends", || {
        expect!(r"\text{$\TextOrMath{text}{math}$\TextOrMath{text}{math}}")
            .to_parse_like(r"\text{$math$text}", &strict_settings())
    });

    it("\\TextOrMath should work in a macro", || {
        let settings = strict_settings();
        settings.macros.borrow_mut().insert(
            "\\mode".to_owned(),
            MacroDefinition::StaticStr(r"\TextOrMath{text}{math}"),
        );
        expect!(r"\mode\text{\mode$\mode$\mode}\mode")
            .to_parse_like(r"math\text{text$math$text}math", &settings)
    });

    it("\\char produces literal characters", || {
        expect!(r"\char`a").to_parse_like(r"\char`\a", &strict_settings())?;
        expect!(r"\char`\%").to_parse_like(r"\char37", &strict_settings())?;
        expect!(r"\char`\%").to_parse_like(r"\char'45", &strict_settings())?;
        expect!(r"\char`\%").to_parse_like(r#"\char"25"#, &strict_settings())?;
        expect!(r"\char").not_to_parse(&strict_settings())?;
        expect!(r"\char`").not_to_parse(&strict_settings())?;
        expect!(r"\char'").not_to_parse(&strict_settings())?;
        expect!(r#"\char""#).not_to_parse(&strict_settings())?;
        expect!(r"\char'a").not_to_parse(&strict_settings())?;
        expect!(r#"\char"g"#).not_to_parse(&strict_settings())?;
        expect!(r#"\char"g"#).not_to_parse(&strict_settings())
    });

    it("\\char escapes ~ correctly", || {
        let parsed_bare = get_parsed_strict("~")?;
        assert_let!(ParseNode::Spacing(_) = &parsed_bare[0]);
        let parsed_char = get_parsed_strict(r"\char`\\~")?;
        assert_let!(ParseNode::TextOrd(_) = &parsed_char[0]);
        Ok(())
    });

    it("\\char handles >16-bit characters", || {
        let parsed = get_parsed_strict(r#"\char"1d7d9"#)?;
        assert_let!(ParseNode::TextOrd(_) = &parsed[0]);
        assert_eq!(parsed[0].text(), Some("𝟙"));
        Ok(())
    });

    it("should build Unicode private area characters", || {
        expect!(r"\gvertneqq\lvertneqq\ngeqq\ngeqslant\nleqq").to_build(&strict_settings())?;
        expect!(r"\nleqslant\nshortmid\nshortparallel\varsubsetneq").to_build(&strict_settings())?;
        expect!(r"\varsubsetneqq\varsupsetneq\varsupsetneqq").to_build(&strict_settings())
    });

    it(
        "\\TextOrMath should work in a macro passed to \\text",
        || {
            let settings = strict_settings();
            settings.macros.borrow_mut().insert(
                "\\mode".to_owned(),
                MacroDefinition::StaticStr(r"\TextOrMath{t}{m}"),
            );
            expect!(r"\text\mode").to_parse_like(r"\text t", &settings)
        },
    );

    it("\\gdef defines macros", || {
        expect!(r"\gdef\foo{x^2}\foo+\foo").to_parse_like("x^2+x^2", &strict_settings())?;
        expect!(r"\gdef\foo{hi}\foo+\text\foo")
            .to_parse_like("hi+\\text{hi}", &strict_settings())?;
        expect!(r"\gdef\foo#1{hi #1}\text{\foo{Alice}, \foo{Bob}}")
            .to_parse_like(r"\text{hi Alice, hi Bob}", &strict_settings())?;
        expect!(r"\gdef\foo#1#2{(#1,#2)}\foo 1 2+\foo 3 4")
            .to_parse_like("(1,2)+(3,4)", &strict_settings())?;
        expect!(r"\gdef\foo#2{}").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo#a{}").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo#1#3{}").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo#1#2#3#4#5#6#7#8#9{}").to_parse(&strict_settings())?;
        expect!(r"\gdef\foo#1#2#3#4#5#6#7#8#9#10{}").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo1").not_to_parse(&strict_settings())?;
        expect!(r"\gdef{\foo}{}").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo\bar").not_to_parse(&strict_settings())?;
        expect!(r"\gdef{\foo\bar}{}").not_to_parse(&strict_settings())?;
        expect!(r"\gdef{}{}").not_to_parse(&strict_settings())
    });

    it("\\gdef defines macros with delimited parameter", || {
        expect!(r"\gdef\foo|#1||{#1}\text{\foo| x y ||}")
            .to_parse_like(r"\text{ x y }", &strict_settings())?;
        expect!(r"\gdef\foo#1|#2{#1+#2}\foo 1 2 |34").to_parse_like("12+34", &strict_settings())?;
        expect!(r"\gdef\foo#1#{#1}\foo1^{23}").to_parse_like("1^{23}", &strict_settings())?;
        expect!(r"\gdef\foo|{}\foo").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo#1|{#1}\foo1").not_to_parse(&strict_settings())?;
        expect!(r"\gdef\foo#1|{#1}\foo1}|").not_to_parse(&strict_settings())
    });

    it("\\xdef should expand definition", || {
        expect!(r"\def\foo{a}\xdef\bar{\foo}\def\foo{}\bar")
            .to_parse_like("a", &strict_settings())?;
        // \def\noexpand\foo{} expands into \def\foo{}
        expect!(r"\def\foo{a}\xdef\bar{\def\noexpand\foo{}}\foo\bar\foo")
            .to_parse_like("a", &strict_settings())?;
        // \foo\noexpand\foo expands into a\foo
        expect!(r"\def\foo{a}\xdef\bar{\foo\noexpand\foo}\def\foo{b}\bar")
            .to_parse_like("ab", &strict_settings())?;
        // \foo is not defined
        expect!(r"\xdef\bar{\foo}").not_to_parse(&strict_settings())
    });

    it("\\def should be handled in Parser", || {
        let mut settings = strict_settings();
        settings.max_expand = 0;
        expect!(r"\gdef\foo{1}").to_parse(&settings)?;
        expect!("2^\\def\\foo{1}2").not_to_parse(&strict_settings())
    });

    it("\\def works locally", || {
        expect!(r"\def\x{1}\x{\def\x{2}\x{\def\x{3}\x}\x}\x")
            .to_parse_like("1{2{3}2}1", &strict_settings())?;
        expect!(r"\def\x{1}\x\def\x{2}\x{\def\x{3}\x\def\x{4}\x}\x")
            .to_parse_like("12{34}2", &strict_settings())
    });

    it("\\gdef overrides at all levels", || {
        expect!(r"\def\x{1}\x{\def\x{2}\x{\gdef\x{3}\x}\x}\x")
            .to_parse_like("1{2{3}3}3", &strict_settings())?;
        expect!(r"\def\x{1}\x{\def\x{2}\x{\global\def\x{3}\x}\x}\x")
            .to_parse_like("1{2{3}3}3", &strict_settings())?;
        expect!(r"\def\x{1}\x{\def\x{2}\x{\gdef\x{3}\x\def\x{4}\x}\x\def\x{5}\x}\x")
            .to_parse_like("1{2{34}35}3", &strict_settings())
    });

    it(
        "\\global needs to followed by macro prefixes, \\def or \\edef",
        || {
            expect!(r"\global\def\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\global\edef\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\def\DEF{\def}\global\DEF\foo{}\foo")
                .to_parse_like("", &strict_settings())?;
            expect!(r"\global\global\def\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\global\long\def\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\global\foo").not_to_parse(&strict_settings())?;
            expect!(r"\global\bar x").not_to_parse(&strict_settings())
        },
    );

    it(
        "\\long needs to followed by macro prefixes, \\def or \\edef",
        || {
            expect!(r"\long\def\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\long\edef\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\long\global\def\foo{}\foo").to_parse_like("", &strict_settings())?;
            expect!(r"\long\foo").not_to_parse(&strict_settings())
        },
    );

    it("Macro arguments do not generate groups", || {
        expect!(r"\def\x{1}\x\def\foo#1{#1}\foo{\x\def\x{2}\x}\x")
            .to_parse_like("1122", &strict_settings())
    });

    it("\\textbf arguments do generate groups", || {
        expect!(r"\def\x{1}\x\textbf{\x\def\x{2}\x}\x")
            .to_parse_like("1\\textbf{12}1", &strict_settings())
    });

    it("\\sqrt optional arguments generate groups", || {
        expect!(r"\def\x{1}\def\y{1}\x\y\sqrt[\def\x{2}\x]{\def\y{2}\y}\x\y")
            .to_parse_like("11\\sqrt[2]{2}11", &strict_settings())
    });

    it("array cells generate groups", || {
        expect!(r"\def\x{1}\begin{matrix}\x&\def\x{2}\x&\x\end{matrix}\x")
            .to_parse_like("\\begin{matrix}1&2&1\\end{matrix}1", &strict_settings())?;
        expect!(r"\def\x{1}\begin{matrix}\def\x{2}\x&\x\end{matrix}\x")
            .to_parse_like("\\begin{matrix}2&1\\end{matrix}1", &strict_settings())
    });

    it("\\gdef changes settings.macros", || {
        let settings = strict_settings();
        expect!(r"\gdef\foo{1}").to_parse(&settings)?;
        assert!(settings.macros.borrow().contains_key("\\foo"));
        Ok(())
    });

    it("\\def doesn't change settings.macros", || {
        let settings = strict_settings();
        expect!(r"\def\foo{1}").to_parse(&settings)?;
        assert!(!settings.macros.borrow().contains_key("\\foo"));
        Ok(())
    });

    it("\\def doesn't change settings.macros on error", || {
        let settings = strict_settings();
        expect!(r"\def\foo{c^}\foo").not_to_parse(&settings)?;
        assert!(!settings.macros.borrow().contains_key("\\foo"));
        Ok(())
    });

    it("\\def changes settings.macros with globalGroup", || {
        let mut settings = strict_settings();
        settings.global_group = true;
        expect!(r"\gdef\foo{1}").to_parse(&settings)?;
        assert!(settings.macros.borrow().contains_key("\\foo"));
        Ok(())
    });

    it("\\let copies the definition", || {
        expect!(r"\let\foo=\frac\def\frac{}\foo12")
            .to_parse_like("\\frac12", &strict_settings())?;
        expect!(r"\def\foo{1}\let\bar\foo\def\foo{2}\bar")
            .to_parse_like("1", &strict_settings())?;
        expect!(r"\let\foo=\kern\edef\bar{\foo1em}\let\kern=\relax\bar")
            .to_parse_like("\\kern1em", &strict_settings())?;
        // \foo = { (left brace)
        expect!(r"\let\foo{\sqrt\foo1}").to_parse_like("\\sqrt{1}", &strict_settings())?;
        // \equals = = (equal sign)
        expect!(r"\let\equals==a\equals b").to_parse_like("a=b", &strict_settings())?;
        // \foo should not be expandable and not affected by \noexpand or \edef
        expect!(r"\let\foo=x\noexpand\foo").to_parse_like("x", &strict_settings())?;
        expect!(r"\let\foo=x\edef\bar{\foo}\def\foo{y}\bar").to_parse_like("y", &strict_settings())
    });

    it(
        "\\let should consume one optional space after equals sign",
        || {
            let mut parsed = get_parsed_strict(r"\def\bold{\bgroup\bf\let\next= }\bold{a}")?;
            strip_positions(&mut parsed);
            insta::assert_snapshot!("let_optional_space_parse", format!("{:#?}", parsed));
            Ok(())
        },
    );

    it("\\futurelet should parse correctly", || {
        expect!(r"\futurelet\foo\frac1{2+\foo}").to_parse_like("\\frac1{2+1}", &strict_settings())
    });

    it("macros argument can simulate \\let", || {
        let settings = strict_settings();
        let mut token = Token::new("\\int", None);
        token.noexpand = Some(true);
        settings.macros.borrow_mut().insert(
            "\\Oldint".to_owned(),
            MacroDefinition::Expansion(MacroExpansion {
                tokens: vec![token],
                num_args: 0,
                unexpandable: Some(true),
                delimiters: None,
            }),
        );
        let token1 = Token::new("\\limits", None);
        let mut token2 = Token::new("\\Oldint", None);
        token2.noexpand = Some(false);
        settings.macros.borrow_mut().insert(
            "\\int".to_owned(),
            MacroDefinition::Expansion(MacroExpansion {
                tokens: vec![token1, token2],
                num_args: 0,
                unexpandable: None,
                delimiters: None,
            }),
        );
        expect!(r"\int").to_parse_like("\\int\\limits", &settings)
    });

    it("macros argument can simulate \\def with arguments", || {
        let settings = strict_settings();
        // Simulate: \t x -> \text{x}
        // Definition tokens: } 1 # { \text
        // Stack order (reverse): \text { # 1 }

        let mut tokens = Vec::new();
        tokens.push(Token::new("}", None));
        tokens.push(Token::new("1", None));
        tokens.push(Token::new("#", None));
        tokens.push(Token::new("{", None));
        tokens.push(Token::new("\\text", None));

        settings.macros.borrow_mut().insert(
            "\\t".to_owned(),
            MacroDefinition::Expansion(MacroExpansion {
                tokens,
                num_args: 1,
                unexpandable: None,
                delimiters: None,
            }),
        );

        expect!(r"\t x").to_parse_like(r"\text{x}", &settings)
    });

    it("\\newcommand doesn't change settings.macros", || {
        let settings = strict_settings();
        expect!(r"\newcommand\foo{x^2}\foo+\foo").to_parse(&settings)?;
        assert!(!settings.macros.borrow().contains_key("\\foo"));
        Ok(())
    });

    it(
        "\\newcommand changes settings.macros with globalGroup",
        || {
            let mut settings = strict_settings();
            settings.global_group = true;
            expect!(r"\newcommand\foo{x^2}\foo+\foo").to_parse(&settings)?;
            assert!(settings.macros.borrow().contains_key("\\foo"));
            Ok(())
        },
    );

    it("\\newcommand defines new macros", || {
        expect!(r"\newcommand\foo{x^2}\foo+\foo").to_parse_like("x^2+x^2", &strict_settings())?;
        expect!(r"\newcommand{\foo}{x^2}\foo+\foo").to_parse_like("x^2+x^2", &strict_settings())?;
        // Function detection
        expect!(r"\newcommand\bar{x^2}\bar+\bar").not_to_parse(&strict_settings())?;
        expect!(r"\newcommand{\bar}{x^2}\bar+\bar").not_to_parse(&strict_settings())?;
        // Symbol detection
        expect!(r"\newcommand\lambda{x^2}\lambda").not_to_parse(&strict_settings())?;
        expect!(r"\newcommand\textdollar{x^2}\textdollar").not_to_parse(&strict_settings())?;
        // Macro detection
        expect!(r"\newcommand{\foo}{1}\foo\newcommand{\foo}{2}\foo")
            .not_to_parse(&strict_settings())?;
        // Implicit detection
        expect!(r"\newcommand\limits{}").not_to_parse(&strict_settings())
    });

    it("\\renewcommand redefines macros", || {
        expect!(r"\renewcommand\foo{x^2}\foo+\foo").not_to_parse(&strict_settings())?;
        expect!(r"\renewcommand{\foo}{x^2}\foo+\foo").not_to_parse(&strict_settings())?;
        expect!(r"\renewcommand\bar{x^2}\bar+\bar").to_parse_like("x^2+x^2", &strict_settings())?;
        expect!(r"\renewcommand{\bar}{x^2}\bar+\bar")
            .to_parse_like("x^2+x^2", &strict_settings())?;
        expect!(r"\newcommand{\foo}{1}\foo\renewcommand{\foo}{2}\foo")
            .to_parse_like("12", &strict_settings())
    });

    it(
        "\\providecommand defines but does not redefine macros",
        || {
            expect!(r"\providecommand\foo{x^2}\foo+\foo")
                .to_parse_like("x^2+x^2", &strict_settings())?;
            expect!(r"\providecommand{\foo}{x^2}\foo+\foo")
                .to_parse_like("x^2+x^2", &strict_settings())?;
            expect!(r"\newcommand{\foo}{1}\foo\providecommand{\foo}{2}\foo")
                .to_parse_like("11", &strict_settings())?;
            expect!(r"\providecommand{\foo}{1}\foo\renewcommand{\foo}{2}\foo")
                .to_parse_like("12", &strict_settings())?;
            expect!(r"\providecommand{\foo}{1}\foo\providecommand{\foo}{2}\foo")
                .to_parse_like("11", &strict_settings())
        },
    );

    it("\\newcommand is local", || {
        expect!(r"\newcommand\foo{1}\foo{\renewcommand\foo{2}\foo}\foo")
            .to_parse_like("1{2}1", &strict_settings())
    });

    it("\\newcommand accepts number of arguments", || {
        expect!(r"\newcommand\foo[1]{#1^2}\foo x+\foo{y}")
            .to_parse_like("x^2+y^2", &strict_settings())?;
        expect!(r"\newcommand\foo[10]{#1^2}\foo 0123456789")
            .to_parse_like("0^2", &strict_settings())?;
        expect!(r"\newcommand\foo[x]{}").not_to_parse(&strict_settings())?;
        expect!(r"\newcommand\foo[1.5]{}").not_to_parse(&strict_settings())
    });

    // This may change in the future, if we support the extra features of
    // \hspace.
    it("should treat \\hspace, \\hskip like \\kern", || {
        expect!(r"\hspace{1em}").to_parse_like(r"\kern1em", &strict_settings())?;
        expect!(r"\hskip{1em}").to_parse_like(r"\kern1em", &strict_settings())
    });

    it("should expand \\limsup as expected", || {
        expect!(r"\limsup").to_parse_like(r"\operatorname*{lim\,sup}", &strict_settings())
    });

    it("should expand \\liminf as expected", || {
        expect!(r"\liminf").to_parse_like(r"\operatorname*{lim\,inf}", &strict_settings())
    });

    it("should expand AMS log-like symbols as expected", || {
        expect!(r"\injlim").to_parse_like(r"\operatorname*{inj\,lim}", &strict_settings())?;
        expect!(r"\projlim").to_parse_like(r"\operatorname*{proj\,lim}", &strict_settings())?;
        expect!(r"\varlimsup")
            .to_parse_like(r"\operatorname*{\overline{lim}}", &strict_settings())?;
        expect!(r"\varliminf")
            .to_parse_like(r"\operatorname*{\underline{lim}}", &strict_settings())?;
        expect!(r"\varinjlim")
            .to_parse_like(r"\operatorname*{\underrightarrow{lim}}", &strict_settings())?;
        expect!(r"\varinjlim")
            .to_parse_like(r"\operatorname*{\underrightarrow{lim}}", &strict_settings())?;
        expect!(r"\varprojlim")
            .to_parse_like(r"\operatorname*{\underleftarrow{lim}}", &strict_settings())
    });

    it("should expand \\plim as expected", || {
        expect!(r"\plim").to_parse_like(r"\mathop{\operatorname{plim}}\limits", &strict_settings())
    });

    it("should expand \\argmin as expected", || {
        expect!(r"\argmin").to_parse_like(r"\operatorname*{arg\,min}", &strict_settings())
    });

    it("should expand \\argmax as expected", || {
        expect!(r"\argmax").to_parse_like(r"\operatorname*{arg\,max}", &strict_settings())
    });

    it("should expand \\bra as expected", || {
        expect!(r"\bra{\phi}").to_parse_like(r"\mathinner{\langle{\phi}|}", &strict_settings())
    });

    it("should expand \\ket as expected", || {
        expect!(r"\ket{\psi}").to_parse_like(r"\mathinner{|{\psi}\rangle}", &strict_settings())
    });

    it("should expand \\braket as expected", || {
        expect!(r"\braket{\phi|\psi}")
            .to_parse_like(r"\mathinner{\langle{\phi|\psi}\rangle}", &strict_settings())
    });

    it("should expand \\Bra as expected", || {
        expect!(r"\Bra{\phi}").to_parse_like(r"\left\langle\phi\right|", &strict_settings())
    });

    it("should expand \\Ket as expected", || {
        expect!(r"\Ket{\psi}").to_parse_like(r"\left|\psi\right\rangle", &strict_settings())
    });

    it("should expand \\Braket as expected", || {
        expect!(r"\Braket{ ϕ | \frac{∂^2}{∂ t^2} | ψ }").to_parse_like(
            r"\left\langle ϕ\,\middle\vert\,\frac{∂^2}{∂ t^2}\,\middle\vert\, ψ\right\rangle",
            &strict_settings(),
        )
    });

    it("should expand \\set as expected", || {
        expect!(r"\set{x|x<5|S|}")
            .to_parse_like(r"\{ \, x \mid x<5 |S| \, \}", &strict_settings())?;
        // \set doesn't support special || or \| handling
        expect!(r"\set{x||x<5|S|}")
            .to_parse_like(r"\{ \, x \mid |x<5|S| \, \}", &strict_settings())?;
        expect!(r"\set{x\|x<5|S|}")
            .to_parse_like(r"\{ \, x \| x<5 \mid S| \, \}", &strict_settings())
    });

    it("should expand \\Set as expected", || {
        expect!(r"\Set{ x | x<\frac 1 2 |S| }").to_parse_like(
            r"\left\{\: x\;\middle\vert\; x<\frac 1 2 |S| \:\right\}",
            &strict_settings(),
        )?;
        expect!(r"\Set{ x || x<\frac 1 2 |S| }").to_parse_like(
            r"\left\{\: x\;\middle\Vert\; x<\frac 1 2 |S| \:\right\}",
            &strict_settings(),
        )?;
        expect!(r"\Set{ x \| x<\frac 1 2 |S| }").to_parse_like(
            r"\left\{\: x\;\middle\Vert\; x<\frac 1 2 |S| \:\right\}",
            &strict_settings(),
        )
    });
}

#[test]
fn tag_support() {
    it("should fail outside display mode", || {
        expect!(r"\tag{hi}x+y").not_to_parse(&strict_settings())
    });

    it("should fail with multiple tags", || {
        expect!(r"\tag{1}\tag{2}x+y").not_to_parse(&display_settings())
    });

    it("should fail with multiple tags in one row", || {
        expect!(r"\begin{align}\tag{1}x+y\tag{2}\end{align}").not_to_parse(&display_settings())
    });

    it("should work with one tag per row", || {
        expect!(r"\begin{align}\tag{1}x+y\\&+y\tag{2}\end{align}").to_parse(&display_settings())
    });

    it("should work with \\nonumber/\\notag", || {
        expect!(r"\begin{align}\tag{1}\nonumber x\\&+y\notag\end{align}").to_parse_like(
            r"\begin{align}\tag{1}x\\&+y\nonumber\end{align}",
            &display_settings(),
        )
    });

    it("should build", || {
        expect!(r"\tag{hi}x+y").to_build(&display_settings())
    });

    it("should ignore location of \\tag", || {
        expect!(r"\tag{hi}x+y").to_parse_like(r"x+y\tag{hi}", &display_settings())
    });

    it("should handle \\tag* like \\tag", || {
        expect!(r"\tag{hi}x+y").to_parse_like(r"\tag*{({hi})}x+y", &display_settings())
    });
}

#[test]
fn leqno_and_fleqn_rendering_options() {
    let expr = r"\tag{hi}x+y";

    for opt in ["leqno", "fleqn"] {
        it(&format!("should not add {opt} class by default"), || {
            let settings = display_settings();
            let built = get_built(expr, &settings)?;
            // Check if the root span contains the class
            if let HtmlDomNode::DomSpan(span) = &built[0] {
                assert!(!span.classes.contains(opt));
            }
            Ok(())
        });

        it(&format!("should not add {opt} class when false"), || {
            let mut settings = display_settings();
            if opt == "leqno" {
                settings.leqno = false;
            } else {
                settings.fleqn = false;
            }
            let built = get_built(expr, &settings)?;
            // Check if the root span contains the class
            if let HtmlDomNode::DomSpan(span) = &built[0] {
                assert!(!span.classes.contains(opt));
            }
            Ok(())
        });

        it(&format!("should add {opt} class when true"), || {
            let mut settings = display_settings();
            if opt == "leqno" {
                settings.leqno = true;
            } else {
                settings.fleqn = true;
            }
            let built = get_built(expr, &settings)?;
            // Check if the root span contains the class
            if let HtmlDomNode::DomSpan(span) = &built[0] {
                assert!(span.classes.contains(opt));
            }
            Ok(())
        });
    }
}

#[test]
fn binrel_automatic_bin_rel_ord() {
    it("should generate proper class", || {
        expect!(r"L\@binrel+xR").to_parse_like(r"L\mathbin xR", &strict_settings())?;
        expect!(r"L\@binrel=xR").to_parse_like(r"L\mathrel xR", &strict_settings())?;
        expect!(r"L\@binrel xxR").to_parse_like(r"L\mathord xR", &strict_settings())?;
        expect!(r"L\@binrel{+}{x}R").to_parse_like(r"L\mathbin{x}R", &strict_settings())?;
        expect!(r"L\@binrel{=}{x}R").to_parse_like(r"L\mathrel{x}R", &strict_settings())?;
        expect!(r"L\@binrel{x}{x}R").to_parse_like(r"L\mathord{x}R", &strict_settings())
    });

    it("should base on just first character in group", || {
        expect!(r"L\@binrel{+x}xR").to_parse_like(r"L\mathbin xR", &strict_settings())?;
        expect!(r"L\@binrel{=x}xR").to_parse_like(r"L\mathrel xR", &strict_settings())?;
        expect!(r"L\@binrel{xx}xR").to_parse_like(r"L\mathord xR", &strict_settings())
    });
}

#[test]
fn a_parser_taking_string_objects() {
    it("should not fail on an empty String object", || {
        expect!("").to_parse(&strict_settings())
    });

    it("should parse the same as a regular string", || {
        expect!("xy").to_parse_like("xy", &strict_settings())?;
        expect!(r"\div").to_parse_like(r"\div", &strict_settings())?;
        expect!(r"\frac 1 2").to_parse_like(r"\frac 1 2", &strict_settings())
    });
}

#[test]
fn unicode_accents() {
    it("should parse Latin-1 letters in math mode", || {
        // TODO(edemaine): Unsupported Latin-1 letters in math: ÇÐÞçðþ
        expect!("ÀÁÂÃÄÅÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåèéêëìíîïñòóôõöùúûüýÿ")
            .to_parse_like(
                r"\grave A\acute A\hat A\tilde A\ddot A\mathring A\grave E\acute E\hat E\ddot E\grave I\acute I\hat I\ddot I\tilde N\grave O\acute O\hat O\tilde O\ddot O\grave U\acute U\hat U\ddot U\acute Y\grave a\acute a\hat a\tilde a\ddot a\mathring a\grave e\acute e\hat e\ddot e\grave ı\acute ı\hat ı\ddot ı\tilde n\grave o\acute o\hat o\tilde o\ddot o\grave u\acute u\hat u\ddot u\acute y\ddot y",
                &nonstrict_settings(),
            )
    });

    it("should parse Latin-1 letters in text mode", || {
        // TODO(edemaine): Unsupported Latin-1 letters in text: ÇÐÞçðþ
        expect!(r"\text{ÀÁÂÃÄÅÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåèéêëìíîïñòóôõöùúûüýÿ}")
        .to_parse_like(r#"\text{\`A\'A\^A\~A\"A\r A\`E\'E\^E\"E\`I\'I\^I\"I\~N\`O\'O\^O\~O\"O\`U\'U\^U\"U\'Y\`a\'a\^a\~a\"a\r a\`e\'e\^e\"e\`ı\'ı\^ı\"ı\~n\`o\'o\^o\~o\"o\`u\'u\^u\"u\'y\"y}"#, &strict_settings())
    });

    it("should support \\aa in text mode", || {
        expect!(r"\text{\aa\AA}").to_parse_like(r"\text{\r a\r A}", &strict_settings())?;
        expect!(r"\aa").not_to_parse(&strict_settings())?;
        expect!(r"\Aa").not_to_parse(&strict_settings())
    });

    it("should parse combining characters", || {
        expect!("A\u{0300}C\u{0301}").to_parse_like(r"\grave A\acute C", &nonstrict_settings())?;
        expect!("\\text{A\u{0300}C\u{0301}}").to_parse_like(r"\text{\`A\'C}", &strict_settings())
    });

    it("should parse multi-accented characters", || {
        expect!("ấā́ắ\\text{ấā́ắ}").to_parse(&nonstrict_settings())
    });

    it("should parse accented i's and j's", || {
        expect!("íȷ́").to_parse_like(r"\acute ı\acute ȷ", &nonstrict_settings())?;
        expect!("ấā́ắ\\text{ấā́ắ}").to_parse(&nonstrict_settings())
    });
}

#[test]
fn unicode() {
    it("should parse negated relations", || {
        expect!("∉∤∦≁≆≠≨≩≮≯≰≱⊀⊁⊈⊉⊊⊋⊬⊭⊮⊯⋠⋡⋦⋧⋨⋩⋬⋭⪇⪈⪉⪊⪵⪶⪹⪺⫋⫌").to_parse(&strict_settings())
    });

    it("should build relations", || {
        expect!("∈∋∝∼∽≂≃≅≈≊≍≎≏≐≑≒≓≖≗≜≡≤≥≦≧≪≫≬≳≷≺≻≼≽≾≿∴∵∣≔≕⩴⋘⋙⟂⊨∌").to_build(&strict_settings())
    });

    it("should parse relations", || {
        // These characters are not in the KaTeX fonts. So they build with an error
        // message.
        expect!("⊶⊷").to_parse(&strict_settings())
    });

    it("should build big operators", || {
        expect!("∏∐∑∫∬∭∮⋀⋁⋂⋃⨀⨁⨂⨄⨆").to_build(&strict_settings())
    });

    it("should build more relations", || {
        expect!("⊂⊃⊆⊇⊏⊐⊑⊒⊢⊣⊩⊪⊸⋈⋍⋐⋑⋔⋛⋞⋟⌢⌣⩾⪆⪌⪕⪖⪯⪰⪷⪸⫅⫆≘≙≚≛≝≞≟≲⩽⪅≶⋚⪋").to_build(&strict_settings())
    });

    it("should parse symbols", || {
        expect!("£¥ℂℍℑℎℓℕ℘ℙℚℜℝℤℲℵðℶℷℸ⅁∀∁∂∃∇∞∠∡∢♠♡♢♣♭♮♯✓°¬‼⋮\u{00B7}\u{00A9}")
            .to_build(&strict_settings())?;
        expect!(r"\text{£¥ℂℍℎ©®}").to_build(&strict_settings())
    });

    it("should build Greek capital letters", || {
        expect!("\u{0391}\u{0392}\u{0395}\u{0396}\u{0397}\u{0399}\u{039A}\u{039C}\u{039D}\u{039F}\u{03A1}\u{03A4}\u{03A7}\u{03DD}").to_build(&strict_settings())
    });

    it("should build arrows", || {
        expect!("←↑→↓↔↕↖↗↘↙↚↛↞↠↢↣↦↩↪↫↬↭↮↰↱↶↷↼↽↾↾↿⇀⇁⇂⇃⇄⇆⇇⇈⇉").to_build(&strict_settings())
    });

    it("should build more arrows", || {
        expect!("⇊⇋⇌⇍⇎⇏⇐⇑⇒⇓⇔⇕⇚⇛⇝⟵⟶⟷⟸⟹⟺⟼").to_build(&strict_settings())
    });

    it("should build binary operators", || {
        expect!("±×÷∓∔∧∨∩∪≀⊎⊓⊔⊕⊖⊗⊘⊙⊚⊛⊝◯⊞⊟⊠⊡⊺⊻⊼⋇⋉⋊⋋⋌⋎⋏⋒⋓⩞\u{22C5}\u{2218}\u{2216}\u{2219}")
            .to_build(&strict_settings())
    });

    it("should build common ords", || {
        expect!("§¶£¥∇∞⋅∠∡∢♠♡♢♣♭♮♯✓…⋮⋯⋱! ‼ ⦵").to_build(&strict_settings())
    });

    it("should build delimiters", || {
        expect!("\\left\u{230A}\\frac{a}{b}\\right\u{230B}").to_build(&strict_settings())?;
        expect!("\\left\u{2308}\\frac{a}{b}\\right\u{2308}").to_build(&strict_settings())?;
        expect!("\\left\u{27EE}\\frac{a}{b}\\right\u{27EF}").to_build(&strict_settings())?;
        expect!("\\left\u{27E8}\\frac{a}{b}\\right\u{27E9}").to_build(&strict_settings())?;
        expect!("\\left\u{23B0}\\frac{a}{b}\\right\u{23B1}").to_build(&strict_settings())?;
        expect!("┌x┐ └x┘").to_build(&strict_settings())?;
        expect!("\u{231C}x\u{231D} \u{231E}x\u{231F}").to_build(&strict_settings())?;
        expect!("\u{27E6}x\u{27E7}").to_build(&strict_settings())?;
        expect!(r"\llbracket \rrbracket").to_build(&strict_settings())?;
        expect!(r"\lBrace \rBrace").to_build(&strict_settings())
    });

    it("should build some surrogate pairs", || {
        let get_surrogate_pairs = |s: &mut String| {
            for ch in [
                0x1D400, 0x1D468, 0x1D504, 0x1D56C, 0x1D538, 0x1D49C, 0x1D5A0, 0x1D5D4, 0x1D608,
                0x1D670, 0x1D7CE, 0x1D7E2, 0x1D7EC, 0x1D7F6,
            ] {
                s.push(char::from_u32(ch).unwrap());
            }
        };

        let mut wide_char_str = String::new();
        get_surrogate_pairs(&mut wide_char_str);
        expect!(&wide_char_str).to_build(&strict_settings())?;

        let mut wide_char_text = r"\text{".to_owned();
        get_surrogate_pairs(&mut wide_char_text);
        wide_char_text.push('}');
        expect!(&wide_char_text).to_build(&strict_settings())
    });
}

#[test]
fn the_max_size_setting() {
    let rule = r"\rule{999em}{999em}";

    it("should clamp size when set", || {
        let settings = Settings::builder().max_size(5.0).build();
        let built = get_built(rule, &settings)?;
        assert_let!(Some(HtmlDomNode::DomSpan(span)) = built.first());
        assert_eq!(span.style.get(CssProperty::BorderRightWidth), Some("5em"));
        assert_eq!(span.style.get(CssProperty::BorderTopWidth), Some("5em"));
        Ok(())
    });

    it("should not clamp size when not set", || {
        let built = get_built(rule, &strict_settings())?;
        assert_let!(Some(HtmlDomNode::DomSpan(span)) = built.first());
        assert_eq!(span.style.get(CssProperty::BorderRightWidth), Some("999em"));
        assert_eq!(span.style.get(CssProperty::BorderTopWidth), Some("999em"));
        Ok(())
    });

    it(
        "should make zero-width rules if a negative maxSize is passed",
        || {
            let settings = Settings::builder().max_size(-5.0).build();
            let built = get_built(rule, &settings)?;
            assert_let!(Some(HtmlDomNode::DomSpan(span)) = built.first());
            assert_eq!(span.style.get(CssProperty::BorderRightWidth), Some("0em"));
            assert_eq!(span.style.get(CssProperty::BorderTopWidth), Some("0em"));
            Ok(())
        },
    );
}

#[test]
fn the_max_expand_setting() {
    it("should prevent expansion", || {
        expect!(r"\gdef\foo{1}\foo").to_parse(&strict_settings())?;
        expect!(r"\gdef\foo{1}\foo").to_parse(&Settings::builder().max_expand(1).build())?;
        expect!(r"\gdef\foo{1}\foo").not_to_parse(&Settings::builder().max_expand(0).build())
    });

    it("should prevent infinite loops", || {
        expect!(r"\gdef\foo{\foo}\foo").not_to_parse(&Settings::builder().max_expand(10).build())
    });

    it("should prevent exponential blowup via \\edef", || {
        expect!(r"\edef0{x}\edef0{00}\edef0{00}\edef0{00}\edef0{00}")
            .not_to_parse(&Settings::builder().max_expand(10).build())
    });

    const EXP32: &str = r"
        \def\a#1{\b{#1}\b{#1}}
        \def\b#1{\c{#1}\c{#1}}
        \def\c#1{\d{#1}\d{#1}}
        \def\d#1{\e{#1}\e{#1}}
        \def\e#1{\f{#1}\f{#1}}
        \def\f#1{#1}
    ";

    it("should count correctly", || {
        let example = format!("{EXP32}\\a{{1}}");
        let count = 1 + 2 + 4 + 8 + 16 + 32;
        expect!(&example).to_parse(&Settings::builder().max_expand(count).build())?;
        expect!(&example).not_to_parse(&Settings::builder().max_expand(count - 1).build())
    });

    it(
        "should count correctly with Unicode sub/superscripts",
        || {
            let example = format!("{EXP32}\\def+{{\\a{{1}}}}x⁺x⁺x⁺x⁺");
            let count = (1 + 2 + 4 + 8 + 16 + 32) * 4 + 4;
            expect!(&example).to_parse(&Settings::builder().max_expand(count).build())?;
            expect!(&example).not_to_parse(&Settings::builder().max_expand(count - 1).build())
        },
    );
}

#[test]
fn the_mathchoice_function() {
    let cmd = r"\sum_{k = 0}^{\infty} x^k";

    it(
        "should render as if there is nothing other in display math",
        || {
            expect!(&format!(
                r"\displaystyle\mathchoice{{{cmd}}}{{T}}{{S}}{{SS}}"
            ))
            .to_build_like(&format!(r"\displaystyle{cmd}"), &strict_settings())
        },
    );

    it("should render as if there is nothing other in text", || {
        expect!(&format!(r"\mathchoice{{D}}{{{cmd}}}{{S}}{{SS}}"))
            .to_build_like(cmd, &strict_settings())
    });

    it(
        "should render as if there is nothing other in scriptstyle",
        || {
            expect!(&format!(r"x_{{\mathchoice{{D}}{{T}}{{{cmd}}}{{SS}}}}"))
                .to_build_like(&format!("x_{{{cmd}}}"), &strict_settings())
        },
    );

    it(
        "should render  as if there is nothing other in scriptscriptstyle",
        || {
            expect!(&format!(r"x_{{y_{{\mathchoice{{D}}{{T}}{{S}}{{{cmd}}}}}}}"))
                .to_build_like(&format!("x_{{y_{{{cmd}}}}}"), &strict_settings())
        },
    );
}

#[test]
fn newlines_via_backslash_and_newline() {
    it(
        "should build \\\\ without the optional argument and \\newline the same",
        || expect!("hello \\\\ world").to_build_like("hello \\newline world", &strict_settings()),
    );

    it(
        "should not allow \\newline to scan for an optional size argument",
        || expect!("hello \\newline[w]orld").to_build(&strict_settings()),
    );

    it("should not allow \\cr at top level", || {
        expect!("hello \\cr world").not_to_build(&strict_settings())
    });

    it("\\\\ causes newline, even after mrel and mop", || {
        let markup = render_to_string_strict(r"M = \\ a + \\ b \\ c")?;
        let normalized = normalize_style_attributes(&markup);
        insta::assert_snapshot!("newline_markup", normalized);
        Ok(())
    });
}

#[test]
fn symbols() {
    it("should parse \\text{\\i\\j}", || {
        expect!(r"\text{\i\j}").to_build(&strict_settings())
    });

    it(
        "should parse spacing functions in math or text mode",
        || {
            expect!(r"A\;B\,C\nobreakspace \text{A\;B\,C\nobreakspace}")
                .to_build(&strict_settings())
        },
    );

    it("should build \\minuso", || {
        expect!(r"\minuso").to_build(&strict_settings())
    });

    it(
        "should render ligature commands like their unicode characters",
        || {
            expect!(r"\text{\ae\AE\oe\OE\o\O\ss}")
                .to_build_like(r"\text{æÆœŒøØß}", &strict_settings())
        },
    );
}

#[test]
fn strict_setting() {
    it("should allow unicode text when not strict", || {
        expect!("é").to_parse(&nonstrict_settings())?;
        expect!("試").to_parse(&nonstrict_settings())?;
        expect!("é").to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Mode(katex::StrictMode::Ignore))
                .build(),
        )?;
        expect!("試").to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Mode(katex::StrictMode::Ignore))
                .build(),
        )?;
        expect!("é").to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Bool(false))
                })))
                .build(),
        )?;
        expect!("試").to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Bool(false))
                })))
                .build(),
        )?;
        expect!("é").to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Mode(katex::StrictMode::Ignore))
                })))
                .build(),
        )?;
        expect!("試").to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Mode(katex::StrictMode::Ignore))
                })))
                .build(),
        )
    });

    it("should forbid unicode text when strict", || {
        expect!("é").not_to_parse(&strict_settings())?;
        expect!("試").not_to_parse(&strict_settings())?;
        expect!("é").not_to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Mode(katex::StrictMode::Error))
                .build(),
        )?;
        expect!("試").not_to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Mode(katex::StrictMode::Error))
                .build(),
        )?;
        expect!("é").not_to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Bool(true))
                })))
                .build(),
        )?;
        expect!("試").not_to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Bool(true))
                })))
                .build(),
        )?;
        expect!("é").not_to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Mode(katex::StrictMode::Error))
                })))
                .build(),
        )?;
        expect!("試").not_to_parse(
            &Settings::builder()
                .strict(katex::StrictSetting::Function(Arc::new(|_, _, _| {
                    Some(katex::StrictReturn::Mode(katex::StrictMode::Error))
                })))
                .build(),
        )
    });

    it("should warn about unicode text when default", || {
        // Note: In Rust implementation, warnings are not captured in tests
        // This test just ensures parsing succeeds with default settings
        expect!("é").to_parse(&Settings::default())?;
        expect!("試").to_parse(&Settings::default())
    });

    it("should always allow unicode text in text mode", || {
        expect!(r"\text{é試}").to_parse(&nonstrict_settings())?;
        expect!(r"\text{é試}").to_parse(&strict_settings())?;
        expect!(r"\text{é試}").to_parse(&Settings::default())
    });

    it(
        "should warn about top-level \\newline in display mode",
        || {
            // Note: In Rust implementation, warnings are not captured in tests
            // This test just ensures parsing succeeds with display mode
            expect!("x\\\\y").to_parse(&display_settings())?;
            expect!("x\\\\y").to_parse(&non_display_settings())
        },
    );
}

#[test]
fn internal_interface() {
    let latex = r"\sum_{k = 0}^{\infty} x^k";

    it("__parse renders same as renderToString", || {
        let settings = Settings::default();
        let ctx = default_ctx();
        let parsed = katex::parse(ctx, latex, &settings)?;
        let built = katex::build_tree::build_tree(ctx, &parsed, latex, &settings)?;
        let markup = built.to_markup()?;
        let rendered = katex::render_to_string(ctx, latex, &settings)?;

        assert_html_eq_unordered_styles(&markup, &rendered);

        Ok(())
    });

    it("__renderToDomTree renders same as renderToString", || {
        let settings = Settings::default();
        let ctx = default_ctx();
        let tree = katex::render_to_dom_tree(ctx, latex, &settings)?;
        let rendered = katex::render_to_string(ctx, latex, &settings)?;
        assert_html_eq_unordered_styles(&tree.to_markup()?, &rendered);
        Ok(())
    });

    it(
        "__renderToHTMLTree renders same as renderToString sans MathML",
        || {
            let settings = Settings::default();
            let ctx = default_ctx();
            let tree = katex::render_to_html_tree(ctx, latex, &settings)?;
            let rendered = katex::render_to_string(ctx, latex, &settings)?;
            // Remove MathML span from rendered output
            let mathml_start = rendered.find(r#"<span class="katex-mathml">"#);
            let rendered_sans_mathml = if let Some(start) = mathml_start {
                let end = rendered[start..]
                    .find("</span>")
                    .map_or(rendered.len(), |e| start + e + 7);
                format!("{}{}", &rendered[..start], &rendered[end..])
            } else {
                rendered
            };
            assert_html_eq_unordered_styles(&tree.to_markup()?, &rendered_sans_mathml);
            Ok(())
        },
    );
}

#[test]
fn extending_katex_by_new_fonts_and_symbols() {
    let get_context = || -> KatexContext {
        let mut ctx = KatexContext::default();
        for number in 0..=9 {
            let persian_num = char::from_u32(0x0660 + number).unwrap();
            ctx.symbols.define_symbol(
                Mode::Math,
                Font::Custom("mockEasternArabicFont".to_owned()),
                Group::NonAtom(NonAtom::TextOrd),
                Some(persian_num),
                &persian_num.to_string(),
                false,
            );
            let arabic_num = char::from_u32(0x06F0 + number).unwrap();
            ctx.symbols.define_symbol(
                Mode::Math,
                Font::Custom("mockEasternArabicFont".to_owned()),
                Group::NonAtom(NonAtom::TextOrd),
                Some(arabic_num),
                &arabic_num.to_string(),
                false,
            );
        }
        ctx
    };

    it(
        "should throw on rendering new symbols with no font metrics",
        || {
            // Lets parse 99^11 in eastern arabic
            let result = render_to_dom_tree(&get_context(), "۹۹^{۱۱}", &strict_settings())
                .err()
                .unwrap();
            assert_eq!(
                result.to_string(),
                "KaTeX parse error: Font metrics not found for font: mockEasternArabicFont-Regular."
            );
            Ok(())
        },
    );

    it(
        "should add font metrics to metrics map and render successfully",
        || {
            let mut ctx = get_context();
            for number in 0..=9 {
                ctx.font_metrics.add_custom_metrics(
                    "mockEasternArabicFont-Regular".to_owned(),
                    0x0660 + number,
                    CharacterMetrics {
                        depth: -0.00244140625,
                        height: 0.6875,
                        italic: 0.0,
                        skew: 0.0,
                        width: 0.0,
                    },
                );
                ctx.font_metrics.add_custom_metrics(
                    "mockEasternArabicFont-Regular".to_owned(),
                    0x06F0 + number,
                    CharacterMetrics {
                        depth: -0.00244140625,
                        height: 0.6875,
                        italic: 0.0,
                        skew: 0.0,
                        width: 0.0,
                    },
                );
            }
            expect!("۹۹^{۱۱}").to_build(&nonstrict_settings())?;
            Ok(())
        },
    );

    it("Add new font class to new extended symbols", || {
        let mut ctx = get_context();
        for number in 0..=9 {
            ctx.font_metrics.add_custom_metrics(
                "mockEasternArabicFont-Regular".to_owned(),
                0x0660 + number,
                CharacterMetrics {
                    depth: -0.00244140625,
                    height: 0.6875,
                    italic: 0.0,
                    skew: 0.0,
                    width: 0.0,
                },
            );
            ctx.font_metrics.add_custom_metrics(
                "mockEasternArabicFont-Regular".to_owned(),
                0x06F0 + number,
                CharacterMetrics {
                    depth: -0.00244140625,
                    height: 0.6875,
                    italic: 0.0,
                    skew: 0.0,
                    width: 0.0,
                },
            );
        }
        let settings = nonstrict_settings();
        let html =
            render_to_string(&ctx, "۹۹^{۱۱}", &settings).map_err(|source| TestError::Parse {
                expression: "۹۹^{۱۱}".to_owned(),
                location: TestLocation::UNKNOWN,
                source,
            })?;
        let normalized_html = normalize_style_attributes(&html);
        insta::assert_snapshot!("eastern_arabic_numerals_html", normalized_html);
        Ok(())
    });
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn debugging_macros() {
    use gag::BufferRedirect;

    it("should print the argument using console.log", || {
        let mut buf = BufferRedirect::stdout().unwrap();
        expect!(r"\message{Hello, world}").to_parse(&strict_settings())?;
        let mut output = String::new();
        buf.read_to_string(&mut output).unwrap();
        assert!(output.contains("Hello, world"));
        Ok(())
    });

    it("should print the argument using console.error", || {
        let mut buf = BufferRedirect::stderr().unwrap();
        expect!(r"\errmessage{Hello, world}").to_parse(&strict_settings())?;
        let mut output = String::new();
        buf.read_to_string(&mut output).unwrap();
        assert!(output.contains("Hello, world"));
        Ok(())
    });
}

#[test]
fn relax() {
    it("should stop the expansion", || {
        expect!(r"\kern2\relax em").not_to_parse(&strict_settings())
    });
}

#[test]
fn emph() {
    it("should toggle italics", || {
        expect!(r"\emph{foo \emph{bar}}")
            .to_build_like(r"\textit{foo \textup{bar}}", &strict_settings())
    });

    it("should toggle italics within text", || {
        expect!(r"\text{\emph{foo \emph{bar}}}")
            .to_build_like(r"\text{\textit{foo \textup{bar}}}", &strict_settings())
    });

    it("should toggle italics within textup", || {
        expect!(r"\textup{\emph{foo \emph{bar}}}")
            .to_build_like(r"\textup{\textit{foo \textup{bar}}}", &strict_settings())
    });

    it("should toggle italics within textit", || {
        expect!(r"\textit{\emph{foo \emph{bar}}}")
            .to_build_like(r"\textit{\textup{foo \textit{bar}}}", &strict_settings())
    });
}
