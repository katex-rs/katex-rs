extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use core::{
    error::Error as CoreError,
    fmt::{self, Write as _},
    iter::Peekable,
};
use std::{
    env, fs,
    fs::File,
    io::{BufWriter, Write as _},
    path::PathBuf,
};

type BuildResult<T> = Result<T, Box<dyn CoreError>>;

#[derive(Debug)]
struct BuildScriptError(String);

impl fmt::Display for BuildScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl CoreError for BuildScriptError {}

#[path = "src/unicode/unicode_accents.rs"]
mod unicode_accents;
use unicode_accents::UNICODE_ACCENTS;

#[derive(serde::Deserialize)]
struct Symbol {
    mode: String,
    font: String,
    group: String,
    replace: Option<String>,
    name: String,
    #[serde(rename = "acceptUnicodeChar")]
    accept_unicode_char: bool,
}

fn main() -> BuildResult<()> {
    println!("cargo:rerun-if-changed=data/font_metrics_data.json");
    println!("cargo:rerun-if-changed=data/symbols.json");
    println!("cargo:rerun-if-changed=data/sigmas_and_xis.json");

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    let sigmas = generate_sigmas_and_xis()?;
    write_file(out_dir.join("sigmas_and_xis_generated.rs"), &sigmas)?;

    let font_metrics = generate_font_metrics()?;
    write_file(out_dir.join("font_metrics_data_phf.rs"), &font_metrics)?;

    let unicode_symbols = generate_unicode_symbols()?;
    write_file(out_dir.join("unicode_symbols_phf.rs"), &unicode_symbols)?;

    let symbols = generate_symbols()?;
    write_file(out_dir.join("generated_symbols_data.rs"), &symbols)?;

    Ok(())
}

// Function to convert camelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        if let Some(lower) = ch.to_lowercase().next() {
            result.push(lower);
        }
    }
    result
}

fn generate_sigmas_and_xis() -> BuildResult<String> {
    let data = read_json("data/sigmas_and_xis.json")?;
    let sigmas_and_xis = data
        .get("sigmasAndXis")
        .and_then(|value| value.as_object())
        .ok_or_else(|| BuildScriptError("sigmasAndXis object".into()))?;
    let field_docs = data
        .get("fieldDocs")
        .and_then(|value| value.as_object())
        .ok_or_else(|| BuildScriptError("fieldDocs object".into()))?;

    let mut fields = String::new();
    for (key, _) in sigmas_and_xis {
        let snake_key = to_snake_case(key);
        if let Some(doc) = field_docs.get(key).and_then(|value| value.as_str()) {
            let _ = writeln!(&mut fields, "    /// {doc}");
        }
        let _ = writeln!(&mut fields, "    pub {snake_key}: f64,");
    }
    let _ = writeln!(
        &mut fields,
        "    /// CSS em per mu\n    pub css_em_per_mu: f64,"
    );

    let mut consts = String::new();
    let quad = sigmas_and_xis
        .get("quad")
        .and_then(|value| value.as_array())
        .ok_or_else(|| BuildScriptError("quad should be an array".into()))?;

    for index in 0..3 {
        let _ = writeln!(&mut consts, "    FontMetrics {{");
        for (key, value) in sigmas_and_xis {
            let Some(values) = value.as_array() else {
                continue;
            };
            let Some(entry) = values.get(index) else {
                continue;
            };
            let formatted = if entry.is_i64() {
                entry
                    .as_f64()
                    .map_or_else(|| entry.to_string(), |value| format!("{value:.1}"))
            } else {
                entry.to_string()
            };
            let snake = to_snake_case(key);
            let _ = writeln!(&mut consts, "        {snake}: {formatted},");
        }

        let css_em_per_mu = quad
            .get(index)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| BuildScriptError("quad entry should be a float".into()))?
            / 18.0;
        let _ = writeln!(&mut consts, "        css_em_per_mu: {css_em_per_mu},");
        let _ = writeln!(&mut consts, "    }},");
    }

    Ok(format!(
        "// Auto-generated from KaTeX/src/fontMetrics.js
// Do not edit manually

#[derive(Debug, Clone)]
/// Font metrics for a specific style size (text, script, scriptscript)
pub struct FontMetrics {{
{fields}}}

/// Constant font metrics for textstyle, scriptstyle, and scriptscriptstyle
pub const FONT_METRICS: [FontMetrics; 3] = [
{consts}];
"
    ))
}

fn generate_font_metrics() -> BuildResult<String> {
    use phf_codegen::Map as PhfMap;

    let json_data = fs::read_to_string("data/font_metrics_data.json")?;
    let font_metrics: BTreeMap<String, BTreeMap<String, Vec<f64>>> =
        serde_json::from_str(&json_data)?;

    let mut output = String::new();
    let mut font_index = PhfMap::new();

    for (font_family, metrics) in font_metrics {
        let font_name = font_family.replace(['-', '.'], "_");
        font_index.entry(
            font_family.clone(),
            format!("&{}_METRICS", font_name.to_uppercase()),
        );

        let mut map = PhfMap::new();
        for (char_code_str, metrics_array) in metrics {
            let char_code: u32 = char_code_str.parse()?;
            let values = metrics_array
                .iter()
                .map(|v| {
                    if v.fract() == 0.0 {
                        format!("{v:.1}")
                    } else {
                        v.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            map.entry(char_code, format!("CharacterMetrics::new({values})"));
        }

        let _ = writeln!(
            &mut output,
            "/// Font metrics for the {font_family} font family"
        );
        let _ = writeln!(
            &mut output,
            "#[allow(clippy::expect_used)]\n#[allow(clippy::approx_constant)]\npub const {}_METRICS: phf::Map<u32, CharacterMetrics> = {};\n",
            font_name.to_uppercase(),
            map.build()
        );
    }

    let _ = writeln!(
        &mut output,
        "/// Mapping of font family names to their corresponding metrics maps"
    );
    let _ = writeln!(
        &mut output,
        "#[allow(clippy::expect_used)]\n#[allow(clippy::non_ascii_literal)]\npub const FONT_METRICS_INDEX: phf::Map<&'static str, &'static phf::Map<u32, CharacterMetrics>> = \n{};\n",
        font_index.build()
    );

    Ok(output)
}

#[allow(clippy::unnecessary_wraps)]
fn generate_unicode_symbols() -> BuildResult<String> {
    use phf_codegen::Map as PhfMap;

    let accents: Vec<char> = UNICODE_ACCENTS.keys().copied().collect();
    let letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ\u{3b1}\u{3b2}\u{3b3}\u{3b4}\u{3b5}\u{3f5}\u{3b6}\u{3b7}\u{3b8}\u{3d1}\u{3b9}\u{3ba}\u{3bb}\u{3bc}\u{3bd}\u{3be}\u{3bf}\u{3c0}\u{3d6}\u{3c1}\u{3f1}\u{3c2}\u{3c3}\u{3c4}\u{3c5}\u{3c6}\u{3d5}\u{3c7}\u{3c8}\u{3c9}\u{393}\u{394}\u{398}\u{39b}\u{39e}\u{3a0}\u{3a3}\u{3a5}\u{3a6}\u{3a8}\u{3a9}";

    let mut seen = BTreeSet::new();
    let mut map = PhfMap::<char>::new();

    for letter in letters.chars() {
        for accent in &accents {
            let combined = format!("{letter}{accent}");
            if let Some(normalized) = normalize_to_single_char(&combined)
                && seen.insert(normalized)
            {
                map.entry(normalized, format!("\"{}\"", escape_as_unicode(&combined)));
            }

            for accent2 in &accents {
                if accent == accent2 {
                    continue;
                }

                let combined2 = format!("{letter}{accent2}{accent}");
                if let Some(normalized) = normalize_to_single_char(&combined2)
                    && seen.insert(normalized)
                {
                    map.entry(normalized, format!("\"{}\"", escape_as_unicode(&combined2)));
                }
            }
        }
    }

    Ok(format!(
        "/// Mapping of normalized Unicode symbols to their component parts
/// Unicode symbols map for Modifier tone letters
#[allow(clippy::expect_used)]
#[allow(clippy::non_ascii_literal)]
pub const UNICODE_SYMBOLS: phf::Map<char, &str> = \n{};\n",
        map.build()
    ))
}

fn generate_symbols() -> BuildResult<String> {
    let json_data = fs::read_to_string("data/symbols.json")?;
    let symbol_data: Vec<Symbol> = serde_json::from_str(&json_data)?;

    let (math_symbols, text_symbols): (Vec<Symbol>, Vec<Symbol>) =
        symbol_data.into_iter().partition(|s| s.mode == "math");

    let mut output = String::new();
    let _ = writeln!(&mut output, "// Auto-generated file - do not edit manually");
    let _ = writeln!(&mut output, "// Generated from data/symbols.json\n");
    let _ = writeln!(&mut output, "/// Populate math symbols from JSON data");
    write_symbols(&mut output, &math_symbols, "POPULATE_MATH_SYMBOLS")?;
    let _ = writeln!(&mut output, "/// Populate text symbols from JSON data");
    write_symbols(&mut output, &text_symbols, "POPULATE_TEXT_SYMBOLS")?;

    Ok(output)
}

fn write_file(path: PathBuf, contents: &str) -> BuildResult<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    writer.write_all(contents.as_bytes())?;
    Ok(())
}

fn read_json(path: &str) -> BuildResult<serde_json::Value> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn escape_as_unicode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() && !c.is_control() {
                c.to_string()
            } else {
                format!("\\u{{{:x}}}", c as u32)
            }
        })
        .collect::<String>()
}

fn parse_unicode_escape<I>(iter: &mut Peekable<I>) -> Option<String>
where
    I: Iterator<Item = char> + Clone,
{
    let mut lookahead = iter.clone();
    if lookahead.next()? != '\\' {
        return None;
    }
    if lookahead.next()? != 'u' {
        return None;
    }

    if matches!(lookahead.peek(), Some('{')) {
        lookahead.next();
        let mut hex = String::new();
        while let Some(&c) = lookahead.peek() {
            if c == '}' {
                break;
            }
            if c.is_ascii_hexdigit() {
                hex.push(c);
                lookahead.next();
            } else {
                return None;
            }
        }
        if hex.is_empty() || lookahead.next()? != '}' {
            return None;
        }
        *iter = lookahead;
        return Some(format!("\\u{{{hex}}}"));
    }

    let mut hex = String::new();
    for _ in 0..4 {
        let c = lookahead.next()?;
        if c.is_ascii_hexdigit() {
            hex.push(c);
        } else {
            return None;
        }
    }
    *iter = lookahead;
    Some(format!("\\u{{{hex}}}"))
}

fn convert_unicode_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        if let Some(escaped) = parse_unicode_escape(&mut chars) {
            result.push_str(&escaped);
        } else if let Some(next) = chars.next() {
            result.push(next);
        } else {
            break;
        }
    }
    result
}

fn normalize_to_single_char(s: &str) -> Option<char> {
    use unicode_normalization::UnicodeNormalization as _;

    let normalized: String = s.chars().nfc().collect();
    let mut chars = normalized.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn write_symbols(buffer: &mut String, symbols: &[Symbol], variant: &str) -> BuildResult<()> {
    let mut grouped: BTreeMap<(String, String, Option<String>), BTreeSet<String>> = BTreeMap::new();

    for symbol in symbols {
        let key = (
            symbol.font.clone(),
            symbol.group.clone(),
            symbol.replace.clone(),
        );
        let entry = grouped.entry(key).or_default();
        entry.insert(symbol.name.clone());

        if let (true, Some(replace)) = (symbol.accept_unicode_char, &symbol.replace) {
            entry.insert(replace.clone());
        }
    }

    let _ = writeln!(
        buffer,
        "const {variant}_MAP: phf::Map<&str, CharInfo> = phf_map!("
    );

    for ((font, group, replace), names) in grouped {
        let arm = names
            .into_iter()
            .map(|s| convert_unicode_escapes(&s))
            .collect::<Vec<_>>()
            .join("\" | \"");

        let font_str = match font.as_str() {
            "main" => "Font::Main",
            "ams" => "Font::Ams",
            _ => return Err(BuildScriptError(format!("Invalid font: {font}")).into()),
        };

        let group_str = match group.as_str() {
            "bin" => "Group::Atom(Atom::Bin)",
            "close" => "Group::Atom(Atom::Close)",
            "inner" => "Group::Atom(Atom::Inner)",
            "open" => "Group::Atom(Atom::Open)",
            "punct" => "Group::Atom(Atom::Punct)",
            "rel" => "Group::Atom(Atom::Rel)",
            "accent" | "accent-token" => "Group::NonAtom(NonAtom::AccentToken)",
            "mathord" => "Group::NonAtom(NonAtom::MathOrd)",
            "op" | "op-token" => "Group::NonAtom(NonAtom::OpToken)",
            "spacing" => "Group::NonAtom(NonAtom::Spacing)",
            "textord" => "Group::NonAtom(NonAtom::TextOrd)",
            _ => return Err(BuildScriptError(format!("Invalid group: {group}")).into()),
        };

        let replace_value = replace.as_ref().map_or_else(
            || "None".to_owned(),
            |s| format!("Some(\'{}\')", convert_unicode_escapes(s)),
        );

        let _ = writeln!(buffer, "    \"{arm}\" => CharInfo {{");
        let _ = writeln!(buffer, "        font: {font_str},");
        let _ = writeln!(buffer, "        group: {group_str},");
        let _ = writeln!(buffer, "        replace: {replace_value},");
        let _ = writeln!(buffer, "    }},");
    }

    let _ = writeln!(buffer, ");\n");

    Ok(())
}
