# KaTeX Rendering Performance Benchmarks

## Running the benchmarks directly

The renderer benchmarks rely on the KaTeX screenshotter fixtures – fetch them
with `git submodule update --init --recursive` before running any of the
commands below.

### JavaScript (reference)

```bash
cd KaTeX
npm install
npm run test:perf
```

The upstream script uses [`benchmark.js`](https://benchmarkjs.com) and reports
operations per second for KaTeX’s JavaScript renderer.

### Rust (native, Criterion)

```bash
cargo bench --bench perf
```

The Criterion harness primes each case before measurement so the caches mirror
production behaviour. Use Criterion’s reporting to track throughput changes over time and run targeted comparisons.

### Rust (native, Gungraun)

```bash
cargo bench --bench perf_gungraun
```

Gungraun replays the same cases, emits callgrind traces plus SVG flamegraphs,
and checks for regressions with a +5% soft limit on instruction counts before
surfacing warnings.

### Rust (WebAssembly)

The WebAssembly benchmark is a work-in-progress. Since the native implementation
matches the JavaScript renderer closely, optimising the WASM path has lower
priority right now.

## Flamegraph tooling

Gungraun automatically generates callgrind traces and SVG flamegraphs alongside
the benchmark results in `target/gungraun/`. Use a viewer such as
`kcachegrind`, `callgrind_annotate`, or a browser to inspect the output. The
`xtask` flamegraph helper has been retired; profile the WebAssembly and
JavaScript harnesses with their native tooling when necessary.
