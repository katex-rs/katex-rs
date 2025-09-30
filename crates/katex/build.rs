use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

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

fn main() {
    println!("cargo:rerun-if-changed=data/font_metrics_data.json");
    println!("cargo:rerun-if-changed=data/symbols.json");
    println!("cargo:rerun-if-changed=data/sigmas_and_xis.json");

    let out_dir = env::var("OUT_DIR").unwrap();

    // Generate sigmas and xis
    generate_sigmas_and_xis(&out_dir);

    // Generate font metrics
    generate_font_metrics(&out_dir);

    // Generate unicode symbols
    generate_unicode_symbols(&out_dir);

    // Generate symbols
    generate_symbols(&out_dir);
}

// Function to convert camelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

fn generate_sigmas_and_xis(out_dir: &str) {
    let json_data =
        fs::read_to_string("data/sigmas_and_xis.json").expect("Failed to read sigmas_and_xis.json");

    let data: serde_json::Value =
        serde_json::from_str(&json_data).expect("Failed to parse sigmas_and_xis.json");

    let sigmas_and_xis = data["sigmasAndXis"]
        .as_object()
        .expect("sigmasAndXis should be an object");

    let field_docs = data["fieldDocs"]
        .as_object()
        .expect("fieldDocs should be an object");

    // Generate Rust struct fields with documentation
    let mut fields = Vec::new();
    for (key, _value) in sigmas_and_xis {
        let snake_key = to_snake_case(key);

        let doc = if let Some(field_doc) = field_docs.get(key)
            && let Some(doc_str) = field_doc.as_str()
        {
            format!("    /// {doc_str}\n")
        } else {
            String::new()
        };

        fields.push(format!("{doc}    pub {snake_key}: f64,"));
    }
    fields.push("    /// CSS em per mu\n    pub css_em_per_mu: f64,".to_string());

    // Generate the const array
    let mut const_items = Vec::new();
    for size_index in 0..3 {
        let mut field_values = Vec::new();
        for (key, value) in sigmas_and_xis {
            let snake_key = to_snake_case(key);
            if let Some(array) = value.as_array()
                && let Some(val) = array.get(size_index)
            {
                let formatted_value = if val.is_i64() {
                    format!("{:.1}", val.as_f64().unwrap())
                } else {
                    val.to_string()
                };
                field_values.push(format!("        {snake_key}: {formatted_value},"));
            }
        }

        // Calculate css_em_per_mu
        if let Some(quad_array) = sigmas_and_xis.get("quad")
            && let Some(quad_val) = quad_array.as_array().unwrap().get(size_index)
        {
            let css_em_per_mu = quad_val.as_f64().unwrap() / 18.0;
            field_values.push(format!("        css_em_per_mu: {css_em_per_mu},"));
        }

        const_items.push(format!(
            "    FontMetrics {{\n{}\n    }}",
            field_values.join("\n")
        ));
    }

    // Generate the full Rust code
    let rust_code = format!(
        "// Auto-generated from KaTeX/src/fontMetrics.js
// Do not edit manually

#[derive(Debug, Clone)]
/// Font metrics for a specific style size (text, script, scriptscript)
pub struct FontMetrics {{
{}
}}

/// Constant font metrics for textstyle, scriptstyle, and scriptscriptstyle
pub const FONT_METRICS: [FontMetrics; 3] = [
{}
];
",
        fields.join("\n"),
        const_items.join(",\n")
    );

    // Write to file
    let dest_path = Path::new(out_dir).join("sigmas_and_xis_generated.rs");
    let mut file = File::create(&dest_path).unwrap();
    file.write_all(rust_code.as_bytes()).unwrap();
}

fn generate_font_metrics(out_dir: &str) {
    use phf_codegen::Map as PhfMap;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    let dest_path = Path::new(out_dir).join("font_metrics_data_phf.rs");

    let json_data = fs::read_to_string("data/font_metrics_data.json")
        .expect("Failed to read font_metrics_data.json");

    let font_metrics: HashMap<String, HashMap<String, Vec<f64>>> =
        serde_json::from_str(&json_data).expect("Failed to parse JSON data");

    let mut file = File::create(&dest_path).unwrap();

    let mut font_index = PhfMap::new();

    for (font_family, metrics) in font_metrics {
        let font_name = font_family.replace(['-', '.'], "_");
        font_index.entry(
            font_family.clone(),
            format!("&{}_METRICS", font_name.to_uppercase()),
        );

        let mut map = PhfMap::new();

        for (char_code_str, metrics_array) in metrics {
            let char_code: u32 = char_code_str.parse().unwrap();
            let field_str = metrics_array
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

            let value = format!("CharacterMetrics::new({field_str})");

            map.entry(char_code, value);
        }

        writeln!(
            &mut file,
            "/// Font metrics for the {font_family} font family"
        )
        .unwrap();

        writeln!(
            &mut file,
            "#[allow(clippy::expect_used)]\n#[allow(clippy::approx_constant)]\npub const {}_METRICS: phf::Map<u32, CharacterMetrics> = {};",
            font_name.to_uppercase(),
            map.build()
        )
        .unwrap();

        writeln!(&mut file).unwrap();
    }

    writeln!(
        &mut file,
        "/// Mapping of font family names to their corresponding metrics maps"
    )
    .unwrap();
    writeln!(
        &mut file,
        "#[allow(clippy::expect_used)]\n#[allow(clippy::non_ascii_literal)]\npub const FONT_METRICS_INDEX: phf::Map<&'static str, &'static phf::Map<u32, CharacterMetrics>> = \n{};\n",
        font_index.build()
    )
    .unwrap();
}

/// Generate a mapping of normalized Unicode symbols to their component parts
///
/// This function creates combinations of base letters with Unicode combining
/// accents, normalizes them using NFC normalization, and returns a mapping
/// from the normalized single character to the original component string.
fn generate_unicode_symbols(out_dir: &str) {
    use phf_codegen::Map as PhfMap;

    let dest_path = Path::new(out_dir).join("unicode_symbols_phf.rs");
    let mut file = File::create(&dest_path).expect("Failed to create unicode_symbols_phf.rs");

    let accents: Vec<char> = UNICODE_ACCENTS.keys().copied().collect();

    let letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZαβγδεϵζηθϑικλμνξοπϖρϱςστυφϕχψωΓΔΘΛΞΠΣΥΦΨΩ";
    let mut seen = HashSet::new();
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

                // Use (accent2, accent) order to match KaTeX's behavior
                let combined2 = format!("{letter}{accent2}{accent}");
                if let Some(normalized) = normalize_to_single_char(&combined2)
                    && seen.insert(normalized)
                {
                    map.entry(normalized, format!("\"{}\"", escape_as_unicode(&combined2)));
                }
            }
        }
    }

    writeln!(
        &mut file,
        "/// Mapping of normalized Unicode symbols to their component parts"
    )
    .unwrap();

    writeln!(
        &mut file,
        "/// Unicode symbols map for Modifier tone letters\n\
         #[allow(clippy::expect_used)]\n#[allow(clippy::non_ascii_literal)]\npub const UNICODE_SYMBOLS: phf::Map<char, &str> = \n{};\n",
        map.build()
    )
    .unwrap();
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

fn parse_unicode_escape<I>(iter: &mut std::iter::Peekable<I>) -> Option<String>
where
    I: Iterator<Item = char> + Clone,
{
    let mut lookahead = iter.clone();
    if lookahead.peek() != Some(&'\\') {
        return None;
    }
    lookahead.next(); // consume '\'
    if lookahead.peek() != Some(&'u') {
        return None;
    }
    lookahead.next(); // consume 'u'

    if lookahead.peek() == Some(&'{') {
        lookahead.next(); // consume '{'
        let mut hex_digits = String::new();
        while let Some(&c) = lookahead.peek() {
            if c == '}' {
                break;
            }
            if c.is_ascii_hexdigit() {
                hex_digits.push(c);
                lookahead.next();
            } else {
                return None;
            }
        }
        if lookahead.peek() == Some(&'}') && !hex_digits.is_empty() {
            lookahead.next(); // consume '}'
            *iter = lookahead;
            return Some(format!("\\u{{{}}}", hex_digits));
        } else {
            return None;
        }
    }

    let mut hex_digits = String::new();
    for _ in 0..4 {
        if let Some(&c) = lookahead.peek() {
            if c.is_ascii_hexdigit() {
                hex_digits.push(c);
                lookahead.next();
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    *iter = lookahead;
    Some(format!("\\u{{{}}}", hex_digits))
}

fn convert_unicode_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        if let Some(escaped) = parse_unicode_escape(&mut chars) {
            result.push_str(&escaped);
        } else {
            result.push(chars.next().unwrap());
        }
    }
    result
}
fn convert_name_unicode_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        if let Some(escaped) = parse_unicode_escape(&mut chars) {
            result.push_str(&escaped);
        } else {
            result.push(chars.next().unwrap());
        }
    }
    result
}

/// Normalize a string using NFC normalization and return the single character
/// if applicable
fn normalize_to_single_char(s: &str) -> Option<char> {
    use unicode_normalization::UnicodeNormalization;

    let normalized: String = s.chars().nfc().collect();

    if normalized.chars().count() == 1 {
        normalized.chars().next()
    } else {
        None
    }
}

fn write_symbols(file: &mut File, symbols: &[Symbol], variant: &str) {
    let mut char_dict = HashMap::new();
    for symbol in symbols {
        let char_info = (
            symbol.font.clone(),
            symbol.group.clone(),
            symbol.replace.clone(),
        );
        if !char_dict.contains_key(&char_info) {
            char_dict.insert(char_info.clone(), HashSet::new());
        }
        if let Some(val) = char_dict.get_mut(&char_info) {
            val.insert(symbol.name.clone());
        }
        if let Some(replace) = &symbol.replace
            && symbol.accept_unicode_char
            && let Some(val) = char_dict.get_mut(&char_info)
        {
            val.insert(replace.clone());
        }
    }

    writeln!(
        file,
        "const {variant}_MAP: phf::Map<&str, CharInfo> = phf_map!("
    )
    .unwrap();

    for (char_info, names) in char_dict {
        let (font, group, replace) = char_info;
        let arm = names
            .into_iter()
            .map(|s| convert_name_unicode_escapes(&s))
            .collect::<Vec<_>>()
            .join("\" | \"");

        let font_str = match font.as_str() {
            "main" => "Font::Main",
            "ams" => "Font::Ams",
            _ => panic!("Invalid font: {font}"),
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
            _ => panic!("Invalid group: {group}"),
        };

        let replace_value = replace.as_ref().map_or_else(
            || "None".to_string(),
            |s| {
                let converted = convert_unicode_escapes(s);
                format!("Some(\'{converted}\')")
            },
        );

        writeln!(file, "    \"{arm}\" => CharInfo {{").unwrap();
        writeln!(file, "        font: {font_str},").unwrap();
        writeln!(file, "        group: {group_str},").unwrap();
        writeln!(file, "        replace: {replace_value},").unwrap();
        writeln!(file, "    }},").unwrap();
    }

    writeln!(file, ");\n").unwrap();
}

fn generate_symbols(out_dir: &str) {
    let json_data = fs::read_to_string("data/symbols.json").expect("Failed to read symbols.json");

    let symbol_data: Vec<Symbol> =
        serde_json::from_str(&json_data).expect("Failed to parse symbols JSON");

    let dest_path = Path::new(out_dir).join("generated_symbols_data.rs");
    let mut file = File::create(&dest_path).expect("Failed to create generated_symbols_data.rs");

    writeln!(&mut file, "// Auto-generated file - do not edit manually").unwrap();
    writeln!(&mut file, "// Generated from data/symbols.json").unwrap();
    writeln!(&mut file).unwrap();

    let (math_symbols, text_symbols): (Vec<Symbol>, Vec<Symbol>) =
        symbol_data.into_iter().partition(|s| s.mode == "math");

    writeln!(&mut file, "/// Populate math symbols from JSON data").unwrap();
    write_symbols(&mut file, &math_symbols, "POPULATE_MATH_SYMBOLS");
    writeln!(&mut file, "/// Populate text symbols from JSON data").unwrap();
    write_symbols(&mut file, &text_symbols, "POPULATE_TEXT_SYMBOLS");
}
