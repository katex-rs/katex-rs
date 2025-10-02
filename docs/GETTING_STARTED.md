# Getting started with KaTeX-rs

This guide collects the reproducible steps required to hydrate the repository,
install tooling, and run the project’s verification suites. The commands can be
executed on Linux, macOS, or Windows (via WSL) as long as the prerequisites are
available.

## 1. Hydrate the repository

KaTeX-rs vendors fonts, fixtures, and benchmark datasets from the upstream
KaTeX project. Clone the repository and fetch its assets before running any
build or test commands:

```bash
# Clone (or update) the repository
git clone https://github.com/katex-rs/katex-rs.git
cd katex-rs

# Fetch submodules and large files used by tests/benchmarks
git submodule update --init --recursive
git lfs install --skip-repo
git lfs pull
```

The benchmarking harness and screenshot suite expect the upstream KaTeX
fixtures under `KaTeX/test/screenshotter`. Missing fixtures will cause the
benchmark to fail with a helpful error pointing back to the commands above.

## 2. Install toolchains

| Purpose | Command |
| --- | --- |
| Stable Rust toolchain | `rustup default stable` |
| Nightly toolchain for linting/screenshot tests | `rustup toolchain install nightly` |
| Node.js dependencies | Install Node.js 18+ and npm via your package manager |
| wasm-pack (WebAssembly builds) | `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf \| sh` |
| cargo-nextest (faster test runner) | `cargo install --locked cargo-nextest` |

> Tip: the repository ships an [xtask helper](../xtask/src/main.rs) that wraps
> workflows such as screenshot tests and data extraction.

## 3. Run the verification suite

Once the repository is hydrated and the toolchains are available, use the
following commands to verify a checkout:

```bash
# Format the workspace
cargo fmt --all

# Clippy linting (requires the nightly toolchain)
cargo +nightly clippy --all-targets --all-features

# Run unit and integration tests
cargo nextest run --no-fail-fast
```

The integration tests mirror KaTeX’s JavaScript specification suite and rely on
shared fixtures from the `KaTeX` submodule.

## 4. Generate artifacts

### WebAssembly package

Build the npm-compatible WebAssembly bundle that mirrors KaTeX’s JavaScript API:

```bash
wasm-pack build crates/wasm-binding --release --target bundler
```

The generated package exports `render` and `renderToString` functions that match
KaTeX’s camelCase entry points, making it possible to swap KaTeX-rs into existing
JavaScript tooling without adapters.

### Static Data extraction

KaTeX-rs vendors font metrics and other static data from the upstream KaTeX
project. If the submodule is updated, regenerate the JSON assets in the `crates/katex/data`
directory by running:
```bash
cargo xtask extract-data
```

### Screenshot regression tests

The project provides an automated harness that renders hundreds of expressions
in browsers and compares the results to upstream KaTeX output:

```bash
cargo xtask screenshotter
```

Install Google Chrome, Firefox, and their WebDriver companions for full
coverage. Pass `--browser` and `--webdriver` options to target specific setups.

### Native benchmarks and flamegraphs

KaTeX-rs bundles two benchmark harnesses that replay the same inputs as the
screenshot tests. Hydrate the KaTeX submodule before running either command:

```bash
cargo bench --bench perf

cargo bench --bench perf_gungraun
```

`perf` uses Criterion for statistically robust throughput comparisons. The
`perf_gungraun` harness compares results against the prior baseline, surfaces
regressions, and emits callgrind traces plus SVG flamegraphs to
`target/gungraun/`. For additional options, refer to
[`docs/BENCHMARK.md`](BENCHMARK.md) and [`docs/FLAMEGRAPH.md`](FLAMEGRAPH.md).
