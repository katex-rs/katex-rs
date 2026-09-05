extern crate alloc;

use alloc::rc::Rc;
use alloc::sync::Arc;
use std::error::Error;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use katex::{KatexContext, Settings, render_to_string};

#[path = "support.rs"]
mod support;

use support::{CaseDefinition, build_settings, load_case_definitions};

struct PreparedCase {
    name: &'static str,
    tex: Arc<str>,
    settings: Rc<Settings>,
}

fn load_cases() -> Result<Vec<PreparedCase>, Box<dyn Error>> {
    let cases = load_case_definitions()?;

    cases
        .into_iter()
        .map(
            |CaseDefinition {
                 name,
                 tex,
                 display_mode,
                 macros,
             }| {
                Ok(PreparedCase {
                    name,
                    tex,
                    settings: Rc::new(build_settings(display_mode, &macros)),
                })
            },
        )
        .collect()
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
        let settings_for_case = Rc::clone(&settings);

        render_to_string(
            ctx.as_ref(),
            tex_for_case.as_ref(),
            settings_for_case.as_ref(),
        )
        .unwrap_or_else(|err| panic!("failed to prime benchmark {name}: {err}"));

        group.bench_function(name, move |b| {
            let ctx = Arc::clone(&ctx);
            let tex = Arc::clone(&tex);
            let settings = Rc::clone(&settings);

            b.iter(|| {
                black_box(
                    render_to_string(ctx.as_ref(), black_box(tex.as_ref()), settings.as_ref())
                        .unwrap_or_else(|err| panic!("benchmark {name} failed: {err}")),
                );
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_rendering);
criterion_main!(benches);
