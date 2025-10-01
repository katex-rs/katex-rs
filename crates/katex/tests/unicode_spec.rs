#![allow(clippy::non_ascii_literal)]

mod setup;

use katex::{
    symbols::{Mode as SymbolMode, create_symbols},
    types::{Settings, StrictMode, StrictReturn, StrictSetting},
    unicode::{script_from_codepoint, supported_codepoint},
};
use regex::Regex;
use setup::*;
use std::sync::Arc;

#[test]
fn unicode() {
    it("should build Latin-1 inside \\text{}", || {
        let settings = Settings::default();
        expect!(r"\text{ÀÁÂÃÄÅÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåèéêëìíîïñòóôõöùúûüýÿÆÇÐØÞßæçðøþ}")
            .to_build(&settings)?;
        Ok(())
    });

    it(
        "should build Latin-1 inside \\text{} like accent commands",
        || {
            let settings = Settings::default();
            expect!(
                r"\text{ÀÁÂÃÄÅÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåèéêëìíîïñòóôõöùúûüýÿÇç}"
            )
            .to_parse_like(
                r#"\text{\`A\'A\^A\~A\"A\r A\`E\'E\^E\"E\`I\'I\^I\"I\~N\`O\'O\^O\~O\"O\`U\'U\^U\"U\'Y\`a\'a\^a\~a\"a\r a\`e\'e\^e\"e\`ı\'ı\^ı\"ı\~n\`o\'o\^o\~o\"o\`u\'u\^u\"u\'y\"y\c C\c c}"#,
                &settings,
            )?;
            Ok(())
        },
    );

    it(
        "should not parse Latin-1 outside \\text{} with strict",
        || {
            let chars = "ÀÁÂÃÄÅÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåèéêëìíîïñòóôõöùúûüýÿÇÐÞçþ";
            let settings = strict_settings();
            let symbols = create_symbols();
            let mut checked = false;
            for ch in chars.chars() {
                let ch_str = ch.to_string();
                if symbols.contains(SymbolMode::Math, &ch_str) {
                    continue;
                }
                checked = true;
                expect!(ch_str.as_str()).not_to_parse(&settings)?;
            }
            assert!(
                checked,
                "Expected at least one unsupported Latin-1 character"
            );
            Ok(())
        },
    );

    it("should build Latin-1 outside \\text{}", || {
        let settings = nonstrict_settings();
        expect!(r"ÀÁÂÃÄÅÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåèéêëìíîïñòóôõöùúûüýÿÇÐÞçðþ")
            .to_build(&settings)?;
        Ok(())
    });

    it("should build all lower case Greek letters", || {
        let settings = Settings::default();
        expect!("αβγδεϵζηθϑικλμνξοπϖρϱςστυφϕχψω").to_build(&settings)?;
        Ok(())
    });

    it("should build math upper case Greek letters", || {
        let settings = Settings::default();
        expect!("ΓΔΘΛΞΠΣΥΦΨΩ").to_build(&settings)?;
        Ok(())
    });

    it("should build Cyrillic inside \\text{}", || {
        let settings = Settings::default();
        expect!(r"\text{БГДЖЗЙЛФЦШЫЮЯ}").to_build(&settings)?;
        Ok(())
    });

    it("should build Cyrillic outside \\text{}", || {
        let settings = nonstrict_settings();
        expect!("БГДЖЗЙЛФЦШЫЮЯ").to_build(&settings)?;
        Ok(())
    });

    it(
        "should not parse Cyrillic outside \\text{} with strict",
        || {
            let settings = strict_settings();
            expect!("БГДЖЗЙЛФЦШЫЮЯ").not_to_parse(&settings)?;
            Ok(())
        },
    );

    it("should build CJK inside \\text{}", || {
        let settings = Settings::default();
        expect!(r"\text{私はバナナです}").to_build(&settings)?;
        expect!(r"\text{여보세요}").to_build(&settings)?;
        Ok(())
    });

    it("should build CJK outside \\text{}", || {
        let settings = nonstrict_settings();
        expect!("私はバナナです").to_build(&settings)?;
        expect!("여보세요").to_build(&settings)?;
        Ok(())
    });

    it("should not parse CJK outside \\text{} with strict", || {
        let settings = strict_settings();
        expect!("私はバナナです。").not_to_parse(&settings)?;
        expect!("여보세요").not_to_parse(&settings)?;
        Ok(())
    });

    it("should build Devanagari inside \\text{}", || {
        let settings = Settings::default();
        expect!(r"\text{नमस्ते}").to_build(&settings)?;
        Ok(())
    });

    it("should build Devanagari outside \\text{}", || {
        let settings = nonstrict_settings();
        expect!("नमस्ते").to_build(&settings)?;
        Ok(())
    });

    it(
        "should not parse Devanagari outside \\text{} with strict",
        || {
            let settings = strict_settings();
            expect!("नमस्ते").not_to_parse(&settings)?;
            Ok(())
        },
    );

    it("should build Georgian inside \\text{}", || {
        let settings = Settings::default();
        expect!(r"\text{გამარჯობა}").to_build(&settings)?;
        Ok(())
    });

    it("should build Georgian outside \\text{}", || {
        let settings = nonstrict_settings();
        expect!("გამარჯობა").to_build(&settings)?;
        Ok(())
    });

    it(
        "should not parse Georgian outside \\text{} with strict",
        || {
            let settings = strict_settings();
            expect!("გამარჯობა").not_to_parse(&settings)?;
            Ok(())
        },
    );

    it(
        "should build Armenian both inside and outside \\text{}",
        || {
            let settings = nonstrict_settings();
            expect!("ԱԲԳաբգ").to_build(&settings)?;
            expect!(r"\text{ԱԲԳաբգ}").to_build(&settings)?;
            Ok(())
        },
    );

    it(
        "should build extended Latin characters inside \\text{}",
        || {
            let settings = Settings::default();
            expect!(r"\text{ěščřžůřťďňőİı}").to_build(&settings)?;
            Ok(())
        },
    );

    it(
        "should not parse extended Latin outside \\text{} with strict",
        || {
            let settings = strict_settings();
            expect!("ěščřžůřťďňőİı").not_to_parse(&settings)?;
            Ok(())
        },
    );

    it("should not allow emoji in strict mode", || {
        let strict = strict_settings();
        expect!("✌").not_to_parse(&strict)?;
        expect!(r"\text{✌}").not_to_parse(&strict)?;

        let settings = Settings::builder()
            .strict(StrictSetting::Function(Arc::new(|error_code, _, _| {
                if error_code == "unknownSymbol" {
                    Some(StrictReturn::Mode(StrictMode::Error))
                } else {
                    Some(StrictReturn::Mode(StrictMode::Ignore))
                }
            })))
            .build();
        expect!("✌").not_to_parse(&settings)?;
        expect!(r"\text{✌}").not_to_parse(&settings)?;
        Ok(())
    });

    it("should allow emoji outside strict mode", || {
        let settings = nonstrict_settings();
        expect!("✌").to_parse(&settings)?;
        expect!(r"\text{✌}").to_parse(&settings)?;

        let settings = Settings::builder()
            .strict(StrictSetting::Function(Arc::new(|error_code, _, _| {
                if error_code == "unknownSymbol" {
                    Some(StrictReturn::Mode(StrictMode::Ignore))
                } else {
                    Some(StrictReturn::Mode(StrictMode::Error))
                }
            })))
            .build();
        expect!("✌").to_parse(&settings)?;
        expect!(r"\text{✌}").to_parse(&settings)?;
        Ok(())
    });
}

#[test]
fn unicode_scripts() {
    let script_regexps = [
        (
            "latin",
            Regex::new(r"[\u{0100}-\u{024f}\u{0300}-\u{036f}]").unwrap(),
        ),
        ("cyrillic", Regex::new(r"[\u{0400}-\u{04ff}]").unwrap()),
        ("armenian", Regex::new(r"[\u{0530}-\u{058f}]").unwrap()),
        ("brahmic", Regex::new(r"[\u{0900}-\u{109f}]").unwrap()),
        ("georgian", Regex::new(r"[\u{10a0}-\u{10ff}]").unwrap()),
        (
            "cjk",
            Regex::new(r"[\u{3000}-\u{30ff}\u{4e00}-\u{9faf}\u{ff00}-\u{ff60}]").unwrap(),
        ),
        ("hangul", Regex::new(r"[\u{ac00}-\u{d7af}]").unwrap()),
    ];

    it(
        "supportedCodepoint() should return the correct values",
        || {
            for codepoint in 0..=0xffff {
                if let Some(ch) = char::from_u32(codepoint) {
                    let s = ch.to_string();
                    let expected = script_regexps.iter().any(|(_, re)| re.is_match(&s));
                    assert_eq!(
                        supported_codepoint(codepoint),
                        expected,
                        "codepoint U+{:04X}",
                        codepoint
                    );
                } else {
                    assert!(
                        !supported_codepoint(codepoint),
                        "codepoint U+{:04X}",
                        codepoint
                    );
                }
            }
            Ok(())
        },
    );

    it("scriptFromCodepoint() should return correct values", || {
        'outer: for codepoint in 0..=0xffff {
            let Some(ch) = char::from_u32(codepoint) else {
                assert_eq!(script_from_codepoint(codepoint), None);
                assert!(
                    !supported_codepoint(codepoint),
                    "codepoint U+{:04X}",
                    codepoint
                );
                continue;
            };
            let s = ch.to_string();
            let script = script_from_codepoint(codepoint);

            for (script_name, re) in &script_regexps {
                if re.is_match(&s) {
                    assert_eq!(script, Some(*script_name), "codepoint U+{:04X}", codepoint);
                    continue 'outer;
                }
            }

            assert_eq!(script, None, "codepoint U+{:04X}", codepoint);
            assert!(
                !supported_codepoint(codepoint),
                "codepoint U+{:04X}",
                codepoint
            );
        }
        Ok(())
    });
}
