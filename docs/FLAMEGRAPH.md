# Flamegraph tooling and baseline measurements

This document summarises the profiling workflow for the KaTeX renderer. The
native benchmarks rely on the Gungraun harness, which generates callgrind
profiles and SVG flamegraphs directly when you run `cargo bench --bench
perf_gungraun`. The former `cargo xtask flamegraph` helper has been retired, so
profile the WebAssembly and JavaScript environments with their native tooling as
needed. For a reproducible setup that covers repository hydration, toolchain
installation, and verification commands, see
[`docs/GETTING_STARTED.md`](GETTING_STARTED.md).

## Prerequisites

Before collecting data, make sure the repository is fully hydrated:

- Fetch the KaTeX submodule so that the shared test fixtures are available:
  ```bash
  git submodule update --init --recursive
  ```
- Install the external tools used by the helpers:
  - Linux `perf`:
    ```bash
    sudo apt install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
    ```
  - `wasm-pack` (for WebAssembly builds)
  - `npm` (for the Node.js scripts)

The native harness expects debugging information for accurate stack traces. The
workspace exposes a `profiling` Cargo profile that inherits from `release`
while forcing `debug = true` and disabling LTO.

## Native renderer (Gungraun)

Run the following command to record fresh callgrind traces, flamegraphs, and a
regression summary for the Rust renderer:

```bash
cargo bench --bench perf_gungraun
```

Results are written to `target/gungraun/`. Each run compares against the most
recent baseline, reports any regressions beyond the configured limits, and
emits flamegraphs without relying on `perf`.

## WebAssembly and JavaScript renderers

When profiling the WebAssembly or JavaScript implementations, rely on the tools
provided by those environments (for example, Node.js’ inspector or browser
profilers). The retired `cargo xtask flamegraph` wrapper previously automated
these workflows; the manual steps vary based on the environment and desired
profiling backend.

## Outstanding environment gaps

- WebAssembly profiling requires `wasm-pack`; the tool is not present in the
  current container, so building the wasm benchmark will fail until it is
  installed (`cargo install wasm-pack` or the official installer script).
- Installing KaTeX’s JavaScript dependencies with `npm install` currently hits a
  peer-dependency conflict between `stylelint` and `stylelint-scss`. Running
  `npm install --legacy-peer-deps` (or updating the dependency constraints)
  resolves the issue locally before profiling the JavaScript renderer.

Once these prerequisites are met, follow the environment-specific profiler
instructions to generate comparable flamegraphs for the wasm and JavaScript
harnesses.
