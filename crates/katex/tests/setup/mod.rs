#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use std::{
    collections::HashSet,
    panic::{UnwindSafe, catch_unwind},
    sync::OnceLock,
};

#[cfg(feature = "backtrace")]
use btparse::deserialize;

use katex::{
    KatexContext, ParseError, Settings,
    dom_tree::Span,
    options::Options,
    parse,
    parser::parse_node::{ParseNode, ParseNodeArrayTag},
    render_to_dom_tree, render_to_string,
    tree::HtmlDomNode,
    types::ParseErrorKind,
};
use regex::Regex;
use thiserror::Error;

static DEFAULT_CONTEXT: OnceLock<KatexContext> = OnceLock::new();
pub fn default_ctx() -> &'static KatexContext {
    DEFAULT_CONTEXT.get_or_init(KatexContext::default)
}

pub struct TestExpr<'a> {
    pub expr: String,
    pub ctx: &'a KatexContext,
    pub file: &'static str,
    pub line: u32,
    pub code: &'static str,
}

#[derive(Debug, Error)]
pub enum TestError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("Expected parsing to fail for '{expression}'")]
    ExpectedParseFailure { expression: String },
    #[error("Expected building to fail for '{expression}'")]
    ExpectedBuildFailure { expression: String },
    #[error("DOM mismatch between '{left_expr}' and '{right_expr}':\n{left_dom}\n\n{right_dom}")]
    DomMismatch {
        left_expr: String,
        right_expr: String,
        left_dom: String,
        right_dom: String,
    },
    #[error("Expected HTML rendering to fail for '{expression}', but it succeeded: {html}")]
    ExpectedHtmlFailure { expression: String, html: String },
}

pub type TestResult<T> = Result<T, TestError>;

/// Set all `loc` to None for easier comparison
pub fn strip_positions(nodes: &mut [ParseNode]) {
    for node in nodes.iter_mut() {
        match node {
            katex::parser::parse_node::AnyParseNode::Array(parse_node_array) => {
                parse_node_array.loc = None;
                for row in &mut parse_node_array.body {
                    strip_positions(row);
                }
                if let Some(tags) = &mut parse_node_array.tags {
                    for tag in tags {
                        if let ParseNodeArrayTag::Nodes(nodes) = tag {
                            strip_positions(nodes);
                        }
                    }
                }
            }
            katex::parser::parse_node::AnyParseNode::OrdGroup(parse_node_ord_group) => {
                parse_node_ord_group.loc = None;
                strip_positions(&mut parse_node_ord_group.body);
            }
            katex::parser::parse_node::AnyParseNode::SupSub(parse_node_sup_sub) => {
                parse_node_sup_sub.loc = None;
                if let Some(base) = &mut parse_node_sup_sub.base {
                    strip_positions_single(base);
                }
                if let Some(sup) = &mut parse_node_sup_sub.sup {
                    strip_positions_single(sup);
                }
                if let Some(sub) = &mut parse_node_sup_sub.sub {
                    strip_positions_single(sub);
                }
            }
            katex::parser::parse_node::AnyParseNode::Genfrac(parse_node_genfrac) => {
                parse_node_genfrac.loc = None;
                strip_positions_single(&mut parse_node_genfrac.numer);
                strip_positions_single(&mut parse_node_genfrac.denom);
            }
            katex::parser::parse_node::AnyParseNode::LeftRight(parse_node_left_right) => {
                parse_node_left_right.loc = None;
                strip_positions(&mut parse_node_left_right.body);
            }
            katex::parser::parse_node::AnyParseNode::LeftRightRight(
                parse_node_left_right_right,
            ) => {
                parse_node_left_right_right.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Sqrt(parse_node_sqrt) => {
                parse_node_sqrt.loc = None;
                strip_positions_single(&mut parse_node_sqrt.body);
                if let Some(index) = &mut parse_node_sqrt.index {
                    strip_positions_single(index);
                }
            }
            katex::parser::parse_node::AnyParseNode::Atom(parse_node_atom) => {
                parse_node_atom.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::MathOrd(parse_node_math_ord) => {
                parse_node_math_ord.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Op(parse_node_op) => match parse_node_op {
                katex::parser::parse_node::ParseNodeOp::Symbol { loc, .. } => *loc = None,
                katex::parser::parse_node::ParseNodeOp::Body { loc, body, .. } => {
                    *loc = None;
                    strip_positions(body);
                }
            },
            katex::parser::parse_node::AnyParseNode::Spacing(parse_node_spacing) => {
                parse_node_spacing.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Text(parse_node_text) => {
                parse_node_text.loc = None;
                strip_positions(&mut parse_node_text.body);
            }
            katex::parser::parse_node::AnyParseNode::Styling(parse_node_styling) => {
                parse_node_styling.loc = None;
                strip_positions(&mut parse_node_styling.body);
            }
            katex::parser::parse_node::AnyParseNode::Font(parse_node_font) => {
                parse_node_font.loc = None;
                strip_positions_single(&mut parse_node_font.body);
            }
            katex::parser::parse_node::AnyParseNode::Color(parse_node_color) => {
                parse_node_color.loc = None;
                strip_positions(&mut parse_node_color.body);
            }
            katex::parser::parse_node::AnyParseNode::Accent(parse_node_accent) => {
                parse_node_accent.loc = None;
                strip_positions_single(&mut parse_node_accent.base);
            }
            katex::parser::parse_node::AnyParseNode::Overline(parse_node_overline) => {
                parse_node_overline.loc = None;
                strip_positions_single(&mut parse_node_overline.body);
            }
            katex::parser::parse_node::AnyParseNode::Underline(parse_node_underline) => {
                parse_node_underline.loc = None;
                strip_positions_single(&mut parse_node_underline.body);
            }
            katex::parser::parse_node::AnyParseNode::Phantom(parse_node_phantom) => {
                parse_node_phantom.loc = None;
                strip_positions(&mut parse_node_phantom.body);
            }
            katex::parser::parse_node::AnyParseNode::Hphantom(parse_node_hphantom) => {
                parse_node_hphantom.loc = None;
                strip_positions_single(&mut parse_node_hphantom.body);
            }
            katex::parser::parse_node::AnyParseNode::Vphantom(parse_node_vphantom) => {
                parse_node_vphantom.loc = None;
                strip_positions_single(&mut parse_node_vphantom.body);
            }
            katex::parser::parse_node::AnyParseNode::Rule(parse_node_rule) => {
                parse_node_rule.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::CdLabel(parse_node_cd_label) => {
                parse_node_cd_label.loc = None;
                strip_positions_single(&mut parse_node_cd_label.label);
            }
            katex::parser::parse_node::AnyParseNode::CdLabelParent(parse_node_cd_label_parent) => {
                parse_node_cd_label_parent.loc = None;
                strip_positions_single(&mut parse_node_cd_label_parent.fragment);
            }
            katex::parser::parse_node::AnyParseNode::ColorToken(parse_node_color_token) => {
                parse_node_color_token.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Raw(parse_node_raw) => {
                parse_node_raw.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Size(parse_node_size) => {
                parse_node_size.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Tag(parse_node_tag) => {
                parse_node_tag.loc = None;
                strip_positions(&mut parse_node_tag.body);
                strip_positions(&mut parse_node_tag.tag);
            }
            katex::parser::parse_node::AnyParseNode::Url(parse_node_url) => {
                parse_node_url.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Verb(parse_node_verb) => {
                parse_node_verb.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::TextOrd(parse_node_text_ord) => {
                parse_node_text_ord.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::AccentToken(parse_node_accent_token) => {
                parse_node_accent_token.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::OpToken(parse_node_op_token) => {
                parse_node_op_token.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::AccentUnder(parse_node_accent_under) => {
                parse_node_accent_under.loc = None;
                strip_positions_single(&mut parse_node_accent_under.base);
            }
            katex::parser::parse_node::AnyParseNode::Cr(parse_node_cr) => {
                parse_node_cr.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Delimsizing(parse_node_delimsizing) => {
                parse_node_delimsizing.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Enclose(parse_node_enclose) => {
                parse_node_enclose.loc = None;
                strip_positions_single(&mut parse_node_enclose.body);
            }
            katex::parser::parse_node::AnyParseNode::Environment(parse_node_environment) => {
                parse_node_environment.loc = None;
                strip_positions_single(&mut parse_node_environment.name_group);
            }
            katex::parser::parse_node::AnyParseNode::Hbox(parse_node_hbox) => {
                parse_node_hbox.loc = None;
                strip_positions(&mut parse_node_hbox.body);
            }
            katex::parser::parse_node::AnyParseNode::HorizBrace(parse_node_horiz_brace) => {
                parse_node_horiz_brace.loc = None;
                strip_positions_single(&mut parse_node_horiz_brace.base);
            }
            katex::parser::parse_node::AnyParseNode::Href(parse_node_href) => {
                parse_node_href.loc = None;
                strip_positions(&mut parse_node_href.body);
            }
            katex::parser::parse_node::AnyParseNode::Html(parse_node_html) => {
                parse_node_html.loc = None;
                strip_positions(&mut parse_node_html.body);
            }
            katex::parser::parse_node::AnyParseNode::HtmlMathMl(parse_node_html_math_ml) => {
                parse_node_html_math_ml.loc = None;
                strip_positions(&mut parse_node_html_math_ml.html);
                strip_positions(&mut parse_node_html_math_ml.mathml);
            }
            katex::parser::parse_node::AnyParseNode::Includegraphics(
                parse_node_includegraphics,
            ) => {
                parse_node_includegraphics.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Infix(parse_node_infix) => {
                parse_node_infix.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Internal(parse_node_internal) => {
                parse_node_internal.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Kern(parse_node_kern) => {
                parse_node_kern.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Lap(parse_node_lap) => {
                parse_node_lap.loc = None;
                strip_positions_single(&mut parse_node_lap.body);
            }
            katex::parser::parse_node::AnyParseNode::MathChoice(parse_node_math_choice) => {
                parse_node_math_choice.loc = None;
                strip_positions(&mut parse_node_math_choice.display);
                strip_positions(&mut parse_node_math_choice.text);
                strip_positions(&mut parse_node_math_choice.script);
                strip_positions(&mut parse_node_math_choice.scriptscript);
            }
            katex::parser::parse_node::AnyParseNode::Middle(parse_node_middle) => {
                parse_node_middle.loc = None;
            }
            katex::parser::parse_node::AnyParseNode::Mclass(parse_node_mclass) => {
                parse_node_mclass.loc = None;
                strip_positions(&mut parse_node_mclass.body);
            }
            katex::parser::parse_node::AnyParseNode::OperatorName(parse_node_operator_name) => {
                parse_node_operator_name.loc = None;
                strip_positions(&mut parse_node_operator_name.body);
            }
            katex::parser::parse_node::AnyParseNode::Pmb(parse_node_pmb) => {
                parse_node_pmb.loc = None;
                strip_positions(&mut parse_node_pmb.body);
            }
            katex::parser::parse_node::AnyParseNode::Raisebox(parse_node_raisebox) => {
                parse_node_raisebox.loc = None;
                strip_positions_single(&mut parse_node_raisebox.body);
            }
            katex::parser::parse_node::AnyParseNode::Sizing(parse_node_sizing) => {
                parse_node_sizing.loc = None;
                strip_positions(&mut parse_node_sizing.body);
            }
            katex::parser::parse_node::AnyParseNode::Smash(parse_node_smash) => {
                parse_node_smash.loc = None;
                strip_positions_single(&mut parse_node_smash.body);
            }
            katex::parser::parse_node::AnyParseNode::Vcenter(parse_node_vcenter) => {
                parse_node_vcenter.loc = None;
                strip_positions_single(&mut parse_node_vcenter.body);
            }
            katex::parser::parse_node::AnyParseNode::XArrow(parse_node_xarrow) => {
                parse_node_xarrow.loc = None;
                if let Some(body) = &mut parse_node_xarrow.body {
                    strip_positions_single(body);
                }
                if let Some(below) = &mut parse_node_xarrow.below {
                    strip_positions_single(below);
                }
            }
        }
    }
}

/// Helper function to strip positions from a single node
fn strip_positions_single(node: &mut ParseNode) {
    let mut temp_vec = vec![node.clone()];
    strip_positions(&mut temp_vec);
    *node = temp_vec.into_iter().next().unwrap();
}

impl TestExpr<'_> {
    pub fn to_parse(&self, settings: &Settings) -> TestResult<()> {
        parse(self.ctx, &self.expr, settings)?;
        Ok(())
    }

    pub fn not_to_parse(self, settings: &Settings) -> TestResult<()> {
        match parse(self.ctx, &self.expr, settings) {
            Ok(_) => Err(TestError::ExpectedParseFailure {
                expression: self.expr,
            }),
            Err(_) => Ok(()),
        }
    }

    pub fn to_parse_like(&self, other: &str, settings: &Settings) -> TestResult<()> {
        // Compare parse trees for equivalence
        let mut tree1 = parse(self.ctx, &self.expr, settings)?;
        let mut tree2 = parse(self.ctx, other, settings)?;

        strip_positions(&mut tree1);
        strip_positions(&mut tree2);

        assert_eq!(
            tree1, tree2,
            "Parse trees do not match between '{}' and '{}'",
            self.expr, other
        );
        Ok(())
    }

    pub fn to_build(&self, settings: &Settings) -> TestResult<()> {
        render_to_dom_tree(self.ctx, &self.expr, settings)?;
        Ok(())
    }

    pub fn not_to_build(self, settings: &Settings) -> TestResult<()> {
        match render_to_dom_tree(self.ctx, &self.expr, settings) {
            Ok(_) => Err(TestError::ExpectedBuildFailure {
                expression: self.expr,
            }),
            Err(_) => Ok(()),
        }
    }

    pub fn to_build_like(&self, other: &str, settings: &Settings) -> TestResult<()> {
        // Compare rendered DOM trees for equivalence
        let dom1 = render_to_dom_tree(self.ctx, &self.expr, settings)?;
        let dom2 = render_to_dom_tree(self.ctx, other, settings)?;

        // Simple comparison - in a real implementation, we'd do proper DOM comparison
        let dom1_debug = format!("{:?}", dom1);
        let dom2_debug = format!("{:?}", dom2);
        if dom1_debug == dom2_debug {
            Ok(())
        } else {
            Err(TestError::DomMismatch {
                left_expr: self.expr.clone(),
                right_expr: other.to_owned(),
                left_dom: dom1_debug,
                right_dom: dom2_debug,
            })
        }
    }

    pub fn not_to_html(self, settings: &Settings) -> TestResult<()> {
        match render_to_string(self.ctx, &self.expr, settings) {
            Ok(html) => Err(TestError::ExpectedHtmlFailure {
                expression: self.expr,
                html,
            }),
            Err(_) => Ok(()),
        }
    }
}

pub fn expect_impl(
    expr: &str,
    file: &'static str,
    line: u32,
    code: &'static str,
) -> TestExpr<'static> {
    TestExpr {
        expr: expr.to_string(),
        ctx: default_ctx(),
        file,
        line,
        code,
    }
}

#[macro_export]
macro_rules! expect {
    ($e:expr) => {
        $crate::expect_impl($e, file!(), line!(), stringify!($e))
    };
}

#[cfg(feature = "backtrace")]
fn format_backtrace(bt_serialized: &Backtrace) -> String {
    let mut traces = Vec::new();
    let backtrace = deserialize(bt_serialized).unwrap();
    for frame in backtrace.frames {
        let function = frame.function;
        if function.starts_with("katex::types::parse_error::ParseError")
            || function.starts_with("core::ops::function::FnOnce")
        {
            continue;
        }
        if function.starts_with("std::panicking") {
            break;
        }
        let loc = match (frame.file, frame.line) {
            (Some(file), Some(line)) => format!("{}:{}", file, line),
            (Some(file), None) => file,
            _ => "<unknown>".to_string(),
        };
        traces.push(format!("at {} ({})", function, loc));
    }
    traces.join("\n")
}

fn panic_with_error(desc: &str, error: TestError) -> ! {
    match error {
        TestError::Parse(parse_error) => panic_with_parse_error(desc, parse_error),
        other => panic!("Test '{}' failed with Result::Err: {}", desc, other),
    }
}

#[cfg(feature = "backtrace")]
fn panic_with_parse_error(desc: &str, error: ParseError) -> ! {
    let traces = format_backtrace(&error.backtrace);
    panic!(
        "Test '{}' failed with Result::Err: {}\nBacktrace:\n{}",
        desc, error, traces
    );
}

#[cfg(not(feature = "backtrace"))]
fn panic_with_parse_error(desc: &str, error: ParseError) -> ! {
    panic!("Test '{}' failed with Result::Err: {}", desc, error);
}

pub fn it<F>(desc: &str, test_fn: F)
where
    F: FnOnce() -> TestResult<()> + UnwindSafe,
{
    let old_hook = std::panic::take_hook();
    //std::panic::set_hook(Box::new(|_| {}));

    let result = catch_unwind(test_fn);
    std::panic::set_hook(old_hook);

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic_with_error(desc, e),
        Err(panic_payload) => {
            let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "<non-string panic payload>".to_string()
            };
            panic!(
                "Test '{}' panicked: {}\n(no ParseError backtrace, since this was a raw panic)",
                desc, msg
            );
        }
    }
}

/// Settings helpers
pub fn strict_settings() -> Settings {
    Settings::builder()
        .throw_on_error(true)
        .strict(katex::StrictSetting::Bool(true))
        .build()
}

pub fn nonstrict_settings() -> Settings {
    Settings::builder()
        .throw_on_error(false)
        .strict(katex::StrictSetting::Bool(false))
        .build()
}

pub fn trust_settings() -> Settings {
    Settings::builder()
        .trust(katex::TrustSetting::Bool(true))
        .build()
}

pub fn trust_non_strict_settings() -> Settings {
    Settings::builder()
        .trust(katex::TrustSetting::Bool(true))
        .strict(katex::StrictSetting::Bool(false))
        .build()
}

pub fn display_settings() -> Settings {
    Settings::builder().display_mode(true).build()
}

pub fn non_display_settings() -> Settings {
    Settings::builder().display_mode(false).build()
}

pub fn get_parsed(expr: &str, settings: &Settings) -> TestResult<Vec<ParseNode>> {
    parse(default_ctx(), expr, settings).map_err(Into::into)
}

pub fn get_parsed_strict(expr: &str) -> TestResult<Vec<ParseNode>> {
    let settings = strict_settings();
    parse(default_ctx(), expr, &settings).map_err(Into::into)
}

pub fn get_parsed_trust(expr: &str) -> TestResult<Vec<ParseNode>> {
    let settings = trust_settings();
    parse(default_ctx(), expr, &settings).map_err(Into::into)
}

pub fn get_built(expr: &str, settings: &Settings) -> TestResult<Vec<HtmlDomNode>> {
    let mut root_node = render_to_dom_tree(default_ctx(), expr, settings)?;

    if root_node.classes.contains("katex-display") {
        if let Some(HtmlDomNode::DomSpan(first_child)) = root_node.children.get_mut(0) {
            root_node = first_child.clone();
        } else {
            return Err(ParseError::new(ParseErrorKind::ExpectedFirstChildDomSpan).into());
        }
    }
    let built_html = if let Some(HtmlDomNode::DomSpan(first_child)) = root_node.children.get_mut(1)
    {
        first_child
    } else {
        return Err(ParseError::new(ParseErrorKind::ExpectedSecondChildDomSpan).into());
    };
    let mut children = Vec::new();
    for child in built_html.children.iter() {
        if let HtmlDomNode::DomSpan(span) = child {
            for grandchild in &span.children {
                if !grandchild.classes().contains("strut") {
                    children.push(grandchild.clone());
                }
            }
        }
    }

    Ok(children)
}

pub fn build_mathml(expr: &str) -> TestResult<Span<HtmlDomNode>> {
    let tree = get_parsed_strict(expr)?;
    katex::build_mathml::build_mathml(
        default_ctx(),
        &tree,
        expr,
        &Options::default(),
        false,
        false,
    )
    .map_err(Into::into)
}

pub fn render_to_string_strict(expr: &str) -> TestResult<String> {
    let settings = strict_settings();
    render_to_string(default_ctx(), expr, &settings).map_err(Into::into)
}

pub fn render_to_string_nonstrict(expr: &str) -> TestResult<String> {
    let settings = nonstrict_settings();
    render_to_string(default_ctx(), expr, &settings).map_err(Into::into)
}

pub fn render_to_string_trust(expr: &str) -> TestResult<String> {
    let settings = trust_settings();
    render_to_string(default_ctx(), expr, &settings).map_err(Into::into)
}

#[macro_export]
macro_rules! assert_let {
    ($pat:pat = $expr:expr, $($arg:tt)+) => {
        let $pat = $expr else {
            panic!($($arg)+);
        };
    };
    ($pat:pat = $expr:expr) => {
        let $pat = $expr else {
            panic!(
                "assert_let failed: pattern `{}` did not match {} at {}:{} \n{:#?}",
                stringify!($pat), stringify!($expr),
                file!(), line!(),
                $expr
            );
        };
    };
}

/// Helper for comparing HTML with unordered style attributes
pub fn assert_html_eq_unordered_styles(markup: &str, rendered: &str) {
    // Since style attributes order may vary, we should extract and compare them
    // separately style=\"...\"
    let style_regex = Regex::new(r#"style="([^"]*)""#).unwrap();
    // seperate the style attributes from the rest of the markup
    let clean_markup = style_regex.replace_all(markup, "style=\"\"");
    let clean_rendered = style_regex.replace_all(rendered, "style=\"\"");
    assert_eq!(clean_markup, clean_rendered);

    // Now compare the style attributes, both may occur multiple times, so we need
    // to find all
    let mut markup_styles = style_regex.captures_iter(markup);
    let mut rendered_styles = style_regex.captures_iter(rendered);
    loop {
        let caps_markup = markup_styles.next();
        let caps_rendered = rendered_styles.next();
        if caps_markup.is_none() && caps_rendered.is_none() {
            break;
        }
        if let (Some(caps_markup), Some(caps_rendered)) = (caps_markup, caps_rendered) {
            let style_markup: HashSet<&str> = caps_markup[1]
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            let style_rendered: HashSet<&str> = caps_rendered[1]
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(style_markup, style_rendered);
        } else {
            panic!("Mismatch in style attributes between markup and rendered output");
        }
    }
}
