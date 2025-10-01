#![allow(clippy::non_ascii_literal)]

mod setup;

use katex::types::{Settings, StrictSetting, TrustSetting};
use setup::*;

#[test]
fn a_mathml_builder() {
    it("should generate the right types of nodes", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\sin{x}+1\;\text{a}", &settings)?;
        insta::assert_snapshot!("mathml_spec__right_types_of_nodes", markup);
        Ok(())
    });

    it("should concatenate digits into single <mn>", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\sin{\alpha}=0.34=.34^1", &settings)?;
        insta::assert_snapshot!("mathml_spec__concatenate_digits", markup);
        let markup = mathml_markup(r"1{,}000{,}000", &settings)?;
        insta::assert_snapshot!("mathml_spec__concatenate_digits_commas", markup);
        Ok(())
    });

    it("should make prime operators into <mo> nodes", || {
        let settings = Settings::default();
        let markup = mathml_markup("f'", &settings)?;
        insta::assert_snapshot!("mathml_spec__prime_operator", markup);
        Ok(())
    });

    it("should generate <mphantom> nodes for \\phantom", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\phantom{x}", &settings)?;
        insta::assert_snapshot!("mathml_spec__phantom", markup);
        Ok(())
    });

    it("should use <munderover> for large operators", || {
        let mut settings = Settings::default();
        settings.display_mode = true;
        let markup = mathml_markup(r"\displaystyle\sum_a^b", &settings)?;
        insta::assert_snapshot!("mathml_spec__munderover_large_operators", markup);
        Ok(())
    });

    it("should use <msupsub> for integrals", || {
        let mut settings = Settings::default();
        settings.display_mode = true;
        let markup = mathml_markup(
            r"\displaystyle\int_a^b + \oiint_a^b + \oiiint_a^b",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__msupsub_integrals", markup);
        Ok(())
    });

    it("should use <msupsub> for regular operators", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\textstyle\sum_a^b", &settings)?;
        insta::assert_snapshot!("mathml_spec__msupsub_regular_operators", markup);
        Ok(())
    });

    it(
        "should output \\limsup_{x \\rightarrow \\infty} correctly in \\textstyle",
        || {
            let settings = Settings::default();
            let markup = mathml_markup(r"\limsup_{x \\rightarrow \\infty}", &settings)?;
            insta::assert_snapshot!("mathml_spec__limsup_textstyle", markup);
            Ok(())
        },
    );

    it(
        "should output \\limsup_{x \\rightarrow \\infty} in displaymode correctly",
        || {
            let settings = Settings::builder().display_mode(true).build();
            let markup = mathml_markup(r"\limsup_{x \\rightarrow \\infty}", &settings)?;
            insta::assert_snapshot!("mathml_spec__limsup_displaymode", markup);
            Ok(())
        },
    );

    it("should use <mpadded> for raisebox", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\raisebox{0.25em}{b}", &settings)?;
        insta::assert_snapshot!("mathml_spec__raisebox", markup);
        Ok(())
    });

    it("should size delimiters correctly", || {
        let settings = Settings::default();
        let markup = mathml_markup(
            r"(M) \\big(M\\big) \\Big(M\\Big) \\bigg(M\\bigg) \\Bigg(M\\Bigg)",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__sized_delimiters", markup);
        Ok(())
    });

    it("should use <menclose> for colorbox", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\colorbox{red}{b}", &settings)?;
        insta::assert_snapshot!("mathml_spec__menclose_colorbox", markup);
        Ok(())
    });

    it("should build the CD environment properly", || {
        let settings = Settings::builder()
            .display_mode(true)
            .strict(StrictSetting::Bool(false))
            .build();
        let markup = mathml_markup(
            r"\begin{CD} A @>a>> B\\\\ @VVbV @VVcV\\\\ C @>d>> D \end{CD}",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__cd_environment", markup);
        Ok(())
    });

    it("should set href attribute for href appropriately", || {
        let settings = Settings::builder().trust(TrustSetting::Bool(true)).build();
        let markup = mathml_markup(r"\href{http://example.org}{\alpha}", &settings)?;
        insta::assert_snapshot!("mathml_spec__href_trusted", markup);

        let default_settings = Settings::default();
        let _ = mathml_markup(
            r"p \Vdash \beta \href{http://example.org}{+ \alpha} \times \gamma",
            &default_settings,
        )?;
        Ok(())
    });

    it("should render mathchoice as if there was nothing", || {
        let settings = Settings::default();
        let markup = mathml_markup(
            r"\displaystyle\mathchoice{\sum_{k = 0}^{\infty} x^k}{T}{S}{SS}",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__mathchoice_display", markup);
        let markup = mathml_markup(
            r"\mathchoice{D}{\sum_{k = 0}^{\infty} x^k}{S}{SS}",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__mathchoice_inline", markup);
        let markup = mathml_markup(
            r"x_{\mathchoice{D}{T}{\sum_{k = 0}^{\infty} x^k}{SS}}",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__mathchoice_subscript", markup);
        let markup = mathml_markup(
            r"x_{y_{\mathchoice{D}{T}{S}{\sum_{k = 0}^{\infty} x^k}}}",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__mathchoice_nested", markup);
        Ok(())
    });

    it(
        "should render boldsymbol with the correct mathvariants",
        || {
            let settings = Settings::default();
            let markup = mathml_markup(r"\boldsymbol{Ax2k\omega\Omega\imath+}", &settings)?;
            insta::assert_snapshot!("mathml_spec__boldsymbol_mathvariants", markup);
            Ok(())
        },
    );

    it(
        "accents turn into <mover accent=\"true\"> in MathML",
        || {
            let settings = Settings::builder()
                .strict(StrictSetting::Bool(false))
                .build();
            let markup = mathml_markup("über fiancée", &settings)?;
            insta::assert_snapshot!("mathml_spec__accent_mover", markup);
            Ok(())
        },
    );

    it("tags use <mlabeledtr>", || {
        let settings = Settings::builder().display_mode(true).build();
        let markup = mathml_markup(r"\tag{hi} x+y^2", &settings)?;
        insta::assert_snapshot!("mathml_spec__tag_mlabeledtr", markup);
        Ok(())
    });

    it("normal spaces render normally", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\kern1em\kern1ex", &settings)?;
        insta::assert_snapshot!("mathml_spec__normal_spaces", markup);
        Ok(())
    });

    it("special spaces render specially", || {
        let settings = Settings::default();
        let markup = mathml_markup(
            r"\,\thinspace\:\>\medspace\;\thickspace\!\negthinspace\negmedspace\negthickspace\mkern1mu\mkern3mu\mkern4mu\mkern5mu\mkern-1mu\mkern-3mu\mkern-4mu\mkern-5mu",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__special_spaces", markup);
        Ok(())
    });

    it("ligatures render properly", || {
        let settings = Settings::default();
        let markup = mathml_markup(
            r"\text{```Hi----'''}`--\texttt{```Hi----'''}`\text{\tt ```Hi----'''}`",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__ligatures", markup);
        Ok(())
    });

    it("\\text fonts become mathvariant", || {
        let settings = Settings::default();
        let markup = mathml_markup(
            r"\text{roman\textit{italic\textbf{bold italic}}\textbf{bold}\textsf{ss\textit{italic\textbf{bold italic}}\textbf{bold}}\texttt{tt\textit{italic\textbf{bold italic}}\textbf{bold}}}",
            &settings,
        )?;
        insta::assert_snapshot!("mathml_spec__text_fonts_mathvariant", markup);
        Ok(())
    });

    it("\\html@mathml makes clean symbols", || {
        let settings = Settings::default();
        let markup = mathml_markup(r"\copyright\neq\notin\u2258\KaTeX", &settings)?;
        insta::assert_snapshot!("mathml_spec__htmlmathml_clean_symbols", markup);
        Ok(())
    });
}
