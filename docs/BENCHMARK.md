# KaTeX Rendering Performance Benchmarks

## Running the benchmarks directly

The Criterion suite replays the same expressions used by the screenshotter
tests. It expects the KaTeX fixtures from the `KaTeX/test/screenshotter`
directory – fetch them with `git submodule update --init --recursive` before
running any of the commands below.【F:crates/katex/benches/perf.rs†L87-L118】 A
missing dataset triggers a helpful error:

```
missing dataset at …/KaTeX/test/screenshotter/ss_data.yaml. Run `git submodule
update --init --recursive` to fetch the KaTeX fixtures.
```

### JavaScript (reference)

```bash
cd KaTeX
npm install
npm run test:perf
```

The upstream script uses [`benchmark.js`](https://benchmarkjs.com) and reports
operations per second for KaTeX’s JavaScript renderer.

### Rust (native)

```bash
cargo bench --bench perf
```

The harness primes each case once before timing to ensure fonts and layout data
are cached the same way as the production renderer.

### Rust (WebAssembly)

The WebAssembly benchmark is a work-in-progress. Since the native implementation
matches the JavaScript renderer closely, optimising the WASM path has lower
priority right now.

## Flamegraph tooling

The repository provides an `xtask` helper that wraps the setup steps above and
records CPU flamegraphs via Linux `perf` and
[`inferno`](https://github.com/jonhoo/inferno). Examples:

```bash
# Profile the Criterion benchmark harness
cargo xtask flamegraph native

# Profile the upstream JavaScript renderer
cargo xtask flamegraph js
```

All flamegraph SVGs are written to `target/flamegraphs/`. Use `--open` to launch
the generated file, or `--output`/`--perf-data` to customise the output paths.