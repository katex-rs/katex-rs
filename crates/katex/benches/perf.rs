use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use katex::macros::MacroDefinition;
use katex::{KatexContext, Settings, render_to_string};
use serde::Deserialize;

const TESTS_TO_RUN: [&str; 7] = [
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

#[derive(Debug)]
struct TestCase {
    tex: String,
    display_mode: bool,
    macros: HashMap<String, String>,
}

impl RawTestCase {
    fn into_test_case(self) -> Result<TestCase, Box<dyn Error>> {
        match self {
            RawTestCase::Simple(tex) => Ok(TestCase {
                tex,
                display_mode: false,
                macros: HashMap::new(),
            }),
            RawTestCase::Detailed(case) => Ok(TestCase {
                tex: case.tex,
                display_mode: case.display.is_some_and(|value| value.into()),
                macros: case.macros,
            }),
        }
    }
}

struct PreparedCase {
    name: &'static str,
    tex: Arc<str>,
    settings: Arc<Settings>,
}

fn load_cases() -> Result<Vec<PreparedCase>, Box<dyn Error>> {
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
        .map(|&name| -> Result<PreparedCase, Box<dyn Error>> {
            let case = raw_cases
                .remove(name)
                .ok_or_else(|| io_error(format!("missing test case '{name}' in ss_data.yaml")))?
                .into_test_case()?;

            Ok(PreparedCase {
                name,
                tex: Arc::<str>::from(case.tex),
                settings: Arc::new(build_settings(case.display_mode, &case.macros)),
            })
        })
        .collect()
}

fn build_settings(display_mode: bool, macros: &HashMap<String, String>) -> Settings {
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

fn io_error(message: String) -> std::io::Error {
    std::io::Error::other(message)
}

fn dataset_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../KaTeX/test/screenshotter/ss_data.yaml")
        .canonicalize()
        .unwrap_or_else(|_| manifest_dir.join("../../KaTeX/test/screenshotter/ss_data.yaml"))
}

fn bench_rendering(c: &mut Criterion) {
    let ctx = Arc::new(KatexContext::default());
    let cases = load_cases().expect("failed to load KaTeX screenshotter cases");

    let mut group = c.benchmark_group("katex_render");
    for PreparedCase {
        name,
        tex,
        settings,
    } in cases
    {
        let ctx = Arc::clone(&ctx);
        let tex_for_case = Arc::clone(&tex);
        let settings_for_case = Arc::clone(&settings);

        // Ensure rendering succeeds once before measuring performance.
        render_to_string(
            ctx.as_ref(),
            tex_for_case.as_ref(),
            settings_for_case.as_ref(),
        )
        .expect("rendering failed while priming benchmark caches");

        group.bench_function(name, move |b| {
            let ctx = Arc::clone(&ctx);
            let tex = Arc::clone(&tex);
            let settings = Arc::clone(&settings);

            b.iter(|| {
                let rendered = render_to_string(ctx.as_ref(), tex.as_ref(), settings.as_ref())
                    .expect("rendering failed during benchmark");
                black_box(rendered.len());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_rendering);
criterion_main!(benches);
