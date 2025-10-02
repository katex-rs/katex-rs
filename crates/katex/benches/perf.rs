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
    let cases = match load_cases() {
        Ok(cases) => cases,
        Err(err) => {
            eprintln!("failed to load KaTeX screenshotter cases: {err}");
            return;
        }
    };

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

        if let Err(err) = render_to_string(
            ctx.as_ref(),
            tex_for_case.as_ref(),
            settings_for_case.as_ref(),
        ) {
            eprintln!(
                "skipping benchmark for {name}: failed to render test case while priming caches: {err}"
            );
            continue;
        }

        group.bench_function(name, move |b| {
            let ctx = Arc::clone(&ctx);
            let tex = Arc::clone(&tex);
            let settings = Rc::clone(&settings);

            b.iter(|| {
                if let Ok(rendered) =
                    render_to_string(ctx.as_ref(), tex.as_ref(), settings.as_ref())
                {
                    black_box(rendered.len());
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_rendering);
criterion_main!(benches);
