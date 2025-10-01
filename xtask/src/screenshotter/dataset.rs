use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{Result, bail, eyre};
use serde_json::{Map as JsonMap, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::screenshotter::args::ScreenshotterArgs;
use crate::screenshotter::models::TestCase;

pub fn workspace_root() -> Result<Utf8PathBuf> {
    let manifest_dir = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    manifest_dir
        .parent()
        .map(|path| path.to_owned())
        .ok_or_else(|| eyre!("failed to determine workspace root"))
}

pub fn load_cases(root: &Utf8Path, args: &ScreenshotterArgs) -> Result<Vec<TestCase>> {
    if let Some(tex) = &args.tex {
        let key = args.case.clone().unwrap_or_else(|| "AdHoc".to_string());
        let mut payload = JsonMap::new();
        payload.insert("tex".to_owned(), JsonValue::String(tex.clone()));
        return Ok(vec![TestCase {
            key,
            payload: JsonValue::Object(payload),
        }]);
    }

    let yaml_path = root.join("KaTeX/test/screenshotter/ss_data.yaml");
    if !yaml_path.exists() {
        bail!(
            "screenshotter dataset not found at {}. Did you fetch the KaTeX submodule?",
            yaml_path
        );
    }

    let text = fs::read_to_string(yaml_path.as_std_path())?;
    let value: YamlValue = serde_yaml::from_str(&text)?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| eyre!("screenshotter dataset is not a mapping"))?;

    let mut cases = Vec::new();
    for (key, item) in mapping {
        let name = key
            .as_str()
            .ok_or_else(|| eyre!("case name is not a string"))?;
        cases.push(build_case_from_yaml_item(name, item)?);
    }

    Ok(cases)
}

pub fn filter_cases(mut cases: Vec<TestCase>, args: &ScreenshotterArgs) -> Vec<TestCase> {
    if let Some(case) = &args.case {
        cases.retain(|c| &c.key == case);
    }

    if let Some(include) = &args.include {
        let patterns: Vec<String> = include
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !patterns.is_empty() {
            cases.retain(|c| patterns.iter().any(|p| c.key.contains(p)));
        }
    }

    if let Some(exclude) = &args.exclude {
        let patterns: Vec<String> = exclude
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !patterns.is_empty() {
            cases.retain(|c| !patterns.iter().any(|p| c.key.contains(p)));
        }
    }

    cases
}

fn build_case_from_yaml_item(name: &str, value: &YamlValue) -> Result<TestCase> {
    let payload = match value {
        YamlValue::String(s) => {
            let mut map = JsonMap::new();
            map.insert("tex".to_owned(), JsonValue::String(s.clone()));
            Ok(JsonValue::Object(map))
        }
        YamlValue::Mapping(_) => normalize_mapping_payload(value),
        YamlValue::Sequence(_)
        | YamlValue::Tagged(_)
        | YamlValue::Null
        | YamlValue::Bool(_)
        | YamlValue::Number(_) => {
            let mut map = JsonMap::new();
            map.insert("tex".to_owned(), JsonValue::String(String::new()));
            Ok(JsonValue::Object(map))
        }
    }?;

    Ok(TestCase {
        key: name.to_string(),
        payload,
    })
}

fn normalize_mapping_payload(value: &YamlValue) -> Result<JsonValue> {
    let mut object = match yaml_to_json(value) {
        JsonValue::Object(map) => map,
        _ => JsonMap::new(),
    };

    if !object.contains_key("tex") {
        object.insert("tex".to_owned(), JsonValue::String(String::new()));
    }

    // Ensure macros are objects with string values.
    if let Some(macros) = object.get_mut("macros") {
        if let JsonValue::Object(map) = macros {
            map.retain(|_, v| v.is_string());
        } else {
            object.remove("macros");
        }
    }

    Ok(JsonValue::Object(object))
}

fn yaml_to_json(value: &YamlValue) -> JsonValue {
    match value {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(*b),
        YamlValue::Number(num) => {
            if let Some(i) = num.as_i64() {
                JsonValue::Number(i.into())
            } else if let Some(u) = num.as_u64() {
                JsonValue::Number(serde_json::Number::from(u))
            } else if let Some(f) = num.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            } else {
                JsonValue::Null
            }
        }
        YamlValue::String(s) => JsonValue::String(s.clone()),
        YamlValue::Sequence(seq) => JsonValue::Array(seq.iter().map(yaml_to_json).collect()),
        YamlValue::Mapping(map) => {
            let mut obj = JsonMap::new();
            for (key, value) in map {
                if let Some(key) = key.as_str() {
                    obj.insert(key.to_owned(), yaml_to_json(value));
                }
            }
            JsonValue::Object(obj)
        }
        YamlValue::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}
