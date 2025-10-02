use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use katex::Settings;
use katex::macros::MacroDefinition;
use serde::Deserialize;

pub const TESTS_TO_RUN: [&str; 7] = [
    "AccentsText",
    "ArrayMode",
    "GroupMacros",
    "MathBb",
    "SqrtRoot",
    "StretchyAccent",
    "Units",
];

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawTestCase {
    Simple(String),
    Detailed(DetailedCase),
}

#[derive(Debug, Deserialize)]
struct DetailedCase {
    tex: String,
    #[serde(default)]
    macros: HashMap<String, String>,
    #[serde(default)]
    display: Option<DisplayValue>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DisplayValue {
    Bool(bool),
    Int(i64),
}

impl From<DisplayValue> for bool {
    fn from(value: DisplayValue) -> Self {
        match value {
            DisplayValue::Bool(value) => value,
            DisplayValue::Int(value) => value != 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaseDefinition {
    pub name: &'static str,
    pub tex: Arc<str>,
    pub display_mode: bool,
    pub macros: Arc<HashMap<String, String>>,
}

pub fn load_case_definitions() -> Result<Vec<CaseDefinition>, Box<dyn Error>> {
    let data_path = dataset_path();
    if !data_path.exists() {
        return Err(Box::new(io_error(format!(
            "missing dataset at {}. Run `git submodule update --init --recursive` to fetch the KaTeX fixtures.",
            data_path.display()
        ))));
    }

    let file = File::open(data_path)?;
    let reader = BufReader::new(file);
    let mut raw_cases: HashMap<String, RawTestCase> = serde_yaml::from_reader(reader)?;

    TESTS_TO_RUN
        .iter()
        .map(|&name| -> Result<CaseDefinition, Box<dyn Error>> {
            let case = raw_cases
                .remove(name)
                .ok_or_else(|| io_error(format!("missing test case '{name}' in ss_data.yaml")))?
                .into_test_case();

            Ok(CaseDefinition {
                name,
                tex: Arc::<str>::from(case.tex),
                display_mode: case.display_mode,
                macros: Arc::new(case.macros),
            })
        })
        .collect()
}

pub fn build_settings(display_mode: bool, macros: &HashMap<String, String>) -> Settings {
    let settings = Settings::builder().display_mode(display_mode).build();

    if !macros.is_empty() {
        let mut slots = settings.macros.borrow_mut();
        slots.clear();
        for (name, expansion) in macros {
            slots.insert(name.clone(), MacroDefinition::String(expansion.clone()));
        }
    }

    settings
}

fn dataset_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../KaTeX/test/screenshotter/ss_data.yaml")
        .canonicalize()
        .unwrap_or_else(|_| manifest_dir.join("../../KaTeX/test/screenshotter/ss_data.yaml"))
}

fn io_error(message: String) -> io::Error {
    io::Error::other(message)
}

impl RawTestCase {
    fn into_test_case(self) -> TestCase {
        match self {
            Self::Simple(tex) => TestCase {
                tex,
                display_mode: false,
                macros: HashMap::new(),
            },
            Self::Detailed(case) => TestCase {
                tex: case.tex,
                display_mode: case.display.map_or(false, Into::into),
                macros: case.macros,
            },
        }
    }
}

#[derive(Debug)]
struct TestCase {
    tex: String,
    display_mode: bool,
    macros: HashMap<String, String>,
}
