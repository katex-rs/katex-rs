# KaTeX-rs

[![Crates.io](https://img.shields.io/crates/v/katex-rs.svg)](https://crates.io/crates/katex-rs)
[![Documentation](https://docs.rs/katex-rs/badge.svg)](https://docs.rs/katex-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![NPM version](https://img.shields.io/npm/v/katex-rs.svg)](https://www.npmjs.com/package/katex-rs)

> Fast, fully configurable KaTeX rendering from Rust with drop-in WebAssembly bindings.

KaTeX-rs is a Rust re-implementation of the
[KaTeX](https://github.com/KaTeX/KaTeX) rendering engine. It converts LaTeX math
into HTML and MathML and is designed for server-side rendering, command-line
tools, and WebAssembly targets. The project currently tracks KaTeX commit
[9fb63136e680715ad83c119366f6f697105d2c55](https://github.com/KaTeX/KaTeX/commit/9fb63136e680715ad83c119366f6f697105d2c55).

## Highlights

- **Native rendering pipeline.** The `render_to_string` function turns LaTeX into
  KaTeX-compatible HTML + MathML markup that can be embedded directly into web
  pages or server-rendered responses.
- **Fine-grained configuration.** Toggle display/inline layout, strictness and
  trust modes, color and size options, equation numbering, custom macros, and
  more through the `Settings` builder.
- **WebAssembly bindings.** The `katex-wasm-binding` crate exports the canonical
  `render` and `renderToString` entry points so the generated `pkg/katex.js`
  bundle can replace KaTeX.js in existing JavaScript tooling without glue code.
- **Spec-driven test suite.** Rust tests mirror the upstream KaTeX spec cases to
  ensure parsing and rendering stay in lockstep with the JavaScript reference
  implementation.

## Project status

- [x] Core parsing, HTML, and MathML rendering
- [x] Spec-aligned unit and integration tests
- [x] Automated screenshot regression harness
- [x] WebAssembly bindings with KaTeX-compatible API surface
- [ ] Perfect visual parity with the latest KaTeX release (`FireFox` now works well; `Chrome` has minor layout differences)

## Quick start

### Add the crate

```toml
[dependencies]
katex-rs = "0.1"
```

### Render LaTeX to HTML + MathML

```rust
use katex::{render_to_string, KatexContext, Settings};

fn main() -> Result<(), katex::ParseError> {
    // The context caches fonts, macros, and environments – reuse it between renders.
    let ctx = KatexContext::default();

    // Start with the default configuration and tweak as needed.
    let settings = Settings::default();

    let html = render_to_string(&ctx, r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", &settings)?;
    println!("{html}");
    Ok(())
}
```

Configure display mode, numbering, colors and trust checks through the
builder API:

```rust
use katex::{render_to_string, KatexContext, Settings, StrictMode, StrictSetting, TrustSetting};

fn main() -> Result<(), katex::ParseError> {
    let ctx = KatexContext::default();

    let settings = Settings::builder()
        .display_mode(true)
        .fleqn(true)
        .leqno(true)
        .strict(StrictSetting::Mode(StrictMode::Warn))
        .trust(TrustSetting::Bool(true))
        .color_is_text_color(true)
        .build();

    let html = render_to_string(&ctx, r"\\RR_{>0}", &settings)?;
    println!("{html}");
    Ok(())
}
```

### Use the WebAssembly build

Install the npm package and invoke the familiar KaTeX API surface:

```bash
npm install katex-rs
```

```ts
import katex from "katex-rs";

const html = katex.renderToString("\\int_0^\\infty e^{-x^2} dx", {
  displayMode: true,
  trust: true,
});
```

The WASM bundle exposes the same `render`/`renderToString` signatures as
KaTeX.js, accepts plain JavaScript option objects, and throws matching error
types for easy drop-in replacement.【F:crates/wasm-binding/src/lib.rs†L5-L116】【F:crates/wasm-binding/src/lib.rs†L299-L358】

## Development & reproducibility

A reproducible workflow – including repository hydration, tooling installation,
and verification commands – is documented in
[`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md). The quick-reference
checklist is:

1. Hydrate the KaTeX submodule and Git LFS assets (`git submodule update --init --recursive`).
2. Install Rust (stable + nightly), Node.js, `wasm-pack`, and `cargo-nextest`.
3. Run formatting, Clippy, and the Nextest-powered test suite.
4. Use `cargo xtask screenshotter` for browser-based regression tests, run
   `cargo bench --bench perf` for the Criterion-based native benchmarks, and
   `cargo bench --bench perf_gungraun` for Gungraun flamegraphs plus regression
   checks (Need installing `gungraun-runner` for this). You can checkout generated 
   flamegraphs in the `target/gungraun/katex-rs/perf_gungraun` folder.

Refer to [`docs/BENCHMARK.md`](docs/BENCHMARK.md) and
[`docs/FLAMEGRAPH.md`](docs/FLAMEGRAPH.md) for deeper performance workflows.

## Repository layout

The repository is organised as a Cargo workspace:

- [`crates/katex`](crates/katex) – core renderer crate exported on crates.io.
- [`crates/wasm-binding`](crates/wasm-binding) – WebAssembly bindings that mirror
  KaTeX’s JavaScript API.
- [`xtask`](xtask) – developer tooling for screenshot tests, flamegraphs, and
  other automation.

## License

KaTeX-rs is available under the MIT License. See [LICENSE](LICENSE) for details.
