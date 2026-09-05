mod setup;

use katex::types::{OutputFormat, StrictMode, StrictSetting, TrustSetting};
use katex::{KatexContext, Settings, render_to_string};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixtures {
    revision: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    expression: String,
    output: String,
    #[serde(rename = "displayMode")]
    display_mode: bool,
    expected: String,
}

fn normalize(markup: &str) -> String {
    setup::normalize_html_attributes(&setup::normalize_style_attributes(markup))
}

#[test]
fn pinned_upstream_rendering() {
    let fixtures: Fixtures = serde_json::from_str(include_str!("fixtures/upstream.json")).unwrap();
    assert_eq!(
        fixtures.revision,
        "49904aa2b6c5d82ba0c5a1bc3a4d9b3353a1401c"
    );
    let ctx = KatexContext::default();
    let mut failures = Vec::new();
    for case in fixtures.cases {
        let settings = Settings::builder()
            .output(if case.output == "html" {
                OutputFormat::Html
            } else {
                OutputFormat::Mathml
            })
            .display_mode(case.display_mode)
            .strict(StrictSetting::Mode(StrictMode::Ignore))
            .trust(TrustSetting::Bool(true))
            .build();
        let actual = render_to_string(&ctx, &case.expression, &settings).unwrap();
        let actual = normalize(&actual);
        let expected = normalize(&case.expected);
        if actual != expected {
            let at = actual
                .chars()
                .zip(expected.chars())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| actual.chars().count().min(expected.chars().count()));
            let excerpt = |s: &str| {
                s.chars()
                    .skip(at.saturating_sub(40))
                    .take(220)
                    .collect::<String>()
            };
            failures.push(format!(
                "{} ({}) at {at}\nRust: {}\nJS:   {}",
                case.expression,
                case.output,
                excerpt(&actual),
                excerpt(&expected)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn upstream_argument_validation() {
    let ctx = KatexContext::default();
    let settings = Settings::builder()
        .display_mode(true)
        .trust(TrustSetting::Bool(true))
        .strict(StrictSetting::Mode(StrictMode::Ignore))
        .build();
    for expression in [
        r"\begin{alignedat}{0}x&=y\end{alignedat}",
        r"\begin{alignedat}{-1}x&=y\end{alignedat}",
        r"\begin{alignedat}{1.5}x&=y\end{alignedat}",
        r"\begin{alignedat}{1\frac12}x&=y\end{alignedat}",
        r"\begin{alignedat}{}x&=y\end{alignedat}",
        r"\begin{\frac12}x\end{matrix}",
        r"\begin{matrix~}x\end{matrix}",
        r"\big{(x)}",
        r"\big{}",
        r#"\char"110000"#,
        r"\@char{\frac12}",
        r"\htmlData{missing}{x}",
        r"\htmlData{a=b,missing}{x}",
    ] {
        assert!(
            render_to_string(&ctx, expression, &settings).is_err(),
            "{expression}"
        );
    }
    let strict = Settings::builder()
        .strict(StrictSetting::Mode(StrictMode::Error))
        .build();
    assert!(render_to_string(&ctx, r"\sout{x}", &strict).is_err());
    assert!(render_to_string(&ctx, r"\text{\sout{x}}", &strict).is_ok());
}
