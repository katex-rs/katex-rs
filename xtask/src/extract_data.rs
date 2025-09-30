use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Args;
use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Args, Default)]
pub struct ExtractDataArgs {}

pub fn run(_args: ExtractDataArgs) -> Result<()> {
    let root = project_root();
    let katex_src = root.join("KaTeX").join("src");
    let output_dir = root.join("crates").join("katex").join("data");

    fs::create_dir_all(&output_dir).context("failed to create data output directory")?;

    let font_metrics = extract_font_metrics(&katex_src)?;
    let font_metrics_path = output_dir.join("font_metrics_data.json");
    fs::write(&font_metrics_path, &font_metrics)
        .with_context(|| format!("failed to write {}", font_metrics_path))?;
    println!(
        "Successfully converted to JSON: {} ({} bytes)",
        font_metrics_path,
        font_metrics.len()
    );

    let sigmas_and_docs = extract_sigmas_and_xis(&katex_src)?;
    let sigmas_path = output_dir.join("sigmas_and_xis.json");
    fs::write(&sigmas_path, &sigmas_and_docs)
        .with_context(|| format!("failed to write {}", sigmas_path))?;
    println!(
        "Successfully converted to JSON: {} ({} bytes)",
        sigmas_path,
        sigmas_and_docs.len()
    );

    let (symbols, symbol_count) = extract_symbols(&katex_src)?;
    let symbols_path = output_dir.join("symbols.json");
    fs::write(&symbols_path, &symbols)
        .with_context(|| format!("failed to write {}", symbols_path))?;
    println!("Extracted {} symbols to {}", symbol_count, symbols_path);

    Ok(())
}

fn project_root() -> Utf8PathBuf {
    let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("xtask manifest directory should have a parent")
        .to_owned()
}

fn extract_font_metrics(katex_src: &Utf8Path) -> Result<String> {
    let path = katex_src.join("fontMetricsData.js");
    let contents = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path))?;

    let export_start = contents
        .find("export default")
        .context("could not find `export default` in fontMetricsData.js")?;
    let brace_start = contents[export_start..]
        .find('{')
        .map(|idx| export_start + idx)
        .context("could not locate start of font metrics object")?;
    let object_source = extract_braced_block(&contents, brace_start)?;

    let value: Value =
        json5::from_str(&object_source).context("failed to parse font metrics object as JSON5")?;
    serde_json::to_string_pretty(&value).context("failed to format font metrics JSON")
}

fn extract_sigmas_and_xis(katex_src: &Utf8Path) -> Result<String> {
    let path = katex_src.join("fontMetrics.js");
    let contents = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path))?;

    let marker = "const sigmasAndXis =";
    let marker_start = contents
        .find(marker)
        .context("could not find sigmasAndXis declaration")?;
    let brace_start = contents[marker_start..]
        .find('{')
        .map(|idx| marker_start + idx)
        .context("could not locate start of sigmasAndXis object")?;
    let object_source = extract_braced_block(&contents, brace_start)?;

    let sigmas: Value =
        json5::from_str(&object_source).context("failed to parse sigmasAndXis object as JSON5")?;

    let field_docs = extract_field_docs(&contents);

    let output = json!({
        "sigmasAndXis": sigmas,
        "fieldDocs": field_docs,
    });

    serde_json::to_string_pretty(&output).context("failed to format sigmasAndXis JSON")
}

fn extract_symbols(katex_src: &Utf8Path) -> Result<(String, usize)> {
    let path = katex_src.join("symbols.js");
    let contents = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path))?;

    let regex = Regex::new(
        r#"defineSymbol\(\s*([A-Za-z$_][\w$]*)\s*,\s*([A-Za-z$_][\w$]*)\s*,\s*([A-Za-z$_][\w$]*)\s*,\s*(?:\"((?:[^\"\\]|\\.)*)\"|(null|true|false|[A-Za-z$_][\w$]*))\s*,\s*\"((?:[^\"\\]|\\.)*)\"(?:\s*,\s*([^)]+))?\s*\);"#,
    )
    .expect("invalid regex for defineSymbol extraction");

    let mut symbols = Vec::new();

    for capture in regex.captures_iter(&contents) {
        let mode = capture[1].to_string();
        let font = capture[2].to_string();
        let group = capture[3].to_string();

        let replace = capture.get(4).map(|m| m.as_str().to_string()).or_else(|| {
            capture.get(5).and_then(|m| {
                let value = m.as_str().trim();
                if value.eq_ignore_ascii_case("null") {
                    None
                } else {
                    Some(value.to_string())
                }
            })
        });

        let name = capture
            .get(6)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let accept_unicode = capture
            .get(7)
            .map(|m| m.as_str().trim() == "true")
            .unwrap_or(false);

        symbols.push(Symbol {
            mode,
            font,
            group,
            replace,
            name,
            accept_unicode_char: accept_unicode,
        });
    }

    let count = symbols.len();
    let json = serde_json::to_string_pretty(&symbols).context("failed to format symbols JSON")?;
    Ok((json, count))
}

fn extract_field_docs(contents: &str) -> BTreeMap<String, String> {
    let mut docs = BTreeMap::new();
    let mut current_doc = Vec::new();
    let mut in_block = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        if !in_block {
            if trimmed.starts_with("const sigmasAndXis = {") {
                in_block = true;
            }
            continue;
        }

        if trimmed == "};" {
            break;
        }

        if trimmed.starts_with("//") && !trimmed.contains(':') {
            current_doc.push(trimmed.trim_start_matches("//").trim().to_string());
            continue;
        }

        if let Some((field, _)) = trimmed.split_once(':') {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }

            let inline_comment = trimmed
                .splitn(2, "//")
                .nth(1)
                .map(|comment| comment.trim().to_string());

            let doc_text = match (current_doc.is_empty(), inline_comment) {
                (false, Some(comment)) => {
                    let mut parts = current_doc.join(" ");
                    if !comment.is_empty() {
                        if !parts.is_empty() {
                            parts.push(' ');
                        }
                        parts.push_str(&comment);
                    }
                    parts
                }
                (false, None) => current_doc.join(" "),
                (true, Some(comment)) => comment,
                (true, None) => {
                    current_doc.clear();
                    continue;
                }
            };

            docs.insert(field.to_string(), doc_text);
            current_doc.clear();
        }
    }

    let default_docs = [
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
    ];

    for (key, doc) in default_docs {
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
