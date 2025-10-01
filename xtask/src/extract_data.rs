use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use clap::Args;
use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Args, Default)]
pub struct ExtractDataArgs {}

pub fn run(_args: ExtractDataArgs) -> Result<()> {
    let root = project_root();
    let katex_src = root.join("KaTeX").join("src");
    let output_dir = root.join("crates").join("katex").join("data");

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir))?;

    write_pretty_json(
        output_dir.join("font_metrics_data.json"),
        extract_font_metrics(&katex_src)?,
    )?;

    write_pretty_json(
        output_dir.join("sigmas_and_xis.json"),
        extract_sigmas_and_xis(&katex_src)?,
    )?;

    let (symbols, count) = extract_symbols(&katex_src)?;
    write_pretty_json(output_dir.join("symbols.json"), &symbols)?;
    println!("Extracted {} symbols", count);

    Ok(())
}

fn project_root() -> Utf8PathBuf {
    let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("xtask manifest directory should have a parent")
        .to_owned()
}

fn extract_font_metrics(katex_src: &Utf8Path) -> Result<Value> {
    let path = katex_src.join("fontMetricsData.js");
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("failed to read {}", path))?;

    let object_source = extract_object_after(&contents, "export default")?;
    json5::from_str(&object_source).context("failed to parse font metrics object as JSON5")
}

fn extract_sigmas_and_xis(katex_src: &Utf8Path) -> Result<Value> {
    let path = katex_src.join("fontMetrics.js");
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("failed to read {}", path))?;

    let sigmas_source = extract_object_after(&contents, "const sigmasAndXis =")?;
    let sigmas: Value =
        json5::from_str(&sigmas_source).context("failed to parse sigmasAndXis object as JSON5")?;

    Ok(json!({
        "sigmasAndXis": sigmas,
        "fieldDocs": extract_field_docs(&contents),
    }))
}

fn extract_symbols(katex_src: &Utf8Path) -> Result<(Vec<Symbol>, usize)> {
    let path = katex_src.join("symbols.js");
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("failed to read {}", path))?;

    let regex = Regex::new(
        r#"defineSymbol\(\s*([A-Za-z$_][\w$]*)\s*,\s*([A-Za-z$_][\w$]*)\s*,\s*([A-Za-z$_][\w$]*)\s*,\s*(?:\"((?:[^\"\\]|\\.)*)\"|(null|true|false|[A-Za-z$_][\w$]*))\s*,\s*\"((?:[^\"\\]|\\.)*)\"(?:\s*,\s*([^)]+))?\s*\);"#,
    )
    .expect("invalid regex for defineSymbol extraction");

    let symbols: Vec<_> = regex
        .captures_iter(&contents)
        .map(|capture| Symbol {
            mode: capture[1].to_string(),
            font: capture[2].to_string(),
            group: capture[3].to_string(),
            replace: capture
                .get(4)
                .map(|m| m.as_str().to_string())
                .or_else(|| map_literal_value(capture.get(5))),
            name: capture
                .get(6)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
            accept_unicode_char: capture
                .get(7)
                .map(|m| m.as_str().trim() == "true")
                .unwrap_or(false),
        })
        .collect();

    let count = symbols.len();
    Ok((symbols, count))
}

fn extract_field_docs(contents: &str) -> BTreeMap<String, String> {
    let mut docs = BTreeMap::new();
    let mut active_comment = Vec::new();
    let mut seen_fields = BTreeSet::new();
    let mut in_block = false;

    for line in contents.lines().map(str::trim) {
        if !in_block {
            in_block = line.starts_with("const sigmasAndXis = {");
            continue;
        }

        if line == "};" {
            break;
        }

        if let Some(comment) = line.strip_prefix("//") {
            if !line.contains(':') {
                active_comment.push(comment.trim().to_string());
                continue;
            }
        }

        let Some((field, _)) = line.split_once(':') else {
            continue;
        };

        let field = field.trim();
        if field.is_empty() || !seen_fields.insert(field.to_string()) {
            active_comment.clear();
            continue;
        }

        let inline_comment = line.split_once("//").map(|(_, rest)| rest.trim());
        let mut doc = active_comment.join(" ");
        if let Some(comment) = inline_comment {
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(comment);
        }

        if !doc.is_empty() {
            docs.insert(field.to_string(), doc);
        }

        active_comment.clear();
    }

    for (key, doc) in default_field_docs() {
        docs.entry(key.to_string())
            .or_insert_with(|| doc.to_string());
    }

    docs
}

fn extract_braced_block(contents: &str, start: usize) -> Result<String> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut string_delim = '\0';
    let mut escape = false;
    let mut start_idx = None;

    for (offset, ch) in contents[start..].char_indices() {
        let idx = start + offset;

        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => {
                    escape = true;
                }
                _ if ch == string_delim => {
                    in_string = false;
                }
                _ => {}
            }
            continue;
        }

        match ch {
            '\'' | '"' => {
                in_string = true;
                string_delim = ch;
            }
            '{' => {
                if depth == 0 {
                    start_idx = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    bail!("encountered closing brace without matching opening brace");
                }
                depth -= 1;
                if depth == 0 {
                    let start_idx = start_idx.expect("start index set when entering brace block");
                    return Ok(contents[start_idx..=idx].to_string());
                }
            }
            _ => {}
        }
    }

    bail!("unterminated brace-delimited block")
}

#[derive(Serialize)]
struct Symbol {
    mode: String,
    font: String,
    group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    replace: Option<String>,
    name: String,
    #[serde(rename = "acceptUnicodeChar")]
    accept_unicode_char: bool,
}

fn extract_object_after(contents: &str, marker: &str) -> Result<String> {
    let start = contents
        .find(marker)
        .with_context(|| format!("could not find `{marker}`"))?;
    let brace_start = contents[start..]
        .find('{')
        .map(|offset| start + offset)
        .context("could not locate opening brace")?;
    extract_braced_block(contents, brace_start)
}

fn write_pretty_json<T>(path: Utf8PathBuf, value: T) -> Result<()>
where
    T: Serialize,
{
    let mut file = File::create(&path).with_context(|| format!("failed to write {}", path))?;
    let json = serde_json::to_vec_pretty(&value).context("failed to serialize JSON")?;
    file.write_all(&json)
        .with_context(|| format!("failed to write {}", path))?;
    println!("Successfully wrote {} ({} bytes)", path, json.len());
    Ok(())
}

fn map_literal_value(value: Option<regex::Match<'_>>) -> Option<String> {
    value.and_then(|m| {
        let value = m.as_str().trim();
        if value.eq_ignore_ascii_case("null") {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn default_field_docs() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "sqrtRuleThickness",
            "The \\sqrt rule width is taken from the height of the surd character. Since we use the same font at all sizes, this thickness doesn't scale.",
        ),
        (
            "ptPerEm",
            "This value determines how large a pt is, for metrics which are defined in terms of pts. This value is also used in katex.scss; if you change it make sure the values match.",
        ),
        (
            "doubleRuleSep",
            "The space between adjacent `|` columns in an array definition. From `\\showthe\\doublerulesep` in LaTeX. Equals 2.0 / ptPerEm.",
        ),
        (
            "arrayRuleWidth",
            "The width of separator lines in {array} environments. From `\\showthe\\arrayrulewidth` in LaTeX. Equals 0.4 / ptPerEm.",
        ),
        ("fboxsep", "Two values from LaTeX source2e: 3 pt / ptPerEm"),
        (
            "fboxrule",
            "Two values from LaTeX source2e: 0.4 pt / ptPerEm",
        ),
    ]
}
