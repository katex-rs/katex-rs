# KaTeX Rendering Performance Benchmarks

## Running the benchmarks directly

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

### Rust (WebAssembly)

This part is work-in-progress. Since native part shows similar performance to
JavaScript, there is less urgency to optimise the WebAssembly path.

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