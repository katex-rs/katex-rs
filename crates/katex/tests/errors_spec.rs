#![allow(clippy::non_ascii_literal)]

mod setup;

use katex::types::{Mode, ParseErrorKind};
use setup::*;

#[test]
fn parser_handle_infix_nodes() {
    it("rejects repeated infix operators", || {
        let error = expect!(r"1\over 2\over 3").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::MultipleInfixOperators
        ));
        Ok(())
    });

    it("rejects conflicting infix operators", || {
        let error = expect!(r"1\over 2\choose 3").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::MultipleInfixOperators
        ));
        Ok(())
    });
}

#[test]
fn parser_handle_sup_subscript() {
    it("rejects ^ at end of group", || {
        let error = expect!(r"{1^}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedGroupAfterSymbol { symbol } => {
                assert_eq!(symbol, "^");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("rejects _ at end of input", || {
        let error = expect!("1_").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedGroupAfterSymbol { symbol } => {
                assert_eq!(symbol, "_");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("rejects \\sqrt as argument to ^", || {
        let error = expect!(r"1^\sqrt{2}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::FunctionMissingArguments { func, context } => {
                assert_eq!(func, "\\sqrt");
                assert_eq!(context, "superscript");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_parse_atom() {
    it("rejects \\limits without operator", || {
        let error = expect!(r"\alpha\limits\omega").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::LimitsMustFollowBase
        ));
        Ok(())
    });

    it("rejects \\limits at the beginning of the input", || {
        let error = expect!(r"\limits\omega").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::LimitsMustFollowBase
        ));
        Ok(())
    });

    it("rejects double superscripts", || {
        let error = expect!(r"1^2^3").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::DoubleSuperscript
        ));
        let error = expect!(r"1^{2+3}_4^5").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::DoubleSuperscript
        ));
        Ok(())
    });

    it("rejects double superscripts involving primes", || {
        for expr in [r"1'_2^3", r"1^2'", r"1^2_3'", r"1'_2'"] {
            let error = expect!(expr).parse_error(&strict_settings())?;
            assert!(matches!(
                error.kind.as_ref(),
                ParseErrorKind::DoubleSuperscript
            ));
        }
        Ok(())
    });

    it("rejects double subscripts", || {
        let error = expect!(r"1_2_3").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::DoubleSubscript
        ));
        let error = expect!(r"1_{2+3}^4_5").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::DoubleSubscript
        ));
        Ok(())
    });
}

#[test]
fn parser_parse_implicit_group() {
    it("reports unknown environments", || {
        let error = expect!(r"\begin{foo}bar\end{foo}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::NoSuchEnvironment { name } => assert_eq!(name, "foo"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("reports mismatched environments", || {
        let error =
            expect!(r"\begin{pmatrix}1&2\\3&4\end{bmatrix}+5").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::MismatchedEnvironmentEnd { begin, end } => {
                assert_eq!(begin, "pmatrix");
                assert_eq!(end, "bmatrix");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_parse_function() {
    it("rejects math-mode functions in text mode", || {
        let error = expect!(r"\text{\sqrt2 is irrational}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::FunctionDisallowedInMode { func, mode } => {
                assert_eq!(func, "\\sqrt");
                assert_eq!(mode, &Mode::Text);
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("rejects text-mode-only functions in math mode", || {
        let error = expect!("$").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::FunctionDisallowedInMode { func, mode } => {
                assert_eq!(func, "$");
                assert_eq!(mode, &Mode::Math);
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it(
        "rejects strict-mode text-mode-only functions in math mode",
        || {
            let error = expect!("\\'echec").parse_error(&strict_settings())?;
            match error.kind.as_ref() {
                ParseErrorKind::StrictModeError { message, code } => {
                    assert!(message.contains("accent \\'"));
                    assert_eq!(code, "mathVsTextAccents");
                }
                other => panic!("Unexpected error kind: {other:?}"),
            }
            Ok(())
        },
    );
}

#[test]
fn parser_parse_arguments() {
    it("complains about missing argument at end of input", || {
        let error = expect!(r"2\sqrt").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedGroupAs { context } => {
                assert_eq!(context, "argument to '\\sqrt'");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about missing argument at end of group", || {
        let error = expect!(r"1^{2\sqrt}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedGroupAs { context } => {
                assert_eq!(context, "argument to '\\sqrt'");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about functions as arguments to others", || {
        let error = expect!(r"\sqrt\over2").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::FunctionMissingArguments { func, context } => {
                assert_eq!(func, "\\over");
                assert_eq!(context, "argument to '\\sqrt'");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_parse_group() {
    it("complains about undefined control sequence", || {
        let error = expect!(r"\xyz").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UndefinedControlSequence { name } => assert_eq!(name, "\\xyz"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_verb() {
    it(
        "complains about mismatched \\verb with end of string",
        || {
            let error = expect!(r"\verb|hello").parse_error(&strict_settings())?;
            assert!(matches!(
                error.kind.as_ref(),
                ParseErrorKind::VerbMissingDelimiter
            ));
            Ok(())
        },
    );

    it("complains about mismatched \\verb with end of line", || {
        let error = expect!("\\verb|hello\nworld|").parse_error(&strict_settings())?;
        assert!(matches!(
            error.kind.as_ref(),
            ParseErrorKind::VerbMissingDelimiter
        ));
        Ok(())
    });
}

#[test]
fn parser_expect_calls_parse_input_expecting_eof() {
    it("complains about extra }", || {
        let error = expect!(r"{1+2}}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedToken { expected, found } => {
                assert_eq!(expected, "EOF");
                assert_eq!(found, "}");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about extra \\end", || {
        let error = expect!(r"x\end{matrix}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedToken { expected, found } => {
                assert_eq!(expected, "EOF");
                assert_eq!(found, "\\end");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about top-level &", || {
        let error = expect!("1&2").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedToken { expected, found } => {
                assert_eq!(expected, "EOF");
                assert_eq!(found, "&");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_expect_calls_parse_implicit_group_expecting_right() {
    it("rejects missing \\right", || {
        let error = expect!(r"\left(1+2)").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedToken { expected, found } => {
                assert_eq!(expected, "\\right");
                assert_eq!(found, "EOF");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("rejects incorrectly scoped \\right", || {
        let error = expect!(r"{\left(1+2}\right)").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedToken { expected, found } => {
                assert_eq!(expected, "\\right");
                assert_eq!(found, "}");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_parse_special_group_expecting_braces() {
    it("complains about missing { for color", || {
        let error = expect!(r"\textcolor#ffffff{text}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidColor { color } => assert_eq!(color, "#"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about missing { for size", || {
        let error = expect!(r"\rule{1em}[2em]").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidSize { size } => assert_eq!(size, "["),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about missing } for color", || {
        let error = expect!(r"\textcolor{#ffffff{text}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnexpectedEndOfMacroArgument { expected } => {
                assert_eq!(expected, "}");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about missing ] for size", || {
        let error = expect!(r"\rule[1em{2em}{3em}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnexpectedEndOfMacroArgument { expected } => {
                assert_eq!(expected, "]");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("complains about missing ] for size at end of input", || {
        let error = expect!(r"\rule[1em").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnexpectedEndOfMacroArgument { expected } => {
                assert_eq!(expected, "]");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it(
        "complains about missing } for color at end of input",
        || {
            let error = expect!(r"\textcolor{#123456").parse_error(&strict_settings())?;
            match error.kind.as_ref() {
                ParseErrorKind::UnexpectedEndOfMacroArgument { expected } => {
                    assert_eq!(expected, "}");
                }
                other => panic!("Unexpected error kind: {other:?}"),
            }
            Ok(())
        },
    );
}

#[test]
fn parser_parse_group_expecting_rbrace() {
    it("at end of file", || {
        let error = expect!(r"\sqrt{2").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedToken { expected, found } => {
                assert_eq!(expected, "}");
                assert_eq!(found, "EOF");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn parser_parse_optional_group_expecting_rbrack() {
    it("at end of file", || {
        let error = expect!(r"\sqrt[3").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnexpectedEndOfMacroArgument { expected } => {
                assert_eq!(expected, "]");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("before group", || {
        let error = expect!(r"\sqrt[3{2}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnexpectedEndOfMacroArgument { expected } => {
                assert_eq!(expected, "]");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn environments_parse_array() {
    it("rejects missing \\end", || {
        let error = expect!(r"\begin{matrix}1").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedArrayDelimiter { found } => assert_eq!(found, "EOF"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("rejects incorrectly scoped \\end", || {
        let error = expect!(r"{\begin{matrix}1}\end{matrix}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::ExpectedArrayDelimiter { found } => assert_eq!(found, "}"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn environments_array_environment() {
    it("rejects unknown column types", || {
        let error = expect!(r"\begin{array}{cba}\end{array}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnknownColumnAlignment { alignment } => {
                assert_eq!(alignment, "b");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn functions_delimiter_functions() {
    it("reject invalid opening delimiters", || {
        let error = expect!(r"\bigl 1 + 2 \bigr").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidDelimiterAfter {
                delimiter,
                function,
            } => {
                assert_eq!(delimiter, "textord");
                assert_eq!(function, "\\bigl");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("reject invalid closing delimiters", || {
        let error = expect!(r"\bigl(1+2\bigr=3").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidDelimiterAfter {
                delimiter,
                function,
            } => {
                assert_eq!(delimiter, "atom");
                assert_eq!(function, "\\bigr");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("reject group opening delimiters", || {
        let error = expect!(r"\bigl{(}1+2\bigr)3").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidDelimiterTypeAfter { function } => {
                assert_eq!(function, "\\bigl");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("reject group closing delimiters", || {
        let error = expect!(r"\bigl(1+2\bigr{)}3").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidDelimiterTypeAfter { function } => {
                assert_eq!(function, "\\bigr");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn functions_begin_end() {
    it("reject invalid environment names", || {
        let error = expect!(r"\begin x\end y").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::NoSuchEnvironment { name } => assert_eq!(name, "x"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn lexer_inner_lex() {
    it("rejects lone backslash at end of input", || {
        let error = expect!("\\").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::UnexpectedCharacter { character } => {
                assert_eq!(character, "\\");
            }
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    // JavaScript test suite also covers the case "rejects lone surrogate
    // characters", but Rust strings must be valid UTF-8, so that scenario
    // cannot be represented directly here.
}

#[test]
fn lexer_inner_lex_color() {
    it("reject 3-digit hex notation without #", || {
        let error = expect!(r"\textcolor{1a2}{foo}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidColor { color } => assert_eq!(color, "1a2"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn lexer_inner_lex_size() {
    it("reject size without unit", || {
        let error = expect!(r"\rule{0}{2em}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidSize { size } => assert_eq!(size, "0"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("reject size with bogus unit", || {
        let error = expect!(r"\rule{1au}{2em}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidUnit { unit } => assert_eq!(unit, "au"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });

    it("reject size without number", || {
        let error = expect!(r"\rule{em}{2em}").parse_error(&strict_settings())?;
        match error.kind.as_ref() {
            ParseErrorKind::InvalidSize { size } => assert_eq!(size, "em"),
            other => panic!("Unexpected error kind: {other:?}"),
        }
        Ok(())
    });
}

#[test]
fn unicode_accents() {
    it(
        "should return error for invalid combining characters",
        || {
            let error = expect!("A\u{0328}").parse_error(&strict_settings())?;
            match error.kind.as_ref() {
                ParseErrorKind::UnknownAccent { accent } => assert_eq!(accent, "̨"),
                other => panic!("Unexpected error kind: {other:?}"),
            }
            Ok(())
        },
    );
}
