use std::collections::HashMap;
use std::fmt;
use std::hint::black_box;
use std::sync::{Arc, OnceLock};

use gungraun::{
    Callgrind, EventKind, FlamegraphConfig, LibraryBenchmarkConfig, library_benchmark,
    library_benchmark_group, main,
};
use katex::{KatexContext, Settings, render_to_string};

#[path = "support.rs"]
mod support;

use support::{CaseDefinition, TESTS_TO_RUN, build_settings, load_case_definitions};

#[derive(Clone)]
struct BenchmarkCase {
    name: &'static str,
    tex: Arc<str>,
    settings: Settings,
    ctx: Arc<KatexContext>,
}

impl fmt::Debug for BenchmarkCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

static CASE_DEFINITIONS: OnceLock<HashMap<&'static str, CaseDefinition>> = OnceLock::new();
static KATEX_CONTEXT: OnceLock<Arc<KatexContext>> = OnceLock::new();

#[library_benchmark]
#[benches::render(iter = TESTS_TO_RUN.into_iter(), setup = prepare_case)]
fn bench_rendering(case: BenchmarkCase) -> usize {
    let rendered = render_to_string(case.ctx.as_ref(), case.tex.as_ref(), &case.settings)
        .unwrap_or_else(|err| panic!("failed to render {name}: {err}", name = case.name));

    black_box(rendered.len())
}

library_benchmark_group!(
    name = katex_render;
    benchmarks = bench_rendering
);

main!(
    config = LibraryBenchmarkConfig::default()
        .tool(
            Callgrind::default()
                .flamegraph(FlamegraphConfig::default())
                .soft_limits([(EventKind::Ir, 5.0)])
        );
    library_benchmark_groups = katex_render
);

fn prepare_case(name: &'static str) -> BenchmarkCase {
    let definitions = case_definitions();
    let definition = definitions
        .get(name)
        .unwrap_or_else(|| panic!("missing benchmark case '{name}'"));

    let settings = build_settings(definition.display_mode, &definition.macros);
    let ctx = katex_context();

    render_to_string(ctx.as_ref(), definition.tex.as_ref(), &settings)
        .unwrap_or_else(|err| panic!("failed to prepare {name}: {err}"));

    BenchmarkCase {
        name,
        tex: Arc::clone(&definition.tex),
        settings,
        ctx,
    }
}

fn case_definitions() -> &'static HashMap<&'static str, CaseDefinition> {
    CASE_DEFINITIONS.get_or_init(|| {
        load_case_definitions()
            .unwrap_or_else(|err| panic!("failed to load KaTeX screenshotter fixtures: {err}"))
            .into_iter()
            .map(|definition| (definition.name, definition))
            .collect()
    })
}

fn katex_context() -> Arc<KatexContext> {
    Arc::clone(KATEX_CONTEXT.get_or_init(|| Arc::new(KatexContext::default())))
}
